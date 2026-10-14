//! The `Backend` QML singleton: the only object QML talks to.
//!
//! * Properties expose *confirmed* state (auth, playback, theme, settings)
//!   plus `pending`, the list of user requests still awaiting confirmation.
//! * Invokables record a request and forward a [`Command`] to the service.
//! * Signals deliver browse data (home, search, library, collection, queue)
//!   as JSON, which QML turns into `ListModel`s incrementally.
//!
//! Service events arrive on the service thread and are queued onto the Qt
//! thread via `CxxQtThread`, so all QObject access happens on the GUI thread.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bigify_core::color::Scheme;
use bigify_core::error::ServiceError;
use bigify_core::model::join_names;
use bigify_core::playback::{Intent, PlaybackState, PlayerUpdate, DEFAULT_CONFIRM_TIMEOUT_MS};
use bigify_core::settings::Settings;
use bigify_core::{paths, AuthState, Command, Event, LibraryKind};
use bigify_spotify::{EngineConfig, EventSink, ServiceHandle};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!("bigify/cpp/input.h");
        #[cxx_name = "bigifyPostAction"]
        fn post_action(action: i32, pressed: bool, autorepeat: bool);

        // `QObject` is declared implicitly by every cxx-qt bridge.
        #[cxx_name = "bigifyInstallInputModeFilter"]
        fn install_input_mode_filter(target: Pin<&mut QObject>);
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        // Session
        #[qproperty(QString, auth_state, cxx_name = "authState")]
        #[qproperty(QString, user_name, cxx_name = "userName")]
        #[qproperty(QString, device_name, cxx_name = "deviceName")]
        #[qproperty(QString, auth_message, cxx_name = "authMessage")]
        #[qproperty(bool, discovery_available, cxx_name = "discoveryAvailable")]
        // Playback (confirmed)
        #[qproperty(QString, playback_status, cxx_name = "playbackStatus")]
        #[qproperty(bool, is_playing, cxx_name = "isPlaying")]
        #[qproperty(bool, has_track, cxx_name = "hasTrack")]
        #[qproperty(QString, track_uri, cxx_name = "trackUri")]
        #[qproperty(QString, track_title, cxx_name = "trackTitle")]
        #[qproperty(QString, track_artists, cxx_name = "trackArtists")]
        #[qproperty(QString, track_album, cxx_name = "trackAlbum")]
        #[qproperty(QString, artwork_url, cxx_name = "artworkUrl")]
        #[qproperty(i32, duration_ms, cxx_name = "durationMs")]
        #[qproperty(i32, position_ms, cxx_name = "positionMs")]
        /// Wall-clock time (ms since epoch, like JS `Date.now()`) at which
        /// `positionMs` was confirmed.
        #[qproperty(f64, position_sampled_at, cxx_name = "positionSampledAt")]
        #[qproperty(bool, shuffle)]
        #[qproperty(QString, repeat_mode, cxx_name = "repeatMode")]
        #[qproperty(f64, volume)]
        /// Space-separated ids of requests awaiting confirmation.
        #[qproperty(QString, pending)]
        // Appearance & settings
        #[qproperty(QString, scheme_json, cxx_name = "schemeJson")]
        #[qproperty(QString, settings_json, cxx_name = "settingsJson")]
        #[qproperty(bool, restart_required, cxx_name = "restartRequired")]
        #[qproperty(QString, app_version, cxx_name = "appVersion")]
        #[qproperty(QString, log_file_path, cxx_name = "logFilePath")]
        #[qproperty(bool, design_preview, cxx_name = "designPreview")]
        /// Developer options from the environment (screenshot path, start
        /// screen, window size) as JSON. See docs/DEVELOPMENT.md.
        #[qproperty(QString, dev_json, cxx_name = "devJson")]
        /// "gamepad" or "keyboard" (keyboard, mouse, touch): which button
        /// hints to show. Kept current by the C++ input-mode filter.
        #[qproperty(QString, input_mode, cxx_name = "inputMode")]
        /// Library access via the user's own Spotify app, as JSON
        /// `{status, message}`; status is "notConfigured", "setup",
        /// "connected" or "error".
        #[qproperty(QString, library_access, cxx_name = "libraryAccess")]
        /// Button-label family of the controller in use: "deck", "xbox",
        /// "playstation" or "nintendo".
        #[qproperty(QString, controller_family, cxx_name = "controllerFamily")]
        type Backend = super::BackendRust;

        #[qinvokable]
        fn sign_in_with_browser(self: Pin<&mut Self>);
        #[qinvokable]
        fn cancel_sign_in(self: Pin<&mut Self>);
        #[qinvokable]
        fn sign_out(self: Pin<&mut Self>);
        /// Open the one-time library access setup page in the browser.
        #[qinvokable]
        fn start_library_setup(self: Pin<&mut Self>);
        #[qinvokable]
        fn cancel_library_setup(self: Pin<&mut Self>);
        #[qinvokable]
        fn remove_library_access(self: Pin<&mut Self>);

        #[qinvokable]
        fn toggle_play(self: Pin<&mut Self>);
        #[qinvokable]
        fn next(self: Pin<&mut Self>);
        #[qinvokable]
        fn previous(self: Pin<&mut Self>);
        #[qinvokable]
        fn seek(self: Pin<&mut Self>, position_ms: i32);
        #[qinvokable]
        fn toggle_shuffle(self: Pin<&mut Self>);
        #[qinvokable]
        fn cycle_repeat(self: Pin<&mut Self>);
        #[qinvokable]
        fn set_volume_level(self: Pin<&mut Self>, level: f64);
        #[qinvokable]
        fn step_volume(self: Pin<&mut Self>, delta: f64);
        /// Play `context_uri` (album/playlist/artist/"spotify:collection"),
        /// optionally starting at `track_uri`. With an empty context, plays
        /// the single track.
        #[qinvokable]
        fn play_uri(self: Pin<&mut Self>, context_uri: &QString, track_uri: &QString);
        #[qinvokable]
        fn add_to_queue(self: Pin<&mut Self>, uri: &QString);
        /// Save/remove (follow/unfollow for artists). Confirmation arrives
        /// as `savedChanged`, failure as `saveFailed`.
        #[qinvokable]
        fn set_saved(self: Pin<&mut Self>, uri: &QString, saved: bool);
        /// `uris_json` is a JSON array of Spotify URIs.
        #[qinvokable]
        fn check_saved(self: Pin<&mut Self>, uris_json: &QString);
        /// Report requests that were never confirmed. Called by QML on a
        /// timer only while `pending` is non-empty.
        #[qinvokable]
        fn expire_pending(self: Pin<&mut Self>);

        #[qinvokable]
        fn load_home(self: Pin<&mut Self>);
        /// Returns a request id; results for older ids should be ignored.
        #[qinvokable]
        fn search(self: Pin<&mut Self>, query: &QString) -> i32;
        #[qinvokable]
        fn load_library(self: Pin<&mut Self>, kind: &QString, offset: i32);
        #[qinvokable]
        fn load_collection(self: Pin<&mut Self>, uri: &QString);
        #[qinvokable]
        fn load_artist(self: Pin<&mut Self>, uri: &QString);
        #[qinvokable]
        fn load_queue(self: Pin<&mut Self>);

        /// Update one setting (`value_json` is a JSON value). Returns false
        /// if the key or value is invalid.
        #[qinvokable]
        fn set_setting(self: Pin<&mut Self>, key: &QString, value_json: &QString) -> bool;
        #[qinvokable]
        fn diagnostics(self: &Self) -> QString;

        #[qsignal]
        fn home_loaded(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        fn home_failed(self: Pin<&mut Self>, kind: QString, message: QString);
        #[qsignal]
        fn search_finished(self: Pin<&mut Self>, request_id: i32, json: QString);
        #[qsignal]
        fn search_failed(self: Pin<&mut Self>, request_id: i32, kind: QString, message: QString);
        #[qsignal]
        fn library_loaded(self: Pin<&mut Self>, kind: QString, offset: i32, json: QString);
        #[qsignal]
        fn library_failed(
            self: Pin<&mut Self>,
            kind: QString,
            offset: i32,
            error_kind: QString,
            message: QString,
        );
        #[qsignal]
        fn collection_loaded(self: Pin<&mut Self>, uri: QString, json: QString);
        #[qsignal]
        fn collection_failed(self: Pin<&mut Self>, uri: QString, kind: QString, message: QString);
        #[qsignal]
        fn artist_loaded(self: Pin<&mut Self>, uri: QString, json: QString);
        #[qsignal]
        fn artist_failed(self: Pin<&mut Self>, uri: QString, kind: QString, message: QString);
        #[qsignal]
        fn queue_loaded(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        fn queue_failed(self: Pin<&mut Self>, kind: QString, message: QString);
        #[qsignal]
        fn saved_changed(self: Pin<&mut Self>, uri: QString, saved: bool);
        #[qsignal]
        fn save_failed(self: Pin<&mut Self>, uri: QString, message: QString);
        /// Transient message for a toast. `level` is "info" or "error".
        #[qsignal]
        fn notify(self: Pin<&mut Self>, level: QString, message: QString);
    }

    impl cxx_qt::Threading for Backend {}
    impl cxx_qt::Initialize for Backend {}
}

