//! Playback state with an explicit split between what the user *requested*
//! and what the engine *confirmed*.
//!
//! The UI never flips the play button just because it was pressed. A request
//! is recorded as [`Intent`] (shown as a pending indicator) and the confirmed
//! state only changes when the engine reports a [`PlayerUpdate`]. Requests
//! that are never confirmed expire and are surfaced as errors.
//!
//! Time is passed in explicitly as milliseconds on a monotonic clock so the
//! logic is deterministic and testable.

use serde::Serialize;

use crate::model::TrackInfo;

/// How long a request may stay unconfirmed before it is reported as failed.
pub const DEFAULT_CONFIRM_TIMEOUT_MS: u64 = 8_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    /// Nothing loaded in the local player.
    #[default]
    Idle,
    /// A track is buffering.
    Loading,
    Playing,
    Paused,
}

impl PlaybackStatus {
    pub fn id(self) -> &'static str {
        match self {
            PlaybackStatus::Idle => "idle",
            PlaybackStatus::Loading => "loading",
            PlaybackStatus::Playing => "playing",
            PlaybackStatus::Paused => "paused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatMode {
    #[default]
    Off,
    Context,
    Track,
}

impl RepeatMode {
    /// Spotify's button order: off → repeat all → repeat one → off.
    pub fn cycled(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::Context,
            RepeatMode::Context => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Off,
        }
    }

    pub fn from_flags(context: bool, track: bool) -> Self {
        if track {
            RepeatMode::Track
        } else if context {
            RepeatMode::Context
        } else {
            RepeatMode::Off
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            RepeatMode::Off => "off",
            RepeatMode::Context => "context",
            RepeatMode::Track => "track",
        }
    }
}

/// Confirmed facts reported by the playback engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerUpdate {
    Loading {
        uri: String,
        position_ms: u32,
    },
    Playing {
        uri: String,
        position_ms: u32,
    },
    Paused {
        uri: String,
        position_ms: u32,
    },
    Stopped,
    Seeked {
        position_ms: u32,
    },
    /// Periodic position report or correction while playing.
    Position {
        position_ms: u32,
    },
    TrackChanged(TrackInfo),
    /// The engine could not play the requested track.
    Unavailable {
        uri: String,
    },
    Volume(u16),
    Shuffle(bool),
    Repeat(RepeatMode),
}

/// A user request awaiting confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Play,
    Pause,
    Next,
    Previous,
    Seek(u32),
    Shuffle(bool),
    Repeat(RepeatMode),
    Volume(u16),
    /// Start playing a context or track URI.
    Load(String),
}

impl Intent {
    /// Requests in the same slot supersede each other (pressing play then
    /// pause quickly leaves only "pause" pending).
    fn slot(&self) -> u8 {
        match self {
            Intent::Play | Intent::Pause => 0,
            Intent::Next | Intent::Previous | Intent::Load(_) => 1,
            Intent::Seek(_) => 2,
            Intent::Shuffle(_) => 3,
            Intent::Repeat(_) => 4,
            Intent::Volume(_) => 5,
        }
    }

    /// Stable identifier used by QML for pending indicators.
    pub fn id(&self) -> &'static str {
        match self {
            Intent::Play => "play",
            Intent::Pause => "pause",
            Intent::Next => "next",
            Intent::Previous => "previous",
            Intent::Seek(_) => "seek",
            Intent::Shuffle(_) => "shuffle",
            Intent::Repeat(_) => "repeat",
            Intent::Volume(_) => "volume",
            Intent::Load(_) => "load",
        }
    }

    /// User-facing description used when a request times out.
    pub fn failure_message(&self) -> &'static str {
        match self {
            Intent::Play => "Playback didn't start",
            Intent::Pause => "Couldn't pause playback",
            Intent::Next | Intent::Previous => "Couldn't change track",
            Intent::Seek(_) => "Couldn't seek",
            Intent::Shuffle(_) => "Couldn't change shuffle",
            Intent::Repeat(_) => "Couldn't change repeat",
            Intent::Volume(_) => "Couldn't change volume",
            Intent::Load(_) => "Couldn't start playback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    intent: Intent,
    issued_at_ms: u64,
    /// Track URI at the time of the request, used to detect track changes.
    baseline_track: Option<String>,
}

