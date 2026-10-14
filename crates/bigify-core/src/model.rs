//! Media types shared by the service and the UI.
//!
//! Everything here is plain data with `Serialize`, so the UI bridge can hand
//! it to QML as JSON. Optional fields stay optional: the UI must render a
//! sensible fallback rather than receive invented values.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Track,
    Album,
    Artist,
    Playlist,
    Episode,
    Show,
    /// The user's Liked Songs collection.
    Collection,
}

impl MediaKind {
    /// Derive the kind from a Spotify URI such as `spotify:album:…`.
    pub fn from_uri(uri: &str) -> Option<Self> {
        let mut parts = uri.split(':');
        if parts.next()? != "spotify" {
            return None;
        }
        match parts.next()? {
            "track" => Some(MediaKind::Track),
            "album" => Some(MediaKind::Album),
            "artist" => Some(MediaKind::Artist),
            "playlist" => Some(MediaKind::Playlist),
            "episode" => Some(MediaKind::Episode),
            "show" => Some(MediaKind::Show),
            "user" if uri.ends_with(":collection") => Some(MediaKind::Collection),
            _ => None,
        }
    }

    /// Whether the item can be handed to Spotify Connect as a playback context.
    pub fn is_playable_context(self) -> bool {
        matches!(
            self,
            MediaKind::Album
                | MediaKind::Artist
                | MediaKind::Playlist
                | MediaKind::Show
                | MediaKind::Collection
        )
    }
}

/// A browsable or playable item (card, search result, library entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub uri: String,
    pub kind: MediaKind,
    pub title: String,
    /// Secondary line, e.g. artist names or playlist owner. Empty if unknown.
    pub subtitle: String,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<u32>,
    pub explicit: bool,
    /// For tracks: the album or playlist the track belongs to.
    pub context_uri: Option<String>,
}

impl MediaItem {
    pub fn new(uri: impl Into<String>, kind: MediaKind, title: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            kind,
            title: title.into(),
            subtitle: String::new(),
            artwork_url: None,
            duration_ms: None,
            explicit: false,
            context_uri: None,
        }
    }
}

/// A titled row on the Home screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    pub title: String,
    /// "cards" for artwork shelves, "tracks" for track lists.
    pub layout: SectionLayout,
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionLayout {
    Cards,
    Tracks,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub query: String,
    pub tracks: Vec<MediaItem>,
    pub albums: Vec<MediaItem>,
    pub artists: Vec<MediaItem>,
    pub playlists: Vec<MediaItem>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
    }
}

/// One page of a paged library listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub offset: u32,
    /// Total number of items reported by Spotify, if known.
    pub total: Option<u32>,
    pub items: Vec<MediaItem>,
}

impl Page {
    pub fn next_offset(&self) -> Option<u32> {
        let next = self.offset + self.items.len() as u32;
        match self.total {
            Some(total) if next < total && !self.items.is_empty() => Some(next),
            _ => None,
        }
    }
}

/// Album or playlist detail screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDetail {
    pub item: MediaItem,
    /// Extra header line such as release year or track count.
    pub description: String,
    pub tracks: Vec<MediaItem>,
    /// True when only the first part of a long collection was loaded.
    pub truncated: bool,
    pub total_tracks: Option<u32>,
}

/// The current track as reported by the playback engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub uri: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub duration_ms: u32,
    pub explicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSnapshot {
    pub current: Option<MediaItem>,
    pub upcoming: Vec<MediaItem>,
}

/// Join artist names for a single display line.
pub fn join_names<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names.into_iter().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_from_uri() {
        assert_eq!(
            MediaKind::from_uri("spotify:album:abc"),
            Some(MediaKind::Album)
        );
        assert_eq!(
            MediaKind::from_uri("spotify:user:me:collection"),
            Some(MediaKind::Collection)
        );
        assert_eq!(MediaKind::from_uri("spotify:user:me"), None);
        assert_eq!(MediaKind::from_uri("https://open.spotify.com"), None);
        assert!(!MediaKind::Track.is_playable_context());
        assert!(MediaKind::Playlist.is_playable_context());
    }

    #[test]
    fn page_next_offset() {
        let item = MediaItem::new("spotify:album:a", MediaKind::Album, "A");
        let page = Page {
            offset: 0,
            total: Some(3),
            items: vec![item.clone(), item.clone()],
        };
        assert_eq!(page.next_offset(), Some(2));
        let last = Page {
            offset: 2,
            total: Some(3),
            items: vec![item],
        };
        assert_eq!(last.next_offset(), None);
        let empty = Page {
            offset: 5,
            total: Some(10),
            items: vec![],
        };
        assert_eq!(empty.next_offset(), None, "empty page must not loop");
    }
}