/// Process-wide resources that must outlive QML and be shut down explicitly
/// from `main` (never from a QObject destructor, which could deadlock with a
/// service thread queueing onto the dying object).
struct Runtime {
    service: Option<ServiceHandle>,
    #[cfg(all(feature = "mpris", target_os = "linux"))]
    mpris: Option<crate::mpris::Handle>,
    input_stop: Arc<AtomicBool>,
    input_enabled: Arc<AtomicBool>,
    input_thread: Option<JoinHandle<()>>,
}

static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

fn send(cmd: Command) -> bool {
    match RUNTIME.lock() {
        Ok(rt) => rt
            .as_ref()
            .and_then(|r| r.service.as_ref())
            .is_some_and(|s| s.send(cmd)),
        Err(_) => false,
    }
}

/// Stop the controller thread and the Spotify service. Call after the Qt
/// event loop has returned.
pub fn shutdown() {
    let runtime = RUNTIME.lock().ok().and_then(|mut rt| rt.take());
    if let Some(mut rt) = runtime {
        rt.input_stop.store(true, Ordering::Relaxed);
        #[cfg(all(feature = "mpris", target_os = "linux"))]
        if let Some(mpris) = rt.mpris.take() {
            mpris.shutdown();
        }
        if let Some(service) = rt.service.take() {
            service.shutdown();
        }
        if let Some(t) = rt.input_thread.take() {
            let _ = t.join();
        }
    }
}

