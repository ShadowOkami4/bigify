//! The service event loop: owns the librespot session, the Connect device
//! (Spirc), the local player, LAN discovery and the sign-in flows.
//!
//! The structure mirrors librespot's own binary (`src/main.rs` in
//! librespot 0.8) so behaviour stays close to the reference implementation:
//! a single `select!` loop handles commands, discovery credentials, the
//! browser OAuth result, (re)connection and Spirc termination.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bigify_core::color::Scheme;
use bigify_core::error::{ErrorKind, ServiceError};
use bigify_core::model::MediaKind;
use bigify_core::playback::{PlayerUpdate, RepeatMode};
use bigify_core::settings::{AudioQuality, Settings};
use bigify_core::{AuthState, Command, Event, LibraryKind};
use futures_util::StreamExt;
use librespot::connect::{ConnectConfig, LoadRequest, LoadRequestOptions, PlayingTrack, Spirc};
use librespot::core::authentication::Credentials;
use librespot::core::cache::Cache;
use librespot::core::config::{DeviceType, SessionConfig};
use librespot::core::error::ErrorKind as LsKind;
use librespot::core::session::Session;
use librespot::discovery::Discovery;
use librespot::oauth::OAuthClientBuilder;
use librespot::playback::audio_backend;
use librespot::playback::config::{AudioFormat, Bitrate, PlayerConfig};
use librespot::playback::mixer::{self, Mixer, MixerConfig};
use librespot::playback::player::{Player, PlayerEvent};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use crate::artwork::{self, PaletteCache};
use crate::webapi::WebApi;
use crate::{metadata, player_events};

/// Receives every event; must be cheap and non-blocking (the UI bridge
/// queues it onto the Qt thread).
pub type EventSink = Arc<dyn Fn(Event) + Send + Sync>;

pub struct EngineConfig {
    pub settings: Settings,
    /// Audio file cache root (`$XDG_CACHE_HOME/bigify`).
    pub cache_dir: PathBuf,
    /// Reusable credentials (`$XDG_STATE_HOME/bigify`).
    pub state_dir: PathBuf,
}

/// Port of the local OAuth redirect listener. Spotify's desktop client ID
/// accepts `http://127.0.0.1:<port>/login`.
const OAUTH_PORT: u16 = 8898;
const OAUTH_SCOPES: &[&str] = &[
    "streaming",
    "app-remote-control",
    "user-read-private",
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-currently-playing",
    "user-read-recently-played",
    "user-top-read",
    "user-library-read",
    "user-library-modify",
    "user-follow-read",
    "playlist-read-private",
    "playlist-read-collaborative",
];
const OAUTH_DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Bigify</title>\
<body style=\"font-family:sans-serif;background:#141218;color:#e6e0e9;display:grid;place-items:center;height:90vh\">\
<h1>Signed in. You can return to Bigify.</h1></body>";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BACKOFF_SECS: u64 = 30;

type SpircTask = Pin<Box<dyn Future<Output = ()>>>;

pub struct Engine {
    sink: EventSink,
    settings: Settings,
    session_config: SessionConfig,
    cache: Option<Cache>,
    credentials_file: PathBuf,
    session: Session,
    player: Arc<Player>,
    mixer: Arc<dyn Mixer>,
    spirc: Option<Spirc>,
    spirc_task: Option<SpircTask>,
    discovery: Option<Discovery>,
    credentials: Option<Credentials>,
    connected: bool,
    /// When set, a connection attempt is due at this instant.
    connect_at: Option<Instant>,
    reconnect_attempt: u32,
    oauth: Option<oneshot::Receiver<Result<Credentials, String>>>,
    /// True while the local player holds a track (we are the active device).
    locally_active: Arc<AtomicBool>,
    artist_cursors: Arc<Mutex<HashMap<u32, String>>>,
    palettes: Arc<Mutex<PaletteCache>>,
}

