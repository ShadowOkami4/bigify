//! Translate librespot `PlayerEvent`s into Bigify's confirmed `PlayerUpdate`s.

use bigify_core::model::TrackInfo;
use bigify_core::playback::{PlayerUpdate, RepeatMode};
use librespot::core::SpotifyUri;
use librespot::metadata::audio::{AudioItem, UniqueFields};
use librespot::playback::player::PlayerEvent;

fn uri(id: &SpotifyUri) -> Option<String> {
    id.to_uri().ok()
}

/// Largest cover up to 640 px: sharp on a 4K Now Playing screen without
/// downloading the 1000+ px originals.
fn artwork(item: &AudioItem) -> Option<String> {
    let mut covers: Vec<_> = item.covers.iter().filter(|c| !c.url.is_empty()).collect();
    covers.sort_by_key(|c| c.width);
    covers
        .iter()
        .rev()
        .find(|c| c.width <= 640)
        .or_else(|| covers.first())
        .map(|c| c.url.clone())
}

pub fn track_info(item: &AudioItem) -> TrackInfo {
    let (artists, album) = match &item.unique_fields {
        UniqueFields::Track { artists, album, .. } => (
            artists
                .iter()
                .map(|a| a.name.clone())
                .filter(|n| !n.is_empty())
                .collect(),
            Some(album.clone()).filter(|a| !a.is_empty()),
        ),
        UniqueFields::Local { artists, album, .. } => (
            artists.iter().cloned().collect(),
            album.clone().filter(|a| !a.is_empty()),
        ),
        UniqueFields::Episode { show_name, .. } => (
            vec![show_name.clone()]
                .into_iter()
                .filter(|n| !n.is_empty())
                .collect(),
            None,
        ),
    };
    TrackInfo {
        uri: item.uri.clone(),
        title: item.name.clone(),
        artists,
        album,
        artwork_url: artwork(item),
        duration_ms: item.duration_ms,
        explicit: item.is_explicit,
    }
}

/// `None` for events Bigify does not surface (session bookkeeping, preloading…).
pub fn map(event: &PlayerEvent) -> Option<PlayerUpdate> {
    use PlayerEvent as E;
    Some(match event {
        E::Loading {
            track_id,
            position_ms,
            ..
        } => PlayerUpdate::Loading {
            uri: uri(track_id)?,
            position_ms: *position_ms,
        },
        E::Playing {
            track_id,
            position_ms,
            ..
        } => PlayerUpdate::Playing {
            uri: uri(track_id)?,
            position_ms: *position_ms,
        },
        E::Paused {
            track_id,
            position_ms,
            ..
        } => PlayerUpdate::Paused {
            uri: uri(track_id)?,
            position_ms: *position_ms,
        },
        E::Stopped { .. } => PlayerUpdate::Stopped,
        E::Seeked { position_ms, .. } => PlayerUpdate::Seeked {
            position_ms: *position_ms,
        },
        E::PositionCorrection { position_ms, .. } | E::PositionChanged { position_ms, .. } => {
            PlayerUpdate::Position {
                position_ms: *position_ms,
            }
        }
        E::TrackChanged { audio_item } => PlayerUpdate::TrackChanged(track_info(audio_item)),
        E::Unavailable { track_id, .. } => PlayerUpdate::Unavailable {
            uri: uri(track_id).unwrap_or_default(),
        },
        E::VolumeChanged { volume } => PlayerUpdate::Volume(*volume),
        E::ShuffleChanged { shuffle } => PlayerUpdate::Shuffle(*shuffle),
        E::RepeatChanged { context, track } => {
            PlayerUpdate::Repeat(RepeatMode::from_flags(*context, *track))
        }
        _ => return None,
    })
}