fn wall_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

fn json<T: serde::Serialize>(value: &T) -> QString {
    QString::from(
        serde_json::to_string(value)
            .unwrap_or_else(|_| "null".into())
            .as_str(),
    )
}

fn error_kind(e: &ServiceError) -> QString {
    json(&e.kind).to_string().trim_matches('"').into()
}

/// Settings keys that only take effect when the playback engine restarts.
const RESTART_KEYS: &[&str] = &[
    "deviceName",
    "audioQuality",
    "audioBackend",
    "normalisation",
    "gapless",
    "initialVolume",
    "zeroconfDiscovery",
];

pub struct BackendRust {
    auth_state: QString,
    user_name: QString,
    device_name: QString,
    auth_message: QString,
    discovery_available: bool,
    playback_status: QString,
    is_playing: bool,
    has_track: bool,
    track_uri: QString,
    track_title: QString,
    track_artists: QString,
    track_album: QString,
    artwork_url: QString,
    duration_ms: i32,
    position_ms: i32,
    position_sampled_at: f64,
    shuffle: bool,
    repeat_mode: QString,
    volume: f64,
    pending: QString,
    scheme_json: QString,
    settings_json: QString,
    restart_required: bool,
    app_version: QString,
    log_file_path: QString,
    design_preview: bool,
    dev_json: QString,
    input_mode: QString,
    library_access: QString,
    controller_family: QString,

    playback: PlaybackState,
    settings: Settings,
    epoch: Instant,
    next_search_id: i32,
    /// Artwork URL whose palette is (or will be) applied.
    palette_for: String,
}

impl Default for BackendRust {
    fn default() -> Self {
        Self {
            auth_state: QString::from("starting"),
            user_name: QString::default(),
            device_name: QString::default(),
            auth_message: QString::default(),
            discovery_available: false,
            playback_status: QString::from("idle"),
            is_playing: false,
            has_track: false,
            track_uri: QString::default(),
            track_title: QString::default(),
            track_artists: QString::default(),
            track_album: QString::default(),
            artwork_url: QString::default(),
            duration_ms: 0,
            position_ms: 0,
            position_sampled_at: 0.0,
            shuffle: false,
            repeat_mode: QString::from("off"),
            volume: 0.0,
            pending: QString::default(),
            scheme_json: json(&Scheme::brand()),
            settings_json: QString::default(),
            restart_required: false,
            app_version: QString::from(env!("CARGO_PKG_VERSION")),
            log_file_path: QString::from(paths::log_file().to_string_lossy().as_ref()),
            design_preview: false,
            dev_json: QString::from("{}"),
            input_mode: QString::from("keyboard"),
            library_access: QString::from(r#"{"status":"notConfigured","message":""}"#),
            controller_family: QString::from("xbox"),
            playback: PlaybackState::new(),
            settings: Settings::default(),
            epoch: Instant::now(),
            next_search_id: 0,
            palette_for: String::new(),
        }
    }
}

impl cxx_qt::Initialize for qobject::Backend {
    fn initialize(mut self: Pin<&mut Self>) {
        let settings = match Settings::load(&paths::settings_file()) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("{e}; using defaults");
                Settings::default()
            }
        };
        self.as_mut().set_settings_json(json(&settings));
        self.as_mut()
            .set_device_name(QString::from(settings.device_name.as_str()));
        self.as_mut().rust_mut().settings = settings.clone();