impl Engine {
    pub fn new(config: EngineConfig, sink: EventSink) -> Result<Self, ServiceError> {
        let settings = config.settings;
        let session_config = SessionConfig::default();
        let credentials_dir = config.state_dir.clone();
        std::fs::create_dir_all(&credentials_dir).ok();
        let audio_dir = config.cache_dir.join("audio");
        // 1 GiB audio cache keeps replays off the network without letting a
        // TV box fill its disk.
        let cache = Cache::new(
            Some(&credentials_dir),
            Some(&credentials_dir),
            Some(&audio_dir),
            Some(1 << 30),
        )
        .map_err(|e| {
            log::warn!("cache unavailable: {e}");
            e
        })
        .ok();

        let mixer_builder = mixer::find(None)
            .ok_or_else(|| ServiceError::internal("No software mixer available"))?;
        let mixer = mixer_builder(MixerConfig::default())
            .map_err(|e| ServiceError::internal(format!("Mixer failed: {e}")))?;

        let session = Session::new(session_config.clone(), cache.clone());
        let player = Self::make_player(&settings, &session, &mixer);

        Ok(Self {
            sink,
            settings,
            session_config,
            cache,
            credentials_file: credentials_dir.join("credentials.json"),
            session,
            player,
            mixer,
            spirc: None,
            spirc_task: None,
            discovery: None,
            credentials: None,
            connected: false,
            connect_at: None,
            reconnect_attempt: 0,
            oauth: None,
            locally_active: Arc::new(AtomicBool::new(false)),
            artist_cursors: Arc::default(),
            palettes: Arc::new(Mutex::new(PaletteCache::new(48))),
        })
    }

