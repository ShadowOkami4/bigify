//! MPRIS (`org.mpris.MediaPlayer2.Bigify`) for desktop media keys, system
//! media widgets (KDE, GNOME), `playerctl` and Bluetooth/headset buttons.
//!
//! MPRIS reports the same *confirmed* state as the UI. Remote controls are
//! queued onto the Qt thread and go through the same `Backend` methods as
//! on-screen buttons, so requests show as pending and are confirmed (or
//! reported as failed) exactly like UI actions.
//!
//! The zbus-based player is not `Send`, so it runs on its own thread with a
//! single-threaded Tokio runtime and a `LocalSet`.

use bigify_core::playback::RepeatMode;

/// Snapshot of what MPRIS publishes. Sent from the Qt thread on change.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MprisState {
    pub status: Status,
    pub track_uri: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub art_url: String,
    pub length_ms: u32,
    pub position_ms: u32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// 0.0 ..= 1.0
    pub volume: f64,
    /// The position jumped (seek); clients must be told explicitly.
    pub seeked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Remote-control requests received over D-Bus.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    /// Relative seek in milliseconds (may be negative).
    SeekBy(i64),
    /// Absolute seek, only honoured if `track_uri` is still current.
    SetPosition {
        track_uri: String,
        position_ms: i64,
    },
    Shuffle(bool),
    Repeat(RepeatMode),
    Volume(f64),
}

const TRACK_PATH_PREFIX: &str = "/io/github/bigify/track/";
const NO_TRACK_PATH: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";

/// D-Bus object path identifying a track. Spotify ids are base62, so they
/// are valid path elements; anything else is reduced to `[A-Za-z0-9_]`.
pub fn track_object_path(uri: &str) -> String {
    let id: String = uri
        .rsplit(':')
        .next()
        .unwrap_or("")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if id.is_empty() {
        NO_TRACK_PATH.to_owned()
    } else {
        format!("{TRACK_PATH_PREFIX}{id}")
    }
}

/// Position for an absolute seek, clamped to the track; `None` when the
/// request targets a different track (per the MPRIS spec it is ignored).
pub fn resolve_set_position(state: &MprisState, track_path: &str, position_ms: i64) -> Option<u32> {
    if state.track_uri.is_empty() || track_object_path(&state.track_uri) != track_path {
        return None;
    }
    if position_ms < 0 || (state.length_ms > 0 && position_ms > i64::from(state.length_ms)) {
        return None;
    }
    Some(position_ms as u32)
}

#[cfg(all(feature = "mpris", target_os = "linux"))]
mod server {
    use std::rc::Rc;
    use std::sync::Arc;
    use std::thread::JoinHandle;