        let preview = std::env::var_os("BIGIFY_DESIGN_PREVIEW").is_some_and(|v| v == "1");
        self.as_mut().set_design_preview(preview);
        let env = |k: &str| std::env::var(k).unwrap_or_default();
        self.as_mut().set_dev_json(json(&serde_json::json!({
            "screenshot": env("BIGIFY_SCREENSHOT"),
            "startScreen": env("BIGIFY_START_SCREEN"),
            "windowSize": env("BIGIFY_WINDOW_SIZE"),
            "inputMode": env("BIGIFY_INPUT_MODE"),
            "controller": env("BIGIFY_CONTROLLER"),
        })));

        {
            use cxx_qt::casting::Upcast;
            let obj: Pin<&mut cxx_qt::QObject> = self.as_mut().upcast_pin();
            qobject::install_input_mode_filter(obj);
        }

        let input_stop = Arc::new(AtomicBool::new(false));
        let input_enabled = Arc::new(AtomicBool::new(settings.controller_input));
        let input_thread = start_input(&self, input_stop.clone(), input_enabled.clone());

        // The design preview never touches Spotify; QML fills it with
        // clearly labelled sample content.
        let service = (!preview).then(|| {
            let qt = self.qt_thread();
            let sink: EventSink = Arc::new(move |event: Event| {
                // Fails only if the Backend is already destroyed (shutdown).
                let _ = qt.queue(move |obj| obj.apply_event(event));
            });
            bigify_spotify::spawn(
                EngineConfig {
                    settings,
                    cache_dir: paths::cache_dir(),
                    state_dir: paths::state_dir(),
                },
                sink,
            )
        });

        #[cfg(all(feature = "mpris", target_os = "linux"))]
        let mpris = (!preview)
            .then(|| {
                let qt = self.qt_thread();
                let control: crate::mpris::ControlSink = Arc::new(move |c| {
                    let _ = qt.queue(move |obj| obj.handle_mpris(c));
                });
                crate::mpris::start(control)
            })
            .flatten();

        if let Ok(mut rt) = RUNTIME.lock() {
            *rt = Some(Runtime {
                service,
                #[cfg(all(feature = "mpris", target_os = "linux"))]
                mpris,
                input_stop,
                input_enabled,
                input_thread,
            });
        }
    }
}

#[cfg(feature = "gamepad")]
fn start_input(
    backend: &Pin<&mut qobject::Backend>,
    stop: Arc<AtomicBool>,
    enabled: Arc<AtomicBool>,
) -> Option<JoinHandle<()>> {
    let qt = backend.qt_thread();
    let qt_family = backend.qt_thread();
    std::thread::Builder::new()
        .name("bigify-input".into())
        .spawn(move || {
            crate::input::gamepad::run(
                stop,
                enabled,
                move |events| {
                    let _ = qt.queue(move |_| {
                        for e in events {
                            qobject::post_action(e.action as i32, e.pressed, e.autorepeat);
                        }
                    });
                },
                move |family| {
                    let _ = qt_family.queue(move |mut obj| {
                        obj.as_mut().set_controller_family(QString::from(family));
                        obj.as_mut().set_input_mode(QString::from("gamepad"));
                    });
                },
            )
        })
        .map_err(|e| log::warn!("could not start controller thread: {e}"))
        .ok()
}

#[cfg(not(feature = "gamepad"))]
fn start_input(
    _: &Pin<&mut qobject::Backend>,
    _: Arc<AtomicBool>,
    _: Arc<AtomicBool>,
) -> Option<JoinHandle<()>> {
    None
}

impl qobject::Backend {
    fn now_ms(&self) -> u64 {
        self.rust().epoch.elapsed().as_millis() as u64
    }

    fn notify_error(self: Pin<&mut Self>, message: &str) {
        self.notify(QString::from("error"), QString::from(message));
    }

    /// Record a playback request and send the command; reports immediately
    /// if the service is unavailable.
    fn request(mut self: Pin<&mut Self>, intent: Intent, cmd: Command) {
        if !self.auth_state().to_string().eq("connected") {
            self.notify_error("Not connected to Spotify yet");
            return;
        }
        let now = self.now_ms();
        self.as_mut().rust_mut().playback.request(intent, now);
        self.as_mut().sync_pending();
        if !send(cmd) {
            self.as_mut().rust_mut().playback.clear_pending();
            self.as_mut().sync_pending();
            self.notify_error("The playback service isn't running");
        }
    }