    fn make_player(settings: &Settings, session: &Session, mixer: &Arc<dyn Mixer>) -> Arc<Player> {
        let player_config = PlayerConfig {
            bitrate: match settings.audio_quality {
                AudioQuality::Low => Bitrate::Bitrate96,
                AudioQuality::Normal => Bitrate::Bitrate160,
                AudioQuality::High => Bitrate::Bitrate320,
            },
            gapless: settings.gapless,
            normalisation: settings.normalisation,
            ..PlayerConfig::default()
        };
        let backend_name = settings.audio_backend.librespot_name();
        let sink_builder =
            audio_backend::find(Some(backend_name.to_owned())).unwrap_or_else(|| {
                log::warn!("audio backend {backend_name} unavailable, using default");
                audio_backend::find(None).expect("librespot always provides a default backend")
            });
        let format = AudioFormat::default();
        Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || sink_builder(None, format),
        )
    }

    fn emit(&self, event: Event) {
        (self.sink)(event);
    }

    fn emit_auth(&self, state: AuthState) {
        log::info!("auth state: {}", state.id());
        self.emit(Event::Auth(state));
    }

    fn fail(&self, message: impl Into<String>) {
        self.emit(Event::CommandFailed {
            message: message.into(),
        });
    }

    /// Forward player events to the UI for the lifetime of the player.
    fn spawn_player_events(&self) {
        let mut channel = self.player.get_player_event_channel();
        let sink = self.sink.clone();
        let active = self.locally_active.clone();
        tokio::spawn(async move {
            while let Some(event) = channel.recv().await {
                match &event {
                    PlayerEvent::Loading { .. }
                    | PlayerEvent::Playing { .. }
                    | PlayerEvent::Paused { .. } => active.store(true, Ordering::Relaxed),
                    PlayerEvent::Stopped { .. } => active.store(false, Ordering::Relaxed),
                    _ => {}
                }
                if let Some(update) = player_events::map(&event) {
                    sink(Event::Player(update));
                }
            }
        });
    }

    pub async fn run(mut self, mut commands: mpsc::UnboundedReceiver<Command>) {
        self.emit_auth(AuthState::Starting);
        self.spawn_player_events();
        self.credentials = self.cache.as_ref().and_then(Cache::credentials);
        if self.credentials.is_some() {
            self.connect_at = Some(Instant::now());
        } else {
            self.go_signed_out();
        }

        let mut health = tokio::time::interval(Duration::from_secs(10));
        health.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            let connect_deadline = self
                .connect_at
                .unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));
            tokio::select! {
                cmd = commands.recv() => match cmd {
                    None | Some(Command::Shutdown) => break,
                    Some(cmd) => self.handle(cmd).await,
                },
                creds = async {
                    match self.discovery.as_mut() {
                        Some(d) => d.next().await,
                        None => None,
                    }
                }, if self.discovery.is_some() => match creds {
                    Some(creds) => {
                        log::info!("received credentials via Spotify Connect discovery");
                        self.use_credentials(creds);
                    }
                    None => {
                        log::warn!("discovery stopped unexpectedly");
                        self.discovery = None;
                        if !self.connected {
                            self.emit_auth(AuthState::SignedOut { discovery: false });
                        }
                    }
                },
                result = async {
                    match self.oauth.as_mut() {
                        Some(rx) => rx.await.unwrap_or_else(|_| Err("Sign-in was interrupted".into())),
                        None => std::future::pending().await,
                    }
                }, if self.oauth.is_some() => {
                    self.oauth = None;
                    match result {
                        Ok(creds) => self.use_credentials(creds),
                        Err(message) => {
                            log::warn!("browser sign-in failed: {message}");
                            self.fail("Browser sign-in didn't complete");
                            self.go_signed_out();
                        }
                    }
                },
                _ = tokio::time::sleep_until(connect_deadline), if self.connect_at.is_some() => {
                    self.connect().await;
                },
                _ = async {
                    if let Some(task) = self.spirc_task.as_mut() {
                        task.await;
                    }
                }, if self.spirc_task.is_some() => {
                    self.spirc_task = None;
                    self.spirc = None;
                    self.connected = false;
                    log::warn!("Spotify Connect session ended unexpectedly");
                    self.schedule_reconnect();
                },
                _ = health.tick() => {
                    if self.player.is_invalid() {
                        log::error!("audio player stopped; recreating it");
                        self.player = Self::make_player(&self.settings, &self.session, &self.mixer);
                        self.spawn_player_events();
                        self.fail("Audio output stopped unexpectedly and was restarted");
                        if self.connected {
                            self.teardown_connection().await;
                            self.connect_at = Some(Instant::now());
                        }
                    }
                }
            }
        }

        log::info!("shutting down Spotify service");
        self.cancel_oauth();
        self.teardown_connection().await;
        if let Some(discovery) = self.discovery.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), discovery.shutdown()).await;
        }
    }

    fn use_credentials(&mut self, creds: Credentials) {
        self.credentials = Some(creds);
        self.reconnect_attempt = 0;
        self.connect_at = Some(Instant::now());
        self.emit_auth(AuthState::Connecting);
    }

    async fn connect(&mut self) {
        self.connect_at = None;
        let Some(creds) = self.credentials.clone() else {
            self.go_signed_out();
            return;
        };
        if self.reconnect_attempt == 0 {
            self.emit_auth(AuthState::Connecting);
        }
        // A fresh session per attempt: a session that failed or was shut
        // down cannot be reused.
        self.session = Session::new(self.session_config.clone(), self.cache.clone());
        self.player.set_session(self.session.clone());

        let connect_config = ConnectConfig {
            name: self.settings.device_name.clone(),
            device_type: DeviceType::Tv,
            initial_volume: (u32::from(self.settings.initial_volume) * u32::from(u16::MAX) / 100)
                as u16,
            ..ConnectConfig::default()
        };
        let attempt = Spirc::new(
            connect_config,
            self.session.clone(),
            creds,
            self.player.clone(),
            self.mixer.clone(),
        );
        match tokio::time::timeout(CONNECT_TIMEOUT, attempt).await {
            Ok(Ok((spirc, task))) => {
                self.spirc = Some(spirc);
                self.spirc_task = Some(Box::pin(task));
                self.connected = true;
                self.reconnect_attempt = 0;
                self.protect_credentials_file();
                if let Some(discovery) = self.discovery.take() {
                    tokio::spawn(discovery.shutdown());
                }
                self.emit_auth(AuthState::Connected {
                    user: self.session.username(),
                    device_name: self.settings.device_name.clone(),
                });
            }
            Ok(Err(e)) if matches!(e.kind, LsKind::Unauthenticated | LsKind::PermissionDenied) => {
                log::warn!("stored credentials rejected: {}", e.kind);
                self.forget_credentials();
                self.fail("Your Spotify sign-in has expired. Please sign in again.");
                self.go_signed_out();
            }
            Ok(Err(e)) => {
                log::warn!("connection failed: {e}");
                self.schedule_reconnect();
            }
            Err(_) => {
                log::warn!("connection timed out");
                self.schedule_reconnect();
            }
        }
    }

    fn schedule_reconnect(&mut self) {
        if self.credentials.is_none() {
            self.go_signed_out();
            return;
        }
        self.reconnect_attempt += 1;
        let delay = 1u64
            .checked_shl(self.reconnect_attempt.saturating_sub(1).min(5))
            .unwrap_or(MAX_BACKOFF_SECS)
            .min(MAX_BACKOFF_SECS);
        log::info!(
            "reconnecting in {delay}s (attempt {})",
            self.reconnect_attempt
        );
        self.connect_at = Some(Instant::now() + Duration::from_secs(delay));
        self.emit_auth(AuthState::Reconnecting {
            attempt: self.reconnect_attempt,
        });
    }

    async fn teardown_connection(&mut self) {
        if let Some(spirc) = self.spirc.take() {
            if let Err(e) = spirc.shutdown() {
                log::debug!("spirc shutdown: {e}");
            }
        }
        if let Some(task) = self.spirc_task.take() {
            if tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .is_err()
            {
                log::warn!("Spotify Connect did not shut down in time");
            }
        }
        if !self.session.is_invalid() {
            self.session.shutdown();
        }
        self.connected = false;
    }

    fn go_signed_out(&mut self) {
        self.connected = false;
        self.connect_at = None;
        if self.discovery.is_none() && self.settings.zeroconf_discovery {
            match Discovery::builder(
                self.session_config.device_id.clone(),
                self.session_config.client_id.clone(),
            )
            .name(self.settings.device_name.clone())
            .device_type(DeviceType::Tv)
            .launch()
            {
                Ok(d) => self.discovery = Some(d),
                Err(e) => log::warn!("LAN discovery unavailable: {e}"),
            }
        }
        self.emit_auth(AuthState::SignedOut {
            discovery: self.discovery.is_some(),
        });
    }

    /// librespot writes reusable credentials (not the password, but a token
    /// equivalent). Restrict the file to the current user.
    fn protect_credentials_file(&self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if self.credentials_file.exists() {
                if let Err(e) = std::fs::set_permissions(
                    &self.credentials_file,
                    std::fs::Permissions::from_mode(0o600),
                ) {
                    log::warn!("could not restrict credentials file permissions: {e}");
                }
            }
        }
    }

    fn forget_credentials(&mut self) {
        self.credentials = None;
        match std::fs::remove_file(&self.credentials_file) {
            Ok(()) => log::info!("removed stored credentials"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("could not remove stored credentials: {e}"),
        }
    }

    fn start_browser_sign_in(&mut self) {
        if self.oauth.is_some() || self.connected {
            return;
        }
        let client_id = self.session_config.client_id.clone();
        let (tx, rx) = oneshot::channel();
        self.oauth = Some(rx);
        self.emit_auth(AuthState::BrowserSignIn);
        // The librespot OAuth listener is blocking; keep it off the runtime.
        std::thread::Builder::new()
            .name("bigify-oauth".into())
            .spawn(move || {
                let result = OAuthClientBuilder::new(
                    &client_id,
                    &format!("http://127.0.0.1:{OAUTH_PORT}/login"),
                    OAUTH_SCOPES.to_vec(),
                )
                .open_in_browser()
                .with_custom_message(OAUTH_DONE_PAGE)
                .build()
                .and_then(|client| client.get_access_token())
                .map(|token| Credentials::with_access_token(token.access_token))
                .map_err(|e| e.to_string());
                let _ = tx.send(result);
            })
            .map_err(|e| log::error!("could not start sign-in thread: {e}"))
            .ok();
    }

    /// Unblock librespot's OAuth listener by completing its single request.
    fn cancel_oauth(&mut self) {
        if self.oauth.take().is_none() {
            return;
        }
        std::thread::spawn(|| {
            use std::io::Write;
            if let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", OAUTH_PORT)) {
                let _ = s.write_all(b"GET /login?error=cancelled HTTP/1.1\r\n\r\n");
            }
        });
    }

    fn spirc_or_fail(&self) -> Option<&Spirc> {
        if self.spirc.is_none() {
            self.fail("Not connected to Spotify yet");
        }
        self.spirc.as_ref()
    }

    fn web(&self) -> Result<WebApi, ServiceError> {
        if self.connected {
            Ok(WebApi::new(self.session.clone()))
        } else {
            Err(ServiceError::not_ready())
        }
    }

    fn activate_if_needed(&self, spirc: &Spirc) -> Result<(), librespot::core::Error> {
        if !self.locally_active.load(Ordering::Relaxed) {
            spirc.activate()?;
        }
        Ok(())
    }

    async fn handle(&mut self, cmd: Command) {
        log::debug!("command: {}", command_name(&cmd));
        match cmd {
            Command::SignInWithBrowser => self.start_browser_sign_in(),
            Command::CancelSignIn => {
                self.cancel_oauth();
                self.go_signed_out();
            }
            Command::SignOut => {
                self.cancel_oauth();
                self.teardown_connection().await;
                self.forget_credentials();
                self.locally_active.store(false, Ordering::Relaxed);
                self.emit(Event::Player(PlayerUpdate::Stopped));
                self.go_signed_out();
            }
            Command::Play
            | Command::Pause
            | Command::Next
            | Command::Previous
            | Command::Seek(_)
            | Command::SetShuffle(_)
            | Command::SetRepeat(_)
            | Command::SetVolume(_)
            | Command::PlayContext { .. }
            | Command::PlayTracks { .. } => self.handle_playback(cmd),
            Command::Shutdown => {}
            data => self.handle_data(data),
        }
    }

    fn handle_playback(&self, cmd: Command) {
        let Some(spirc) = self.spirc_or_fail() else {
            return;
        };
        let result = match cmd {
            Command::Play => spirc.play(),
            Command::Pause => spirc.pause(),
            Command::Next => spirc.next(),
            Command::Previous => spirc.prev(),
            Command::Seek(ms) => spirc.set_position_ms(ms),
            Command::SetShuffle(on) => spirc.shuffle(on),
            Command::SetRepeat(mode) => match mode {
                RepeatMode::Off => spirc.repeat_track(false).and_then(|_| spirc.repeat(false)),
                RepeatMode::Context => spirc.repeat_track(false).and_then(|_| spirc.repeat(true)),
                RepeatMode::Track => spirc.repeat(true).and_then(|_| spirc.repeat_track(true)),
            },
            Command::SetVolume(v) => spirc.set_volume(v),
            Command::PlayContext {
                context_uri,
                track_uri,
            } => {
                let context_uri = if context_uri == "spotify:collection" {
                    format!("spotify:user:{}:collection", self.session.username())
                } else {
                    context_uri
                };
                self.activate_if_needed(spirc).and_then(|_| {
                    spirc.load(LoadRequest::from_context_uri(
                        context_uri,
                        LoadRequestOptions {
                            start_playing: true,
                            playing_track: track_uri.map(PlayingTrack::Uri),
                            ..LoadRequestOptions::default()
                        },
                    ))
                })
            }
            Command::PlayTracks { uris, index } => self.activate_if_needed(spirc).and_then(|_| {
                spirc.load(LoadRequest::from_tracks(
                    uris,
                    LoadRequestOptions {
                        start_playing: true,
                        playing_track: Some(PlayingTrack::Index(index)),
                        ..LoadRequestOptions::default()
                    },
                ))
            }),
            _ => Ok(()),
        };
        if let Err(e) = result {
            log::warn!("playback command failed: {e}");
            self.fail("Spotify Connect didn't accept the request");
        }
    }

    fn handle_data(&self, cmd: Command) {
        let sink = self.sink.clone();
        let web = self.web();
        match cmd {
            Command::LoadHome => {
                tokio::spawn(async move {
                    let result = match web {
                        Ok(web) => web.home().await,
                        Err(e) => Err(e),
                    };
                    sink(Event::Home(result));
                });
            }
            Command::Search { request_id, query } => {
                tokio::spawn(async move {
                    let result = match web {
                        Ok(web) => web.search(&query).await,
                        Err(e) => Err(e),
                    };
                    sink(Event::Search { request_id, result });
                });
            }
            Command::LoadLibrary { kind, offset } => {
                let cursors = self.artist_cursors.clone();
                tokio::spawn(async move {
                    let result = match web {
                        Ok(web) => match kind {
                            LibraryKind::Playlists => web.playlists(offset).await,
                            LibraryKind::Albums => web.saved_albums(offset).await,
                            LibraryKind::LikedSongs => web.saved_tracks(offset).await,
                            LibraryKind::Artists => {
                                let after = if offset == 0 {
                                    None
                                } else {
                                    cursors.lock().ok().and_then(|c| c.get(&offset).cloned())
                                };
                                if offset > 0 && after.is_none() {
                                    Err(ServiceError::internal("Lost position in artist list"))
                                } else {
                                    web.followed_artists(offset, after.as_deref()).await.map(
                                        |(page, next)| {
                                            if let (Some(next), Ok(mut c)) = (next, cursors.lock())
                                            {
                                                c.insert(offset + page.items.len() as u32, next);
                                            }
                                            page
                                        },
                                    )
                                }
                            }
                        },
                        Err(e) => Err(e),
                    };
                    sink(Event::Library {
                        kind,
                        offset,
                        result,
                    });
                });
            }
            Command::LoadCollection { uri } => {
                let session = self.session.clone();
                tokio::spawn(async move {
                    let result = match (web, MediaKind::from_uri(&uri)) {
                        (Err(e), _) => Err(e),
                        (Ok(web), Some(MediaKind::Album)) => web.album(&uri).await,
                        (Ok(web), Some(MediaKind::Playlist)) => match web.playlist(&uri).await {
                            // Contents withheld (not owned) or endpoint refused:
                            // read the playlist through librespot instead.
                            Ok(d) if d.tracks.is_empty() && d.total_tracks.unwrap_or(1) > 0 => {
                                metadata::playlist(&session, &uri).await.map(|mut m| {
                                    m.item = d.item;
                                    m
                                })
                            }
                            Ok(d) => Ok(d),
                            Err(e)
                                if matches!(
                                    e.kind,
                                    ErrorKind::Unsupported | ErrorKind::NotFound
                                ) =>
                            {
                                metadata::playlist(&session, &uri).await
                            }
                            Err(e) => Err(e),
                        },
                        _ => Err(ServiceError::unsupported(
                            "Detail pages are available for albums and playlists",
                        )),
                    };
                    sink(Event::Collection { uri, result });
                });
            }
            Command::LoadQueue => {
                tokio::spawn(async move {
                    let result = match web {
                        Ok(web) => web.queue().await,
                        Err(e) => Err(e),
                    };
                    sink(Event::Queue(result));
                });
            }
            Command::AddToQueue(uri) => {
                tokio::spawn(async move {
                    let result = match web {
                        Ok(web) => web.add_to_queue(&uri).await,
                        Err(e) => Err(e),
                    };
                    if let Err(e) = result {
                        sink(Event::CommandFailed {
                            message: format!("Couldn't add to queue. {}", e.message),
                        });
                    }
                });
            }
            Command::LoadPalette { artwork_url } => {
                let cached = self.palettes.lock().ok().and_then(|c| c.get(&artwork_url));
                if let Some(scheme) = cached {
                    sink(Event::Palette {
                        artwork_url,
                        scheme: Box::new(scheme),
                    });
                    return;
                }
                let session = self.session.clone();
                let palettes = self.palettes.clone();
                tokio::spawn(async move {
                    let scheme = match artwork::scheme_for(&session, &artwork_url).await {
                        Ok(s) => s,
                        Err(e) => {
                            log::debug!("palette unavailable: {e}");
                            Scheme::brand()
                        }
                    };
                    if let Ok(mut c) = palettes.lock() {
                        c.insert(artwork_url.clone(), scheme.clone());
                    }
                    sink(Event::Palette {
                        artwork_url,
                        scheme: Box::new(scheme),
                    });
                });
            }
            _ => {}
        }
    }
}

/// Command name for logs, without payloads (search queries are personal).
fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::SignInWithBrowser => "SignInWithBrowser",
        Command::CancelSignIn => "CancelSignIn",
        Command::SignOut => "SignOut",
        Command::Play => "Play",
        Command::Pause => "Pause",
        Command::Next => "Next",
        Command::Previous => "Previous",
        Command::Seek(_) => "Seek",
        Command::SetShuffle(_) => "SetShuffle",
        Command::SetRepeat(_) => "SetRepeat",
        Command::SetVolume(_) => "SetVolume",
        Command::PlayContext { .. } => "PlayContext",
        Command::PlayTracks { .. } => "PlayTracks",
        Command::AddToQueue(_) => "AddToQueue",
        Command::LoadHome => "LoadHome",
        Command::Search { .. } => "Search",
        Command::LoadLibrary { .. } => "LoadLibrary",
        Command::LoadCollection { .. } => "LoadCollection",
        Command::LoadQueue => "LoadQueue",
        Command::LoadPalette { .. } => "LoadPalette",
        Command::Shutdown => "Shutdown",
    }
}