    use bigify_core::playback::RepeatMode;
    use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Player, Time, TrackId};
    use tokio::sync::mpsc;

    use super::{track_object_path, Control, MprisState, Status};

    pub type ControlSink = Arc<dyn Fn(Control) + Send + Sync>;

    pub struct Handle {
        tx: Option<mpsc::UnboundedSender<MprisState>>,
        thread: Option<JoinHandle<()>>,
    }

    impl Handle {
        pub fn update(&self, state: MprisState) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(state);
            }
        }

        pub fn shutdown(mut self) {
            self.tx.take();
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }

    pub fn start(control: ControlSink) -> Option<Handle> {
        let (tx, rx) = mpsc::unbounded_channel();
        let thread = std::thread::Builder::new()
            .name("bigify-mpris".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        log::warn!("MPRIS unavailable: {e}");
                        return;
                    }
                };
                let local = tokio::task::LocalSet::new();
                local.block_on(&rt, run(control, rx));
            })
            .map_err(|e| log::warn!("could not start MPRIS thread: {e}"))
            .ok()?;
        Some(Handle {
            tx: Some(tx),
            thread: Some(thread),
        })
    }

    fn loop_status(r: RepeatMode) -> LoopStatus {
        match r {
            RepeatMode::Off => LoopStatus::None,
            RepeatMode::Context => LoopStatus::Playlist,
            RepeatMode::Track => LoopStatus::Track,
        }
    }

    fn metadata(s: &MprisState) -> Metadata {
        if s.track_uri.is_empty() {
            return Metadata::new();
        }
        let mut b = Metadata::builder()
            .title(s.title.clone())
            .artist(s.artists.clone());
        if let Ok(id) = TrackId::try_from(track_object_path(&s.track_uri)) {
            b = b.trackid(id);
        }
        if !s.album.is_empty() {
            b = b.album(s.album.clone());
        }
        if !s.art_url.is_empty() {
            b = b.art_url(s.art_url.clone());
        }
        if s.length_ms > 0 {
            b = b.length(Time::from_millis(i64::from(s.length_ms)));
        }
        b.build()
    }

    async fn run(control: ControlSink, mut rx: mpsc::UnboundedReceiver<MprisState>) {
        let player = match Player::builder("Bigify")
            .identity("Bigify")
            .desktop_entry("io.github.ShadowOkami4.bigify")
            .can_play(false)
            .can_pause(false)
            .can_seek(false)
            .can_go_next(false)
            .can_go_previous(false)
            .can_control(true)
            .can_quit(false)
            .can_raise(false)
            .build()
            .await
        {
            Ok(p) => Rc::new(p),
            Err(e) => {
                // No session bus (e.g. some kiosk setups); everything else works.
                log::info!("MPRIS not available: {e}");
                return;
            }
        };
        log::info!("MPRIS registered as org.mpris.MediaPlayer2.Bigify");

        let current: Rc<std::cell::RefCell<MprisState>> = Rc::default();

        let c = control.clone();
        player.connect_play(move |_| c(Control::Play));
        let c = control.clone();
        player.connect_pause(move |_| c(Control::Pause));
        let c = control.clone();
        player.connect_stop(move |_| c(Control::Pause));
        let c = control.clone();
        player.connect_play_pause(move |_| c(Control::PlayPause));
        let c = control.clone();
        player.connect_next(move |_| c(Control::Next));
        let c = control.clone();
        player.connect_previous(move |_| c(Control::Previous));
        let c = control.clone();
        player.connect_seek(move |_, offset| c(Control::SeekBy(offset.as_millis())));
        let c = control.clone();
        let cur = current.clone();
        player.connect_set_position(move |_, track_id, position| {
            let path = track_id.as_str().to_owned();
            if super::resolve_set_position(&cur.borrow(), &path, position.as_millis()).is_some() {
                c(Control::SetPosition {
                    track_uri: cur.borrow().track_uri.clone(),
                    position_ms: position.as_millis(),
                });
            }
        });
        let c = control.clone();
        player.connect_set_shuffle(move |_, on| c(Control::Shuffle(on)));
        let c = control.clone();
        player.connect_set_loop_status(move |_, status| {
            c(Control::Repeat(match status {
                LoopStatus::None => RepeatMode::Off,
                LoopStatus::Track => RepeatMode::Track,
                LoopStatus::Playlist => RepeatMode::Context,
            }))
        });
        let c = control;
        player.connect_set_volume(move |_, v| c(Control::Volume(v.clamp(0.0, 1.0))));

        tokio::task::spawn_local(player.run());

        while let Some(next) = rx.recv().await {
            let prev = current.replace(next.clone());
            if let Err(e) = apply(&player, &prev, &next).await {
                log::debug!("MPRIS update failed: {e}");
            }
        }
        log::debug!("MPRIS stopped");
    }

    async fn apply(
        p: &Player,
        prev: &MprisState,
        s: &MprisState,
    ) -> mpris_server::zbus::Result<()> {
        let has_track = !s.track_uri.is_empty();
        let had_track = !prev.track_uri.is_empty();
        let first = *prev == MprisState::default() && s != prev;
        if first || has_track != had_track {
            p.set_can_play(has_track).await?;
            p.set_can_pause(has_track).await?;
            p.set_can_go_next(has_track).await?;
            p.set_can_go_previous(has_track).await?;
            p.set_can_seek(has_track).await?;
        }
        let meta_changed = first
            || prev.track_uri != s.track_uri
            || prev.title != s.title
            || prev.artists != s.artists
            || prev.album != s.album
            || prev.art_url != s.art_url
            || prev.length_ms != s.length_ms;
        if meta_changed {
            p.set_metadata(metadata(s)).await?;
        }
        // Position is read by clients on demand; it is not signalled.
        p.set_position(Time::from_millis(i64::from(s.position_ms)));
        if s.seeked {
            p.seeked(Time::from_millis(i64::from(s.position_ms)))
                .await?;
        }
        if first || prev.status != s.status {
            p.set_playback_status(match s.status {
                Status::Playing => PlaybackStatus::Playing,
                Status::Paused => PlaybackStatus::Paused,
                Status::Stopped => PlaybackStatus::Stopped,
            })
            .await?;
        }
        if first || prev.shuffle != s.shuffle {
            p.set_shuffle(s.shuffle).await?;
        }
        if first || prev.repeat != s.repeat {
            p.set_loop_status(loop_status(s.repeat)).await?;
        }
        if first || (prev.volume - s.volume).abs() > 0.001 {
            p.set_volume(s.volume).await?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "mpris", target_os = "linux"))]
pub use server::{start, ControlSink, Handle};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_paths_are_valid() {
        assert_eq!(
            track_object_path("spotify:track:4uLU6hMCjMI75M1A2tKUQC"),
            "/io/github/bigify/track/4uLU6hMCjMI75M1A2tKUQC"
        );
        assert_eq!(
            track_object_path("spotify:local:a-b:c d"),
            "/io/github/bigify/track/c_d"
        );
        assert_eq!(track_object_path(""), NO_TRACK_PATH);
    }

    #[test]
    fn set_position_requires_current_track_and_range() {
        let s = MprisState {
            track_uri: "spotify:track:abc".into(),
            length_ms: 10_000,
            ..MprisState::default()
        };
        let path = track_object_path(&s.track_uri);
        assert_eq!(resolve_set_position(&s, &path, 5_000), Some(5_000));
        assert_eq!(resolve_set_position(&s, &path, 20_000), None);
        assert_eq!(resolve_set_position(&s, &path, -1), None);
        assert_eq!(
            resolve_set_position(&s, "/io/github/bigify/track/other", 5_000),
            None
        );
    }
}