    fn sync_pending(mut self: Pin<&mut Self>) {
        let ids: Vec<&str> = self.rust().playback.pending().map(Intent::id).collect();
        let joined = QString::from(ids.join(" ").as_str());
        if *self.pending() != joined {
            self.as_mut().set_pending(joined);
        }
    }

    /// Push confirmed playback state into the QML properties.
    fn sync_player(mut self: Pin<&mut Self>) {
        let now = self.now_ms();
        let p = &self.rust().playback;
        let status = QString::from(p.status.id());
        let is_playing = p.is_playing();
        let has_track = p.has_track();
        let track = p.track.clone();
        let position = p.position_at(now).min(i32::MAX as u32) as i32;
        let shuffle = p.shuffle;
        let repeat = QString::from(p.repeat.id());
        let volume = f64::from(p.volume) / f64::from(u16::MAX);

        self.as_mut().set_playback_status(status);
        self.as_mut().set_is_playing(is_playing);
        self.as_mut().set_has_track(has_track);
        let (uri, title, artists, album, art, duration) = match &track {
            Some(t) => (
                t.uri.as_str(),
                t.title.as_str(),
                join_names(t.artists.iter().map(String::as_str)),
                t.album.clone().unwrap_or_default(),
                t.artwork_url.clone().unwrap_or_default(),
                t.duration_ms.min(i32::MAX as u32) as i32,
            ),
            None => ("", "", String::new(), String::new(), String::new(), 0),
        };
        self.as_mut().set_track_uri(QString::from(uri));
        self.as_mut().set_track_title(QString::from(title));
        self.as_mut()
            .set_track_artists(QString::from(artists.as_str()));
        self.as_mut().set_track_album(QString::from(album.as_str()));
        self.as_mut().set_artwork_url(QString::from(art.as_str()));
        self.as_mut().set_duration_ms(duration);
        // Sample time first so bindings never extrapolate from a stale pair.
        self.as_mut().set_position_sampled_at(wall_ms());
        self.as_mut().set_position_ms(position);
        self.as_mut().set_shuffle(shuffle);
        self.as_mut().set_repeat_mode(repeat);
        self.as_mut().set_volume(volume);
        self.as_mut().sync_pending();
    }

    /// Publish confirmed playback state over MPRIS.
    fn publish_mpris(&self, seeked: bool) {
        #[cfg(all(feature = "mpris", target_os = "linux"))]
        {
            use crate::mpris::{MprisState, Status};
            let p = &self.rust().playback;
            let track = p.track.as_ref();
            let state = MprisState {
                status: match p.status {
                    bigify_core::playback::PlaybackStatus::Playing => Status::Playing,
                    _ if p.has_track() => Status::Paused,
                    _ => Status::Stopped,
                },
                track_uri: track.map(|t| t.uri.clone()).unwrap_or_default(),
                title: track.map(|t| t.title.clone()).unwrap_or_default(),
                artists: track.map(|t| t.artists.clone()).unwrap_or_default(),
                album: track.and_then(|t| t.album.clone()).unwrap_or_default(),
                art_url: track
                    .and_then(|t| t.artwork_url.clone())
                    .unwrap_or_default(),
                length_ms: track.map(|t| t.duration_ms).unwrap_or(0),
                position_ms: p.position_at(self.now_ms()),
                shuffle: p.shuffle,
                repeat: p.repeat,
                volume: f64::from(p.volume) / f64::from(u16::MAX),
                seeked,
            };
            if let Ok(rt) = RUNTIME.lock() {
                if let Some(m) = rt.as_ref().and_then(|r| r.mpris.as_ref()) {
                    m.update(state);
                }
            }
        }
        #[cfg(not(all(feature = "mpris", target_os = "linux")))]
        let _ = seeked;
    }

    /// Remote control from MPRIS, routed through the same paths as the UI.
    #[cfg(all(feature = "mpris", target_os = "linux"))]
    fn handle_mpris(mut self: Pin<&mut Self>, control: crate::mpris::Control) {
        use crate::mpris::Control;
        let playing = self.rust().playback.is_playing();
        match control {
            Control::Play if !playing => self.toggle_play(),
            Control::Pause if playing => self.toggle_play(),
            Control::Play | Control::Pause => {}
            Control::PlayPause => self.toggle_play(),
            Control::Next => self.next(),
            Control::Previous => self.previous(),
            Control::SeekBy(delta) => {
                let now = self.now_ms();
                let pos = i64::from(self.rust().playback.position_at(now)) + delta;
                self.seek(pos.clamp(0, i64::from(i32::MAX)) as i32);
            }
            Control::SetPosition {
                track_uri,
                position_ms,
            } => {
                if self.track_uri().to_string() == track_uri {
                    self.seek(position_ms.clamp(0, i64::from(i32::MAX)) as i32);
                }
            }
            Control::Shuffle(on) => {
                if self.rust().playback.shuffle != on {
                    self.toggle_shuffle();
                }
            }
            Control::Repeat(mode) => {
                self.as_mut()
                    .request(Intent::Repeat(mode), Command::SetRepeat(mode));
            }
            Control::Volume(v) => self.set_volume_level(v),
        }
    }