/// Outcome of applying an update, for the UI to react to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    /// User-facing error caused by this update, if any.
    pub error: Option<String>,
    /// The track changed (artwork/palette should be refreshed).
    pub track_changed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub track: Option<TrackInfo>,
    /// Last confirmed position and when (monotonic ms) it was sampled.
    pub position_ms: u32,
    pub position_sampled_at_ms: u64,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Engine volume, 0..=u16::MAX.
    pub volume: u16,
    /// URI the engine is currently playing (may precede `track` metadata).
    current_uri: Option<String>,
    pending: Vec<Pending>,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a user request. Returns the intent so callers can chain it
    /// into a command.
    pub fn request(&mut self, intent: Intent, now_ms: u64) -> Intent {
        self.pending.retain(|p| p.intent.slot() != intent.slot());
        self.pending.push(Pending {
            intent: intent.clone(),
            issued_at_ms: now_ms,
            baseline_track: self.current_uri.clone(),
        });
        intent
    }

    pub fn pending(&self) -> impl Iterator<Item = &Intent> {
        self.pending.iter().map(|p| &p.intent)
    }

    pub fn is_pending(&self, id: &str) -> bool {
        self.pending.iter().any(|p| p.intent.id() == id)
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The pending transport request (play/pause/track change), if any.
    pub fn pending_transport(&self) -> Option<&Intent> {
        self.pending
            .iter()
            .map(|p| &p.intent)
            .find(|i| matches!(i.slot(), 0 | 1))
    }

    fn resolve(&mut self, pred: impl Fn(&Pending) -> bool) {
        self.pending.retain(|p| !pred(p));
    }

    fn set_position(&mut self, position_ms: u32, now_ms: u64) {
        self.position_ms = position_ms;
        self.position_sampled_at_ms = now_ms;
    }

    fn set_current_uri(&mut self, uri: &str) -> bool {
        if self.current_uri.as_deref() == Some(uri) {
            return false;
        }
        self.current_uri = Some(uri.to_owned());
        // Drop metadata of a different track; never show stale info.
        if self.track.as_ref().is_some_and(|t| t.uri != uri) {
            self.track = None;
        }
        true
    }

    fn resolve_track_change(&mut self) {
        let current = self.current_uri.clone();
        self.resolve(|p| match &p.intent {
            Intent::Next | Intent::Previous => p.baseline_track != current,
            Intent::Load(_) => true,
            _ => false,
        });
    }

    /// Apply a confirmed engine update.
    pub fn apply(&mut self, update: PlayerUpdate, now_ms: u64) -> ApplyOutcome {
        let mut outcome = ApplyOutcome::default();
        match update {
            PlayerUpdate::Loading { uri, position_ms } => {
                outcome.track_changed = self.set_current_uri(&uri);
                self.status = PlaybackStatus::Loading;
                self.set_position(position_ms, now_ms);
                self.resolve_track_change();
            }
            PlayerUpdate::Playing { uri, position_ms } => {
                outcome.track_changed = self.set_current_uri(&uri);
                self.status = PlaybackStatus::Playing;
                self.set_position(position_ms, now_ms);
                self.resolve(|p| matches!(p.intent, Intent::Play | Intent::Seek(_)));
                self.resolve_track_change();
            }
            PlayerUpdate::Paused { uri, position_ms } => {
                outcome.track_changed = self.set_current_uri(&uri);
                self.status = PlaybackStatus::Paused;
                self.set_position(position_ms, now_ms);
                self.resolve(|p| matches!(p.intent, Intent::Pause | Intent::Seek(_)));
                self.resolve_track_change();
            }
            PlayerUpdate::Stopped => {
                self.status = PlaybackStatus::Idle;
                self.set_position(0, now_ms);
                self.current_uri = None;
                self.track = None;
                outcome.track_changed = true;
                // Stopping answers every transport request; none can now succeed.
                self.resolve(|p| matches!(p.intent.slot(), 0..=2));
            }
            PlayerUpdate::Seeked { position_ms } | PlayerUpdate::Position { position_ms } => {
                self.set_position(position_ms, now_ms);
                self.resolve(|p| matches!(p.intent, Intent::Seek(_)));
            }
            PlayerUpdate::TrackChanged(info) => {
                outcome.track_changed = self.track.as_ref().map(|t| &t.uri) != Some(&info.uri);
                self.current_uri = Some(info.uri.clone());
                self.track = Some(info);
                self.resolve_track_change();
            }
            PlayerUpdate::Unavailable { uri } => {
                let had_request = self.pending_transport().is_some();
                self.resolve(|p| matches!(p.intent.slot(), 0 | 1));
                if had_request || self.current_uri.as_deref() == Some(uri.as_str()) {
                    outcome.error = Some("This track isn't available to play".to_owned());
                }
            }
            PlayerUpdate::Volume(v) => {
                self.volume = v;
                self.resolve(|p| matches!(p.intent, Intent::Volume(_)));
            }
            PlayerUpdate::Shuffle(s) => {
                self.shuffle = s;
                self.resolve(|p| matches!(p.intent, Intent::Shuffle(_)));
            }
            PlayerUpdate::Repeat(r) => {
                self.repeat = r;
                self.resolve(|p| matches!(p.intent, Intent::Repeat(_)));
            }
        }
        outcome
    }

    /// Remove and return requests older than `timeout_ms`.
    pub fn expire(&mut self, now_ms: u64, timeout_ms: u64) -> Vec<Intent> {
        let (expired, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| now_ms.saturating_sub(p.issued_at_ms) >= timeout_ms);
        self.pending = kept;
        expired.into_iter().map(|p| p.intent).collect()
    }

    /// Discard pending requests, e.g. after the session was lost.
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Best estimate of the current position. Extrapolates only while the
    /// engine confirmed it is playing and never runs past the track length.
    pub fn position_at(&self, now_ms: u64) -> u32 {
        let base = self.position_ms;
        let pos = if self.status == PlaybackStatus::Playing {
            let elapsed = now_ms.saturating_sub(self.position_sampled_at_ms);
            base.saturating_add(u32::try_from(elapsed).unwrap_or(u32::MAX))
        } else {
            base
        };
        match &self.track {
            Some(t) if t.duration_ms > 0 => pos.min(t.duration_ms),
            _ => pos,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.status == PlaybackStatus::Playing
    }

    /// Whether play/pause/seek are meaningful (a track is loaded locally).
    pub fn has_track(&self) -> bool {
        self.current_uri.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(uri: &str) -> TrackInfo {
        TrackInfo {
            uri: uri.to_owned(),
            title: "Song".into(),
            artists: vec!["Artist".into()],
            album: Some("Album".into()),
            artwork_url: None,
            duration_ms: 200_000,
            explicit: false,
        }
    }

    fn playing(state: &mut PlaybackState, uri: &str, pos: u32, now: u64) {
        state.apply(
            PlayerUpdate::Playing {
                uri: uri.into(),
                position_ms: pos,
            },
            now,
        );
    }

    #[test]
    fn request_does_not_change_confirmed_state() {
        let mut s = PlaybackState::new();
        playing(&mut s, "spotify:track:a", 0, 0);
        s.request(Intent::Pause, 10);
        assert!(s.is_playing(), "pause must wait for confirmation");
        assert!(s.is_pending("pause"));
        s.apply(
            PlayerUpdate::Paused {
                uri: "spotify:track:a".into(),
                position_ms: 1_000,
            },
            20,
        );
        assert_eq!(s.status, PlaybackStatus::Paused);
        assert!(!s.has_pending());
    }

    #[test]
    fn opposite_transport_requests_supersede() {
        let mut s = PlaybackState::new();
        s.request(Intent::Play, 0);
        s.request(Intent::Pause, 1);
        let pending: Vec<_> = s.pending().cloned().collect();
        assert_eq!(pending, vec![Intent::Pause]);
    }

    #[test]
    fn next_resolves_only_on_different_track() {
        let mut s = PlaybackState::new();
        playing(&mut s, "spotify:track:a", 0, 0);
        s.request(Intent::Next, 5);
        // Position correction for the same track must not resolve "next".
        playing(&mut s, "spotify:track:a", 100, 6);
        assert!(s.is_pending("next"));
        s.apply(
            PlayerUpdate::Loading {
                uri: "spotify:track:b".into(),
                position_ms: 0,
            },
            7,
        );
        assert!(!s.is_pending("next"));
        assert_eq!(s.status, PlaybackStatus::Loading);
    }

    #[test]
    fn stale_metadata_is_dropped_on_track_change() {
        let mut s = PlaybackState::new();
        s.apply(PlayerUpdate::TrackChanged(track("spotify:track:a")), 0);
        assert!(s.track.is_some());
        playing(&mut s, "spotify:track:b", 0, 1);
        assert!(s.track.is_none(), "must not show metadata of the old track");
    }

    #[test]
    fn unconfirmed_requests_expire() {
        let mut s = PlaybackState::new();
        s.request(Intent::Play, 0);
        s.request(Intent::Volume(10), 4_000);
        assert!(s.expire(5_000, 8_000).is_empty());
        let expired = s.expire(8_500, 8_000);
        assert_eq!(expired, vec![Intent::Play]);
        assert!(s.is_pending("volume"));
    }

    #[test]
    fn unavailable_reports_error_for_requested_track() {
        let mut s = PlaybackState::new();
        s.request(Intent::Load("spotify:album:x".into()), 0);
        let out = s.apply(
            PlayerUpdate::Unavailable {
                uri: "spotify:track:z".into(),
            },
            1,
        );
        assert!(out.error.is_some());
        assert!(!s.has_pending());
    }

    #[test]
    fn position_extrapolates_only_while_playing() {
        let mut s = PlaybackState::new();
        s.apply(PlayerUpdate::TrackChanged(track("spotify:track:a")), 0);
        playing(&mut s, "spotify:track:a", 1_000, 10_000);
        assert_eq!(s.position_at(12_500), 3_500);
        assert_eq!(s.position_at(10_000_000), 200_000, "clamped to duration");
        s.apply(
            PlayerUpdate::Paused {
                uri: "spotify:track:a".into(),
                position_ms: 4_000,
            },
            13_000,
        );
        assert_eq!(s.position_at(60_000), 4_000);
    }

    #[test]
    fn stopped_clears_track_and_transport_requests() {
        let mut s = PlaybackState::new();
        s.apply(PlayerUpdate::TrackChanged(track("spotify:track:a")), 0);
        playing(&mut s, "spotify:track:a", 0, 0);
        s.request(Intent::Next, 1);
        s.request(Intent::Shuffle(true), 1);
        s.apply(PlayerUpdate::Stopped, 2);
        assert!(!s.has_track());
        assert!(s.track.is_none());
        let pending: Vec<_> = s.pending().cloned().collect();
        assert_eq!(pending, vec![Intent::Shuffle(true)]);
    }

    #[test]
    fn repeat_cycles_like_spotify() {
        assert_eq!(RepeatMode::Off.cycled(), RepeatMode::Context);
        assert_eq!(RepeatMode::Context.cycled(), RepeatMode::Track);
        assert_eq!(RepeatMode::Track.cycled(), RepeatMode::Off);
        assert_eq!(RepeatMode::from_flags(true, true), RepeatMode::Track);
    }
}
