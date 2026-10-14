//! Collection details via librespot's metadata API.
//!
//! Since February 2026 the Web API only returns playlist contents for
//! playlists the user owns or collaborates on. librespot reads playlists the
//! same way Spotify's own clients do, so it is used as the fallback for
//! followed and editorial playlists.

use bigify_core::error::{ErrorKind, ServiceError, ServiceResult};
use bigify_core::model::{join_names, CollectionDetail, MediaItem, MediaKind};
use futures_util::stream::{self, StreamExt};
use librespot::core::session::Session;
use librespot::core::SpotifyUri;
use librespot::metadata::{Metadata, Playlist, Track};

use crate::errors::from_librespot;

/// Tracks resolved per request; longer playlists are marked truncated.
pub const MAX_TRACKS: usize = 100;

fn image_url(track: &Track) -> Option<String> {
    let mut covers: Vec<_> = track.album.covers.iter().collect();
    covers.sort_by_key(|c| c.width);
    let pick = covers
        .iter()
        .find(|c| c.width >= 300)
        .or_else(|| covers.last())?;
    pick.id
        .to_base16()
        .ok()
        .map(|hex| format!("https://i.scdn.co/image/{hex}"))
}

fn track_item(track: &Track, context_uri: &str) -> Option<MediaItem> {
    let uri = track.id.to_uri().ok()?;
    if track.name.is_empty() {
        return None;
    }
    let mut item = MediaItem::new(uri, MediaKind::Track, track.name.clone());
    item.subtitle = join_names(track.artists.iter().map(|a| a.name.as_str()));
    item.artwork_url = image_url(track);
    item.duration_ms = u32::try_from(track.duration).ok();
    item.explicit = track.is_explicit;
    item.context_uri = Some(context_uri.to_owned());
    Some(item)
}

pub async fn playlist(session: &Session, uri: &str) -> ServiceResult<CollectionDetail> {
    let id = SpotifyUri::from_uri(uri)
        .map_err(|_| ServiceError::new(ErrorKind::NotFound, "Invalid playlist"))?;
    let playlist = Playlist::get(session, &id).await.map_err(from_librespot)?;
    let ids: Vec<SpotifyUri> = playlist
        .contents
        .items
        .iter()
        .map(|i| i.id.clone())
        .filter(|id| matches!(id, SpotifyUri::Track { .. }))
        .take(MAX_TRACKS)
        .collect();
    let total = playlist.length.max(0) as u32;

    let tracks: Vec<MediaItem> = stream::iter(ids)
        .map(|id| async move { Track::get(session, &id).await })
        .buffered(8)
        .filter_map(|r| async move {
            r.map_err(|e| log::debug!("playlist track skipped: {e}"))
                .ok()
        })
        .filter_map(|t| async move { track_item(&t, uri) })
        .collect()
        .await;

    let name = playlist.attributes.name.trim().to_owned();
    let mut item = MediaItem::new(
        uri,
        MediaKind::Playlist,
        if name.is_empty() {
            "Playlist".into()
        } else {
            name
        },
    );
    item.subtitle = "Playlist".into();
    item.artwork_url = tracks.iter().find_map(|t| t.artwork_url.clone());
    Ok(CollectionDetail {
        description: if total == 1 {
            "1 song".into()
        } else {
            format!("{total} songs")
        },
        truncated: (total as usize) > tracks.len(),
        total_tracks: Some(total),
        item,
        tracks,
    })
}