    fn refresh_palette(mut self: Pin<&mut Self>) {
        let art = self.artwork_url().to_string();
        if !self.rust().settings.dynamic_color || art.is_empty() {
            if !self.rust().palette_for.is_empty() {
                self.as_mut().rust_mut().palette_for.clear();
                self.as_mut().set_scheme_json(json(&Scheme::brand()));
            }
            return;
        }
        if self.rust().palette_for == art {
            return;
        }
        self.as_mut().rust_mut().palette_for = art.clone();
        send(Command::LoadPalette { artwork_url: art });
    }

    fn apply_event(mut self: Pin<&mut Self>, event: Event) {
        match event {
            Event::Auth(state) => {
                let (user, message, discovery) = match &state {
                    AuthState::Connected { user, .. } => (user.clone(), String::new(), false),
                    AuthState::SignedOut { discovery } => {
                        (String::new(), String::new(), *discovery)
                    }
                    AuthState::Reconnecting { attempt } => (
                        self.user_name().to_string(),
                        format!("Connection lost. Reconnecting (attempt {attempt})…"),
                        false,
                    ),
                    AuthState::Failed { message } => (String::new(), message.clone(), false),
                    _ => (self.user_name().to_string(), String::new(), false),
                };
                if !state.is_connected() {
                    self.as_mut().rust_mut().playback.clear_pending();
                    self.as_mut().sync_pending();
                }
                self.as_mut().set_user_name(QString::from(user.as_str()));
                self.as_mut()
                    .set_auth_message(QString::from(message.as_str()));
                self.as_mut().set_discovery_available(discovery);
                self.as_mut().set_auth_state(QString::from(state.id()));
            }
            Event::Player(update) => {
                let now = self.now_ms();
                let seeked = matches!(update, PlayerUpdate::Seeked { .. });
                let outcome = self.as_mut().rust_mut().playback.apply(update, now);
                self.as_mut().sync_player();
                self.publish_mpris(seeked);
                if let Some(err) = outcome.error {
                    self.as_mut().notify_error(&err);
                }
                // Learn whether the new track is in Liked Songs.
                if outcome.track_changed {
                    let uri = self.track_uri().to_string();
                    if uri.starts_with("spotify:") {
                        send(Command::CheckSaved { uris: vec![uri] });
                    }
                }
                self.as_mut().refresh_palette();
            }
            Event::Home(Ok(sections)) => self.home_loaded(json(&sections)),
            Event::Home(Err(e)) => {
                self.home_failed(error_kind(&e), QString::from(e.message.as_str()))
            }
            Event::Search { request_id, result } => {
                let id = request_id as i32;
                match result {
                    Ok(r) => self.search_finished(id, json(&r)),
                    Err(e) => {
                        self.search_failed(id, error_kind(&e), QString::from(e.message.as_str()))
                    }
                }
            }
            Event::Library {
                kind,
                offset,
                result,
            } => {
                let k = QString::from(kind.id());
                let offset = offset as i32;
                match result {
                    Ok(page) => self.library_loaded(k, offset, json(&page)),
                    Err(e) => self.library_failed(
                        k,
                        offset,
                        error_kind(&e),
                        QString::from(e.message.as_str()),
                    ),
                }
            }
            Event::Collection { uri, result } => {
                let u = QString::from(uri.as_str());
                match result {
                    Ok(d) => self.collection_loaded(u, json(&d)),
                    Err(e) => {
                        self.collection_failed(u, error_kind(&e), QString::from(e.message.as_str()))
                    }
                }
            }
            Event::Artist { uri, result } => {
                let u = QString::from(uri.as_str());
                match result {
                    Ok(d) => self.artist_loaded(u, json(&d)),
                    Err(e) => {
                        self.artist_failed(u, error_kind(&e), QString::from(e.message.as_str()))
                    }
                }
            }
            Event::Queue(Ok(q)) => self.queue_loaded(json(&q)),
            Event::Queue(Err(e)) => {
                self.queue_failed(error_kind(&e), QString::from(e.message.as_str()))
            }
            Event::Palette {
                artwork_url,
                scheme,
            } => {
                // Ignore palettes for artwork that is no longer current.
                if self.rust().settings.dynamic_color && self.rust().palette_for == artwork_url {
                    self.as_mut().set_scheme_json(json(&scheme));
                }
            }
            Event::Saved { states } => {
                for (uri, saved) in states {
                    self.as_mut()
                        .saved_changed(QString::from(uri.as_str()), saved);
                }
            }
            Event::SaveFailed { uri, message } => {
                self.as_mut()
                    .save_failed(QString::from(uri.as_str()), QString::from(message.as_str()));
                self.notify_error(&format!("Couldn't update your library. {message}"));
            }
            Event::LibraryAccess { status, message } => {
                let previous = self.library_access().to_string();
                let was_connected = previous.contains(r#""status":"connected""#);
                match status.as_str() {
                    "error" => self
                        .as_mut()
                        .notify_error(&format!("Library access: {message}")),
                    "connected" if !was_connected => self.as_mut().notify(
                        QString::from("info"),
                        QString::from("Library access connected"),
                    ),
                    _ => {}
                }
                self.as_mut().set_library_access(json(&serde_json::json!({
                    "status": status,
                    "message": message,
                })));
            }
            Event::CommandFailed { message } => {
                self.as_mut().rust_mut().playback.clear_pending();
                self.as_mut().sync_pending();
                self.notify_error(&message);
            }
        }
    }

    // -- Invokables ----------------------------------------------------------

    pub fn sign_in_with_browser(self: Pin<&mut Self>) {
        send(Command::SignInWithBrowser);
    }

    pub fn cancel_sign_in(self: Pin<&mut Self>) {
        send(Command::CancelSignIn);
    }

    pub fn sign_out(self: Pin<&mut Self>) {
        send(Command::SignOut);
    }

    pub fn start_library_setup(self: Pin<&mut Self>) {
        if !send(Command::StartLibrarySetup) {
            self.notify_error("Not connected to Spotify yet");
        }
    }

    pub fn cancel_library_setup(self: Pin<&mut Self>) {
        send(Command::CancelLibrarySetup);
    }

    pub fn remove_library_access(self: Pin<&mut Self>) {
        send(Command::RemoveLibraryAccess);
    }

    pub fn toggle_play(self: Pin<&mut Self>) {
        if !self.rust().playback.has_track() {
            self.notify(
                QString::from("info"),
                QString::from("Choose something to play first"),
            );
            return;
        }
        // Base the toggle on the newest request if one is pending, so a
        // quick double press does what the user expects.
        let wants_pause = match self.rust().playback.pending_transport() {
            Some(Intent::Play) => true,
            Some(Intent::Pause) => false,
            _ => self.rust().playback.is_playing(),
        };
        if wants_pause {
            self.request(Intent::Pause, Command::Pause);
        } else {
            self.request(Intent::Play, Command::Play);
        }
    }

    pub fn next(self: Pin<&mut Self>) {
        self.request(Intent::Next, Command::Next);
    }

    pub fn previous(self: Pin<&mut Self>) {
        self.request(Intent::Previous, Command::Previous);
    }

    pub fn seek(self: Pin<&mut Self>, position_ms: i32) {
        let ms = position_ms.max(0) as u32;
        self.request(Intent::Seek(ms), Command::Seek(ms));
    }

    pub fn toggle_shuffle(self: Pin<&mut Self>) {
        let target = !self.rust().playback.shuffle;
        self.request(Intent::Shuffle(target), Command::SetShuffle(target));
    }

    pub fn cycle_repeat(self: Pin<&mut Self>) {
        let target = self.rust().playback.repeat.cycled();
        self.request(Intent::Repeat(target), Command::SetRepeat(target));
    }

    pub fn set_volume_level(self: Pin<&mut Self>, level: f64) {
        let v = (level.clamp(0.0, 1.0) * f64::from(u16::MAX)).round() as u16;
        self.request(Intent::Volume(v), Command::SetVolume(v));
    }

    pub fn step_volume(self: Pin<&mut Self>, delta: f64) {
        let level = *self.volume() + delta;
        self.set_volume_level(level);
    }

    pub fn play_uri(self: Pin<&mut Self>, context_uri: &QString, track_uri: &QString) {
        let context = context_uri.to_string();
        let track = track_uri.to_string();
        let (intent_uri, cmd) = if context.is_empty() {
            if track.is_empty() {
                return;
            }
            (
                track.clone(),
                Command::PlayTracks {
                    uris: vec![track],
                    index: 0,
                },
            )
        } else {
            (
                context.clone(),
                Command::PlayContext {
                    context_uri: context,
                    track_uri: (!track.is_empty()).then_some(track),
                },
            )
        };
        self.request(Intent::Load(intent_uri), cmd);
    }

    pub fn add_to_queue(self: Pin<&mut Self>, uri: &QString) {
        let uri = uri.to_string();
        if !uri.is_empty() && send(Command::AddToQueue(uri)) {
            self.notify(QString::from("info"), QString::from("Added to queue"));
        }
    }

    pub fn set_saved(self: Pin<&mut Self>, uri: &QString, saved: bool) {
        let uri = uri.to_string();
        if !uri.is_empty()
            && !send(Command::SetSaved {
                uri: uri.clone(),
                saved,
            })
        {
            self.save_failed(QString::from(uri.as_str()), QString::from("Not connected"));
        }
    }

    pub fn check_saved(self: Pin<&mut Self>, uris_json: &QString) {
        let uris: Vec<String> = serde_json::from_str(&uris_json.to_string()).unwrap_or_default();
        let uris: Vec<String> = uris
            .into_iter()
            .filter(|u| u.starts_with("spotify:"))
            .take(200)
            .collect();
        if !uris.is_empty() {
            send(Command::CheckSaved { uris });
        }
    }

    pub fn expire_pending(mut self: Pin<&mut Self>) {
        let now = self.now_ms();
        let expired = self
            .as_mut()
            .rust_mut()
            .playback
            .expire(now, DEFAULT_CONFIRM_TIMEOUT_MS);
        if expired.is_empty() {
            return;
        }
        self.as_mut().sync_pending();
        if let Some(first) = expired.first() {
            self.notify_error(first.failure_message());
        }
    }

    pub fn load_home(self: Pin<&mut Self>) {
        send(Command::LoadHome);
    }

    pub fn search(mut self: Pin<&mut Self>, query: &QString) -> i32 {
        let id = self.rust().next_search_id.wrapping_add(1).max(1);
        self.as_mut().rust_mut().next_search_id = id;
        let query = query.to_string().trim().to_owned();
        if !query.is_empty() {
            send(Command::Search {
                request_id: id as u32,
                query,
            });
        }
        id
    }

    pub fn load_library(self: Pin<&mut Self>, kind: &QString, offset: i32) {
        if let Some(kind) = LibraryKind::from_id(&kind.to_string()) {
            send(Command::LoadLibrary {
                kind,
                offset: offset.max(0) as u32,
            });
        }
    }

    pub fn load_collection(self: Pin<&mut Self>, uri: &QString) {
        send(Command::LoadCollection {
            uri: uri.to_string(),
        });
    }

    pub fn load_artist(self: Pin<&mut Self>, uri: &QString) {
        send(Command::LoadArtist {
            uri: uri.to_string(),
        });
    }

    pub fn load_queue(self: Pin<&mut Self>) {
        send(Command::LoadQueue);
    }

    pub fn set_setting(mut self: Pin<&mut Self>, key: &QString, value_json: &QString) -> bool {
        let key = key.to_string();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&value_json.to_string()) else {
            return false;
        };
        let mut settings = self.rust().settings.clone();
        if !settings.set_from_json(&key, &value) {
            return false;
        }
        if settings == self.rust().settings {
            return true;
        }
        if let Err(e) = settings.save(&paths::settings_file()) {
            log::error!("{e}");
            self.notify_error("Couldn't save settings");
            return false;
        }
        if RESTART_KEYS.contains(&key.as_str()) {
            self.as_mut().set_restart_required(true);
        }
        if key == "controllerInput" {
            if let Ok(rt) = RUNTIME.lock() {
                if let Some(rt) = rt.as_ref() {
                    rt.input_enabled
                        .store(settings.controller_input, Ordering::Relaxed);
                }
            }
        }
        self.as_mut().set_settings_json(json(&settings));
        self.as_mut().rust_mut().settings = settings;
        if key == "dynamicColor" {
            self.as_mut().rust_mut().palette_for.clear();
            self.as_mut().set_scheme_json(json(&Scheme::brand()));
            self.as_mut().refresh_palette();
        }
        true
    }

    pub fn diagnostics(&self) -> QString {
        json(&serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "authState": self.auth_state().to_string(),
            "playback": self.playback_status().to_string(),
            "configFile": paths::settings_file(),
            "logFile": paths::log_file(),
            "cacheDir": paths::cache_dir(),
            "gamepadSupport": cfg!(feature = "gamepad"),
            "audioBackend": self.rust().settings.audio_backend.librespot_name(),
        }))
    }
}
