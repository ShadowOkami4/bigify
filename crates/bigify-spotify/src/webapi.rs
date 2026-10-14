//! Spotify Web API access for browse data (home, search, library, queue).
//!
//! Requests are authorised with an access token obtained from the librespot
//! session (`TokenProvider`). See `docs/LIMITATIONS.md` for why, and for the
//! endpoints Spotify restricts. All JSON is parsed defensively: unknown fields
//! are ignored, missing optional fields stay `None`, and items that lack a
//! URI or name are skipped instead of being shown half-empty.

use std::collections::HashSet;

use bigify_core::error::{ErrorKind, ServiceError, ServiceResult};
use bigify_core::model::{
    join_names, CollectionDetail, MediaItem, MediaKind, Page, QueueSnapshot, SearchResults,
    Section, SectionLayout,
};
use bytes::Bytes;
use librespot::core::session::Session;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use std::sync::Arc;

use crate::errors::from_librespot;
use crate::userauth::UserAuth;

const API: &str = "https://api.spotify.com/v1";

/// Scopes requested for Web API tokens.
pub const WEB_API_SCOPES: &str = "user-read-private,user-library-read,user-library-modify,\
user-read-recently-played,user-top-read,user-follow-read,playlist-read-private,\
playlist-read-collaborative,user-read-playback-state,user-modify-playback-state,\
user-follow-modify,playlist-modify-public,playlist-modify-private";

/// Spotify's maximum page size for library endpoints.
pub const PAGE_SIZE: u32 = 50;
/// Search `limit` cap for apps in Development Mode since February 2026.
pub const SEARCH_LIMIT: u32 = 10;

// ---------------------------------------------------------------------------
// Wire types (only the fields Bigify uses)

#[derive(Debug, Deserialize, Clone)]
pub struct Image {
    pub url: String,
    pub width: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArtistRef {
    pub name: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AlbumRef {
    pub uri: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<Image>>,
    #[serde(default)]
    pub artists: Vec<ArtistRef>,
    pub release_date: Option<String>,
    pub total_tracks: Option<u32>,
    pub album_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Track {
    pub uri: Option<String>,
    pub name: Option<String>,
    pub duration_ms: Option<u32>,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
    /// Set for episodes in mixed playlists/queues.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub images: Option<Vec<Image>>,
    pub show: Option<NamedRef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NamedRef {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Artist {
    pub uri: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<Image>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Owner {
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlaylistItemEntry {
    /// `track` was renamed to `item` in February 2026. Responses may carry
    /// both, so they are separate fields (a serde alias would reject that
    /// as a duplicate field).
    track: Option<Track>,
    item: Option<Track>,
}

impl PlaylistItemEntry {
    pub fn track(&self) -> Option<&Track> {
        self.item.as_ref().or(self.track.as_ref())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Playlist {
    pub uri: Option<String>,
    pub name: Option<String>,
    pub owner: Option<Owner>,
    #[serde(default)]
    pub images: Option<Vec<Image>>,
    pub description: Option<String>,
    /// `tracks` was renamed to `items` in February 2026; responses may carry
    /// both (see [`Playlist::contents`]).
    tracks: Option<PlaylistTracks>,
    items: Option<PlaylistTracks>,
}

impl Playlist {
    /// The playlist's item paging object, preferring whichever field
    /// actually carries entries, then whichever reports a total.
    pub fn contents(&self) -> Option<&PlaylistTracks> {
        let candidates = [self.items.as_ref(), self.tracks.as_ref()];
        candidates
            .iter()
            .flatten()
            .find(|c| !c.items.is_empty())
            .or_else(|| candidates.iter().flatten().find(|c| c.total.is_some()))
            .or_else(|| candidates.iter().flatten().next())
            .copied()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlaylistTracks {
    pub total: Option<u32>,
    #[serde(default)]
    pub items: Vec<PlaylistItemEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Paging<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<T>,
    pub total: Option<u32>,
    pub offset: Option<u32>,
    pub cursors: Option<Cursors>,
}

#[derive(Debug, Deserialize)]
pub struct Cursors {
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SavedTrack {
    track: Option<Track>,
}

#[derive(Debug, Deserialize)]
struct SavedAlbum {
    album: Option<AlbumRef>,
}

#[derive(Debug, Deserialize)]
struct PlayHistory {
    track: Option<Track>,
    context: Option<ContextRef>,
}

#[derive(Debug, Deserialize)]
struct ContextRef {
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FollowedArtists {
    artists: Paging<Artist>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    tracks: Option<Paging<Option<Track>>>,
    albums: Option<Paging<Option<AlbumRef>>>,
    artists: Option<Paging<Option<Artist>>>,
    playlists: Option<Paging<Option<Playlist>>>,
}

#[derive(Debug, Deserialize)]
struct QueueResponse {
    currently_playing: Option<Track>,
    #[serde(default)]
    queue: Vec<Track>,
}

#[derive(Debug, Deserialize)]
struct FullAlbum {
    #[serde(flatten)]
    album: AlbumRef,
    tracks: Option<Paging<Track>>,
}

// ---------------------------------------------------------------------------
// Conversion to UI model

/// Pick the smallest image that is still at least `min` px wide (TV cards are
/// ~300 px at 1080p, ~600 px at 4K), else the largest available.
pub fn best_image(images: &[Image], min: u32) -> Option<String> {
    let mut sorted: Vec<&Image> = images.iter().collect();
    sorted.sort_by_key(|i| i.width.unwrap_or(0));
    sorted
        .iter()
        .find(|i| i.width.unwrap_or(0) >= min)
        .or_else(|| sorted.last())
        .map(|i| i.url.clone())
}

const CARD_IMAGE_MIN: u32 = 300;

fn names(artists: &[ArtistRef]) -> String {
    join_names(artists.iter().filter_map(|a| a.name.as_deref()))
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.trim().is_empty())
}

pub fn track_item(t: &Track, context_uri: Option<&str>) -> Option<MediaItem> {
    let uri = non_empty(t.uri.clone())?;
    let title = non_empty(t.name.clone())?;
    let is_episode = t.kind.as_deref() == Some("episode");
    let kind = if is_episode {
        MediaKind::Episode
    } else {
        MediaKind::Track
    };
    let mut item = MediaItem::new(uri, kind, title);
    item.subtitle = if is_episode {
        t.show
            .as_ref()
            .and_then(|s| s.name.clone())
            .unwrap_or_default()
    } else {
        names(&t.artists)
    };
    let images = t
        .album
        .as_ref()
        .and_then(|a| a.images.as_deref())
        .or(t.images.as_deref())
        .unwrap_or(&[]);
    item.artwork_url = best_image(images, CARD_IMAGE_MIN);
    item.duration_ms = t.duration_ms;
    item.explicit = t.explicit;
    item.context_uri = context_uri
        .map(str::to_owned)
        .or_else(|| t.album.as_ref().and_then(|a| a.uri.clone()));
    Some(item)
}

pub fn album_item(a: &AlbumRef) -> Option<MediaItem> {
    let mut item = MediaItem::new(
        non_empty(a.uri.clone())?,
        MediaKind::Album,
        non_empty(a.name.clone())?,
    );
    item.subtitle = names(&a.artists);
    item.artwork_url = best_image(a.images.as_deref().unwrap_or(&[]), CARD_IMAGE_MIN);
    Some(item)
}

pub fn artist_item(a: &Artist) -> Option<MediaItem> {
    let mut item = MediaItem::new(
        non_empty(a.uri.clone())?,
        MediaKind::Artist,
        non_empty(a.name.clone())?,
    );
    item.subtitle = "Artist".into();
    item.artwork_url = best_image(a.images.as_deref().unwrap_or(&[]), CARD_IMAGE_MIN);
    Some(item)
}

pub fn playlist_item(p: &Playlist) -> Option<MediaItem> {
    let mut item = MediaItem::new(
        non_empty(p.uri.clone())?,
        MediaKind::Playlist,
        non_empty(p.name.clone())?,
    );
    item.subtitle = p
        .owner
        .as_ref()
        .and_then(|o| non_empty(o.display_name.clone()))
        .map(|o| format!("Playlist · {o}"))
        .unwrap_or_else(|| "Playlist".into());
    item.artwork_url = best_image(p.images.as_deref().unwrap_or(&[]), CARD_IMAGE_MIN);
    Some(item)
}

fn dedup(items: Vec<MediaItem>) -> Vec<MediaItem> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|i| seen.insert(i.uri.clone()))
        .collect()
}

pub fn parse<T: DeserializeOwned>(body: &[u8]) -> ServiceResult<T> {
    serde_json::from_slice(body)
        .map_err(|e| ServiceError::internal(format!("Unexpected response from Spotify ({e})")))
}

pub fn parse_search(query: &str, body: &[u8]) -> ServiceResult<SearchResults> {
    let r: SearchResponse = parse(body)?;
    fn collect<T>(
        p: Option<Paging<Option<T>>>,
        f: impl Fn(&T) -> Option<MediaItem>,
    ) -> Vec<MediaItem> {
        p.map(|p| p.items.iter().flatten().filter_map(&f).collect())
            .unwrap_or_default()
    }
    Ok(SearchResults {
        query: query.to_owned(),
        tracks: collect(r.tracks, |t| track_item(t, None)),
        albums: collect(r.albums, album_item),
        artists: collect(r.artists, artist_item),
        playlists: collect(r.playlists, playlist_item),
    })
}

pub fn parse_saved_tracks(body: &[u8], offset: u32) -> ServiceResult<Page> {
    let p: Paging<SavedTrack> = parse(body)?;
    Ok(Page {
        offset: p.offset.unwrap_or(offset),
        total: p.total,
        items: p
            .items
            .iter()
            .filter_map(|s| s.track.as_ref())
            .filter_map(|t| track_item(t, None))
            .collect(),
    })
}

pub fn parse_saved_albums(body: &[u8], offset: u32) -> ServiceResult<Page> {
    let p: Paging<SavedAlbum> = parse(body)?;
    Ok(Page {
        offset: p.offset.unwrap_or(offset),
        total: p.total,
        items: p
            .items
            .iter()
            .filter_map(|s| s.album.as_ref())
            .filter_map(album_item)
            .collect(),
    })
}

pub fn parse_playlists(body: &[u8], offset: u32) -> ServiceResult<Page> {
    let p: Paging<Option<Playlist>> = parse(body)?;
    Ok(Page {
        offset: p.offset.unwrap_or(offset),
        total: p.total,
        items: p.items.iter().flatten().filter_map(playlist_item).collect(),
    })
}

/// Followed artists use cursor paging; returns the page and the next cursor.
pub fn parse_followed_artists(body: &[u8], offset: u32) -> ServiceResult<(Page, Option<String>)> {
    let f: FollowedArtists = parse(body)?;
    let after = f.artists.cursors.and_then(|c| c.after);
    Ok((
        Page {
            offset,
            total: f.artists.total,
            items: f.artists.items.iter().filter_map(artist_item).collect(),
        },
        after,
    ))
}

pub fn parse_top_artists(body: &[u8]) -> ServiceResult<Vec<MediaItem>> {
    let p: Paging<Artist> = parse(body)?;
    Ok(p.items.iter().filter_map(artist_item).collect())
}

/// Recently played: (unique tracks, unique context URIs in order).
pub fn parse_recently_played(body: &[u8]) -> ServiceResult<(Vec<MediaItem>, Vec<ContextHint>)> {
    let p: Paging<PlayHistory> = parse(body)?;
    let mut tracks = Vec::new();
    let mut contexts: Vec<ContextHint> = Vec::new();
    for h in &p.items {
        let Some(t) = &h.track else { continue };
        let ctx = h.context.as_ref().and_then(|c| c.uri.clone());
        if let Some(item) = track_item(t, ctx.as_deref()) {
            tracks.push(item);
        }
        let Some(ctx) = ctx else { continue };
        if contexts.iter().any(|c| c.uri == ctx) {
            continue;
        }
        // Album contexts can be described from the track itself.
        let resolved = t
            .album
            .as_ref()
            .filter(|a| a.uri.as_deref() == Some(ctx.as_str()))
            .and_then(album_item);
        contexts.push(ContextHint { uri: ctx, resolved });
    }
    Ok((dedup(tracks), contexts))
}

/// A recently played context, possibly already described.
#[derive(Debug, Clone)]
pub struct ContextHint {
    pub uri: String,
    pub resolved: Option<MediaItem>,
}

pub fn parse_album_detail(body: &[u8]) -> ServiceResult<CollectionDetail> {
    let a: FullAlbum = parse(body)?;
    let item =
        album_item(&a.album).ok_or_else(|| ServiceError::internal("Album is missing its name"))?;
    let artwork = item.artwork_url.clone();
    let tracks: Vec<MediaItem> = a
        .tracks
        .as_ref()
        .map(|p| {
            p.items
                .iter()
                .filter_map(|t| track_item(t, Some(&item.uri)))
                .map(|mut t| {
                    // Album track listings omit per-track artwork.
                    t.artwork_url = t.artwork_url.or_else(|| artwork.clone());
                    t
                })
                .collect()
        })
        .unwrap_or_default();
    let total = a
        .album
        .total_tracks
        .or(a.tracks.as_ref().and_then(|p| p.total));
    let year = a
        .album
        .release_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .map(str::to_owned);
    let kind = a.album.album_type.as_deref().map(capitalise);
    let description = [kind, year, total.map(|n| plural(n, "song"))]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(CollectionDetail {
        truncated: total.is_some_and(|n| n as usize > tracks.len()),
        total_tracks: total,
        item,
        description,
        tracks,
    })
}

/// Parse a playlist. `tracks` is empty when Spotify withheld the contents
/// (the caller may then fall back to librespot metadata).
pub fn parse_playlist_detail(body: &[u8]) -> ServiceResult<CollectionDetail> {
    let p: Playlist = parse(body)?;
    let item =
        playlist_item(&p).ok_or_else(|| ServiceError::internal("Playlist is missing its name"))?;
    let entries = p.contents().map(|t| t.items.as_slice()).unwrap_or(&[]);
    let tracks: Vec<MediaItem> = entries
        .iter()
        .filter_map(|e| e.track())
        .filter_map(|t| track_item(t, Some(&item.uri)))
        .collect();
    let total = p.contents().and_then(|t| t.total);
    let description = non_empty(p.description.clone())
        .map(|d| strip_html(&d))
        .or_else(|| total.map(|n| plural(n, "song")))
        .unwrap_or_default();
    Ok(CollectionDetail {
        truncated: total.is_some_and(|n| n as usize > tracks.len()),
        total_tracks: total,
        item,
        description,
        tracks,
    })
}

/// Split an artist's releases into (albums incl. compilations, singles/EPs).
/// Subtitles show the release year rather than repeating the artist's name.
pub fn parse_artist_albums(body: &[u8]) -> ServiceResult<(Vec<MediaItem>, Vec<MediaItem>)> {
    let p: Paging<Option<AlbumRef>> = parse(body)?;
    let mut albums = Vec::new();
    let mut singles = Vec::new();
    for a in p.items.iter().flatten() {
        let Some(mut item) = album_item(a) else {
            continue;
        };
        item.subtitle = a
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("")
            .to_owned();
        match a.album_type.as_deref() {
            Some("single") => singles.push(item),
            _ => albums.push(item),
        }
    }
    Ok((dedup(albums), dedup(singles)))
}

pub fn parse_queue(body: &[u8]) -> ServiceResult<QueueSnapshot> {
    let q: QueueResponse = parse(body)?;
    Ok(QueueSnapshot {
        current: q
            .currently_playing
            .as_ref()
            .and_then(|t| track_item(t, None)),
        upcoming: q.queue.iter().filter_map(|t| track_item(t, None)).collect(),
    })
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

fn plural(n: u32, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// Playlist descriptions may contain links and HTML entities.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// Percent-encode a query-string value (RFC 3986 unreserved set kept).
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Maximum URIs per `/me/library` request.
pub const LIBRARY_BATCH: usize = 40;

/// Kinds that can be saved or followed.
pub fn library_kind(uri: &str) -> Option<MediaKind> {
    MediaKind::from_uri(uri).filter(|k| {
        matches!(
            k,
            MediaKind::Track
                | MediaKind::Album
                | MediaKind::Artist
                | MediaKind::Playlist
                | MediaKind::Episode
                | MediaKind::Show
        )
    })
}

/// Errors after which the legacy type-specific endpoint is worth trying.
fn is_fallback(e: &ServiceError) -> bool {
    matches!(e.kind, ErrorKind::NotFound | ErrorKind::Unsupported)
}

/// Extract the base62 id from `spotify:<type>:<id>`.
pub fn uri_id(uri: &str) -> Option<&str> {
    let id = uri.rsplit(':').next()?;
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())).then_some(id)
}

// ---------------------------------------------------------------------------
// HTTP

/// Where Web API access tokens come from (see [`WebApi::token`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenSource {
    Login5,
    Keymaster,
}

/// Process-wide preference, so a source that worked is tried first.
static PREFER_KEYMASTER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl TokenSource {
    fn preferred() -> Self {
        if PREFER_KEYMASTER.load(std::sync::atomic::Ordering::Relaxed) {
            TokenSource::Keymaster
        } else {
            TokenSource::Login5
        }
    }

    fn set_preferred(source: Self) {
        PREFER_KEYMASTER.store(
            source == TokenSource::Keymaster,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn other(self) -> Self {
        match self {
            TokenSource::Login5 => TokenSource::Keymaster,
            TokenSource::Keymaster => TokenSource::Login5,
        }
    }
}

/// Which step of a request failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Token,
    Http,
}

/// Thin client bound to one librespot session.
#[derive(Clone)]
pub struct WebApi {
    session: Session,
    /// The user's own Spotify app, when set up (see [`crate::userauth`]).
    user: Option<Arc<UserAuth>>,
}

/// Shown when Spotify throttles the built-in access and no app is set up.
const RATE_LIMITED_HINT: &str = "Spotify is limiting Bigify's built-in library access. \
Set up library access in Settings (one-time, about 3 minutes) to fix this.";

impl WebApi {
    pub fn new(session: Session, user: Option<Arc<UserAuth>>) -> Self {
        Self { session, user }
    }

    /// Obtain an access token from `source`.
    ///
    /// * `Login5`: the token librespot itself uses for Spotify's HTTP APIs,
    ///   minted from the session's stored credentials.
    /// * `Keymaster`: the older Mercury `hm://keymaster` endpoint, which
    ///   Spotify is phasing out; kept as a fallback.
    async fn token(&self, source: TokenSource) -> ServiceResult<String> {
        let result = match source {
            TokenSource::Login5 => self.session.login5().auth_token().await,
            TokenSource::Keymaster => {
                self.session
                    .token_provider()
                    .get_token(WEB_API_SCOPES)
                    .await
            }
        };
        result.map(|t| t.access_token).map_err(|e| {
            // The error text never contains the token itself.
            log::warn!("Web API token via {source:?} failed: {e}");
            ServiceError::new(
                ErrorKind::Network,
                "Couldn't get access to your Spotify library. Playback still works; try again later.",
            )
        })
    }

    async fn request(&self, method: http::Method, path_and_query: &str) -> ServiceResult<Bytes> {
        // The user's own app has its own quota and is used exclusively when
        // configured; built-in tokens are only a fallback if it breaks.
        if let Some(user) = &self.user {
            match user.access_token(&self.session).await {
                Some(Ok(token)) => {
                    return self.send_with_token(&token, method, path_and_query).await;
                }
                Some(Err(e)) => log::warn!("library token unavailable, using built-in access: {e}"),
                None => {}
            }
        }
        self.request_builtin(method, path_and_query)
            .await
            .map_err(|e| {
                if e.kind == ErrorKind::RateLimited {
                    ServiceError::new(ErrorKind::RateLimited, RATE_LIMITED_HINT)
                } else {
                    e
                }
            })
    }

    /// Built-in access: tokens minted from the librespot session.
    async fn request_builtin(
        &self,
        method: http::Method,
        path_and_query: &str,
    ) -> ServiceResult<Bytes> {
        let preferred = TokenSource::preferred();
        let first = self
            .request_with(preferred, method.clone(), path_and_query)
            .await;
        match first {
            // Token unavailable or rejected: try the other source once and
            // remember it if it works.
            Err((stage, e))
                if stage == Stage::Token
                    || matches!(e.kind, ErrorKind::Unauthorized | ErrorKind::Unsupported) =>
            {
                let other = preferred.other();
                match self.request_with(other, method, path_and_query).await {
                    Ok(body) => {
                        log::info!("Web API: using {other:?} tokens");
                        TokenSource::set_preferred(other);
                        Ok(body)
                    }
                    // Report the first error unless the retry got further.
                    Err((Stage::Http, e2)) => Err(e2),
                    Err(_) => Err(e),
                }
            }
            Err((_, e)) => Err(e),
            Ok(body) => Ok(body),
        }
    }

    async fn request_with(
        &self,
        source: TokenSource,
        method: http::Method,
        path_and_query: &str,
    ) -> Result<Bytes, (Stage, ServiceError)> {
        let token = self.token(source).await.map_err(|e| (Stage::Token, e))?;
        self.send_with_token(&token, method, path_and_query)
            .await
            .map_err(|e| (Stage::Http, e))
    }

    async fn send_with_token(
        &self,
        token: &str,
        method: http::Method,
        path_and_query: &str,
    ) -> ServiceResult<Bytes> {
        let req = http::Request::builder()
            .method(method)
            .uri(format!("{API}{path_and_query}"))
            .header(http::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(http::header::ACCEPT, "application/json")
            .header(http::header::CONTENT_LENGTH, "0")
            .body(Bytes::new())
            .map_err(|e| ServiceError::internal(format!("Invalid request ({e})")))?;
        // librespot's client already honours short 429 Retry-After values.
        self.session
            .http_client()
            .request_body(req)
            .await
            .map_err(|e| {
                // Log the endpoint without its query (search terms are personal).
                let path = path_and_query.split('?').next().unwrap_or("");
                log::warn!("Web API {path} failed: {e}");
                from_librespot(e)
            })
    }

    pub async fn get(&self, path_and_query: &str) -> ServiceResult<Bytes> {
        self.request(http::Method::GET, path_and_query).await
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path_and_query: &str) -> ServiceResult<T> {
        parse(&self.get(path_and_query).await?)
    }

    pub async fn post(&self, path_and_query: &str) -> ServiceResult<()> {
        self.request(http::Method::POST, path_and_query)
            .await
            .map(|_| ())
    }

    async fn send(&self, method: http::Method, path_and_query: &str) -> ServiceResult<()> {
        self.request(method, path_and_query).await.map(|_| ())
    }

    // -- Endpoints -----------------------------------------------------------

    pub async fn search(&self, query: &str) -> ServiceResult<SearchResults> {
        let body = self
            .get(&format!(
                "/search?q={}&type=track,album,artist,playlist&limit={SEARCH_LIMIT}",
                encode(query)
            ))
            .await?;
        parse_search(query, &body)
    }

    pub async fn saved_tracks(&self, offset: u32) -> ServiceResult<Page> {
        let body = self
            .get(&format!("/me/tracks?limit={PAGE_SIZE}&offset={offset}"))
            .await?;
        parse_saved_tracks(&body, offset)
    }

    pub async fn saved_albums(&self, offset: u32) -> ServiceResult<Page> {
        let body = self
            .get(&format!("/me/albums?limit={PAGE_SIZE}&offset={offset}"))
            .await?;
        parse_saved_albums(&body, offset)
    }

    pub async fn playlists(&self, offset: u32) -> ServiceResult<Page> {
        let body = self
            .get(&format!("/me/playlists?limit={PAGE_SIZE}&offset={offset}"))
            .await?;
        parse_playlists(&body, offset)
    }

    pub async fn followed_artists(
        &self,
        offset: u32,
        after: Option<&str>,
    ) -> ServiceResult<(Page, Option<String>)> {
        let mut path = format!("/me/following?type=artist&limit={PAGE_SIZE}");
        if let Some(after) = after {
            path.push_str(&format!("&after={}", encode(after)));
        }
        let body = self.get(&path).await?;
        parse_followed_artists(&body, offset)
    }

    pub async fn album(&self, uri: &str) -> ServiceResult<CollectionDetail> {
        let id =
            uri_id(uri).ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid album"))?;
        parse_album_detail(&self.get(&format!("/albums/{id}")).await?)
    }

    pub async fn playlist(&self, uri: &str) -> ServiceResult<CollectionDetail> {
        let id = uri_id(uri)
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid playlist"))?;
        parse_playlist_detail(&self.get(&format!("/playlists/{id}")).await?)
    }

    pub async fn playlist_summary(&self, uri: &str) -> ServiceResult<MediaItem> {
        let id = uri_id(uri)
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid playlist"))?;
        let p: Playlist = self
            .get_json(&format!(
                "/playlists/{id}?fields=uri,name,owner(display_name),images"
            ))
            .await?;
        playlist_item(&p).ok_or_else(|| ServiceError::internal("Playlist is missing its name"))
    }

    pub async fn artist_summary(&self, uri: &str) -> ServiceResult<MediaItem> {
        let id =
            uri_id(uri).ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid artist"))?;
        let a: Artist = self.get_json(&format!("/artists/{id}")).await?;
        artist_item(&a).ok_or_else(|| ServiceError::internal("Artist is missing its name"))
    }

    pub async fn artist_albums(
        &self,
        uri: &str,
    ) -> ServiceResult<(Vec<MediaItem>, Vec<MediaItem>)> {
        let id =
            uri_id(uri).ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid artist"))?;
        let body = self
            .get(&format!(
                "/artists/{id}/albums?include_groups=album,single,compilation&limit={PAGE_SIZE}"
            ))
            .await?;
        parse_artist_albums(&body)
    }

    /// Whether each URI is saved (followed, for artists).
    ///
    /// Uses the generic `/me/library/contains` endpoint introduced in
    /// February 2026 and falls back to the type-specific endpoints when it
    /// is unavailable to this token. Artists always use `/me/following`.
    pub async fn contains(&self, uris: &[String]) -> ServiceResult<Vec<(String, bool)>> {
        let mut out = Vec::new();
        let (artists, others): (Vec<&String>, Vec<&String>) = uris
            .iter()
            .filter(|u| library_kind(u).is_some())
            .partition(|u| library_kind(u) == Some(MediaKind::Artist));

        for chunk in others.chunks(LIBRARY_BATCH) {
            let joined: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();
            let generic: ServiceResult<Vec<bool>> = match self
                .get(&format!(
                    "/me/library/contains?uris={}",
                    encode(&joined.join(","))
                ))
                .await
            {
                Ok(body) => parse(&body),
                Err(e) => Err(e),
            };
            match generic {
                Ok(flags) if flags.len() == chunk.len() => {
                    out.extend(chunk.iter().map(|u| (*u).clone()).zip(flags));
                }
                Ok(_) => return Err(ServiceError::internal("Unexpected library response")),
                Err(e) if is_fallback(&e) => {
                    for uri in chunk {
                        out.push(((*uri).clone(), self.contains_legacy(uri).await?));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        for uri in artists {
            out.push((uri.clone(), self.contains_legacy(uri).await?));
        }
        Ok(out)
    }

    async fn contains_legacy(&self, uri: &str) -> ServiceResult<bool> {
        let id =
            uri_id(uri).ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid item"))?;
        let path = match library_kind(uri) {
            Some(MediaKind::Track) => format!("/me/tracks/contains?ids={id}"),
            Some(MediaKind::Album) => format!("/me/albums/contains?ids={id}"),
            Some(MediaKind::Episode) => format!("/me/episodes/contains?ids={id}"),
            Some(MediaKind::Show) => format!("/me/shows/contains?ids={id}"),
            Some(MediaKind::Artist) => format!("/me/following/contains?type=artist&ids={id}"),
            Some(MediaKind::Playlist) => format!("/playlists/{id}/followers/contains"),
            _ => return Err(ServiceError::unsupported("This item can't be saved")),
        };
        let flags: Vec<bool> = self.get_json(&path).await?;
        Ok(flags.first().copied().unwrap_or(false))
    }

    /// Save or remove one item (follow/unfollow for artists).
    pub async fn set_saved(&self, uri: &str, saved: bool) -> ServiceResult<()> {
        let method = if saved {
            http::Method::PUT
        } else {
            http::Method::DELETE
        };
        if library_kind(uri) != Some(MediaKind::Artist) {
            match self
                .send(method.clone(), &format!("/me/library?uris={}", encode(uri)))
                .await
            {
                Err(e) if is_fallback(&e) => {}
                other => return other,
            }
        }
        let id =
            uri_id(uri).ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "Invalid item"))?;
        let path = match library_kind(uri) {
            Some(MediaKind::Track) => format!("/me/tracks?ids={id}"),
            Some(MediaKind::Album) => format!("/me/albums?ids={id}"),
            Some(MediaKind::Episode) => format!("/me/episodes?ids={id}"),
            Some(MediaKind::Show) => format!("/me/shows?ids={id}"),
            Some(MediaKind::Artist) => format!("/me/following?type=artist&ids={id}"),
            Some(MediaKind::Playlist) => format!("/playlists/{id}/followers"),
            _ => return Err(ServiceError::unsupported("This item can't be saved")),
        };
        self.send(method, &path).await
    }

    pub async fn queue(&self) -> ServiceResult<QueueSnapshot> {
        parse_queue(&self.get("/me/player/queue").await?)
    }

    pub async fn add_to_queue(&self, uri: &str) -> ServiceResult<()> {
        self.post(&format!("/me/player/queue?uri={}", encode(uri)))
            .await
    }

    /// Build the Home screen from the personal data Spotify still exposes.
    /// Each shelf is fetched independently; a failing shelf is omitted rather
    /// than failing the whole screen.
    pub async fn home(&self) -> ServiceResult<Vec<Section>> {
        let (recent, playlists, albums, top) = tokio::join!(
            self.get("/me/player/recently-played?limit=50"),
            self.playlists(0),
            self.saved_albums(0),
            self.get("/me/top/artists?limit=20&time_range=short_term"),
        );

        let mut sections = Vec::new();
        let mut first_error = None;
        let mut note = |e: ServiceError| {
            log::warn!("home shelf failed: {e}");
            first_error.get_or_insert(e);
        };

        match recent.and_then(|b| parse_recently_played(&b)) {
            Ok((tracks, contexts)) => {
                let jump_back = self.resolve_contexts(contexts, 12).await;
                if !jump_back.is_empty() {
                    sections.push(section(
                        "jumpBackIn",
                        "Jump back in",
                        SectionLayout::Cards,
                        jump_back,
                    ));
                }
                if !tracks.is_empty() {
                    sections.push(section(
                        "recentTracks",
                        "Recently played",
                        SectionLayout::Cards,
                        tracks.into_iter().take(20).collect(),
                    ));
                }
            }
            Err(e) => note(e),
        }
        match playlists {
            Ok(p) if !p.items.is_empty() => sections.push(section(
                "playlists",
                "Your playlists",
                SectionLayout::Cards,
                p.items.into_iter().take(20).collect(),
            )),
            Ok(_) => {}
            Err(e) => note(e),
        }
        match top.and_then(|b| parse_top_artists(&b)) {
            Ok(a) if !a.is_empty() => sections.push(section(
                "topArtists",
                "Your top artists",
                SectionLayout::Cards,
                a,
            )),
            Ok(_) => {}
            Err(e) => note(e),
        }
        match albums {
            Ok(p) if !p.items.is_empty() => sections.push(section(
                "albums",
                "Saved albums",
                SectionLayout::Cards,
                p.items.into_iter().take(20).collect(),
            )),
            Ok(_) => {}
            Err(e) => note(e),
        }

        match (sections.is_empty(), first_error) {
            (true, Some(e)) => Err(e),
            _ => Ok(sections),
        }
    }

    /// Describe recently played contexts, fetching names for playlists and
    /// artists (albums were already described by their tracks).
    async fn resolve_contexts(&self, hints: Vec<ContextHint>, max: usize) -> Vec<MediaItem> {
        use futures_util::stream::{self, StreamExt};
        let resolved: Vec<Option<MediaItem>> = stream::iter(hints.into_iter().take(max))
            .map(|h| async move {
                if let Some(item) = h.resolved {
                    return Some(item);
                }
                let result = match MediaKind::from_uri(&h.uri)? {
                    MediaKind::Playlist => self.playlist_summary(&h.uri).await,
                    MediaKind::Artist => self.artist_summary(&h.uri).await,
                    _ => return None,
                };
                result
                    .map_err(|e| log::debug!("context not resolved: {e}"))
                    .ok()
            })
            .buffered(4)
            .collect()
            .await;
        resolved.into_iter().flatten().collect()
    }
}

fn section(id: &str, title: &str, layout: SectionLayout, items: Vec<MediaItem>) -> Section {
    Section {
        id: id.into(),
        title: title.into(),
        layout,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: &str = r#"{
        "uri": "spotify:track:t1", "name": "Song", "duration_ms": 1000, "explicit": true,
        "artists": [{"name": "A", "uri": "spotify:artist:a"}, {"name": "B"}],
        "album": {"uri": "spotify:album:al", "name": "Alb",
                  "images": [{"url": "big", "width": 640}, {"url": "mid", "width": 300}, {"url": "small", "width": 64}]}
    }"#;

    #[test]
    fn track_conversion_picks_card_sized_art() {
        let t: Track = serde_json::from_str(TRACK).unwrap();
        let item = track_item(&t, None).unwrap();
        assert_eq!(item.subtitle, "A, B");
        assert_eq!(item.artwork_url.as_deref(), Some("mid"));
        assert_eq!(item.context_uri.as_deref(), Some("spotify:album:al"));
        assert!(item.explicit);
    }

    #[test]
    fn search_skips_null_and_incomplete_items() {
        let body = format!(
            r#"{{"tracks": {{"items": [{TRACK}, null, {{"uri": "spotify:track:x"}}]}},
                 "playlists": {{"items": [null, {{"uri": "spotify:playlist:p", "name": "P", "images": null, "owner": {{"display_name": "me"}}}}]}}}}"#
        );
        let r = parse_search("q", body.as_bytes()).unwrap();
        assert_eq!(r.tracks.len(), 1);
        assert_eq!(r.playlists.len(), 1);
        assert_eq!(r.playlists[0].subtitle, "Playlist · me");
        assert!(r.albums.is_empty());
    }

    #[test]
    fn playlist_accepts_renamed_items_field() {
        let new_style = format!(
            r#"{{"uri": "spotify:playlist:p", "name": "P", "items": {{"total": 2, "items": [{{"item": {TRACK}}}]}}}}"#
        );
        let d = parse_playlist_detail(new_style.as_bytes()).unwrap();
        assert_eq!(d.tracks.len(), 1);
        assert!(d.truncated);
        let old_style = format!(
            r#"{{"uri": "spotify:playlist:p", "name": "P", "tracks": {{"total": 1, "items": [{{"track": {TRACK}}}]}}}}"#
        );
        assert_eq!(
            parse_playlist_detail(old_style.as_bytes())
                .unwrap()
                .tracks
                .len(),
            1
        );
    }

    #[test]
    fn playlist_with_both_old_and_new_fields() {
        // Spotify sends both `tracks` and `items` (and `track` + `item`).
        let body = format!(
            r#"{{"uri": "spotify:playlist:p", "name": "P",
                "tracks": {{"href": "x", "total": 2}},
                "items": {{"total": 2, "items": [{{"track": {TRACK}, "item": {TRACK}}}]}}}}"#
        );
        let d = parse_playlist_detail(body.as_bytes()).unwrap();
        assert_eq!(d.tracks.len(), 1);
        assert_eq!(d.total_tracks, Some(2));

        let search = r#"{"playlists": {"items": [
            {"uri": "spotify:playlist:p", "name": "P", "owner": {"display_name": "me"},
             "tracks": {"href": "x", "total": 5}, "items": {"href": "y", "total": 5}}
        ]}}"#;
        assert_eq!(
            parse_search("q", search.as_bytes())
                .unwrap()
                .playlists
                .len(),
            1
        );
        let library = r#"{"items": [{"uri": "spotify:playlist:p", "name": "P",
            "tracks": {"total": 1}, "items": {"total": 1}}], "total": 1}"#;
        assert_eq!(
            parse_playlists(library.as_bytes(), 0).unwrap().items.len(),
            1
        );
    }

    #[test]
    fn recently_played_dedups_and_resolves_albums() {
        let body = format!(
            r#"{{"items": [
                {{"track": {TRACK}, "context": {{"uri": "spotify:album:al"}}}},
                {{"track": {TRACK}, "context": {{"uri": "spotify:album:al"}}}},
                {{"track": {TRACK}, "context": {{"uri": "spotify:playlist:p"}}}},
                {{"track": {TRACK}, "context": null}}
            ]}}"#
        );
        let (tracks, contexts) = parse_recently_played(body.as_bytes()).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(contexts.len(), 2);
        assert!(contexts[0].resolved.is_some());
        assert!(contexts[1].resolved.is_none());
    }

    #[test]
    fn album_detail_description() {
        let body = format!(
            r#"{{"uri": "spotify:album:al", "name": "Alb", "album_type": "album", "release_date": "2019-05-01",
                "total_tracks": 1, "images": [{{"url": "x", "width": 640}}],
                "tracks": {{"items": [{TRACK}]}}}}"#
        );
        let d = parse_album_detail(body.as_bytes()).unwrap();
        assert_eq!(d.description, "Album · 2019 · 1 song");
        assert!(!d.truncated);
    }

    #[test]
    fn artist_albums_split_by_type() {
        let body = br#"{"items": [
            {"uri": "spotify:album:a", "name": "LP", "album_type": "album", "release_date": "2020-01-01"},
            {"uri": "spotify:album:s", "name": "Single", "album_type": "single", "release_date": "2021"},
            {"uri": "spotify:album:c", "name": "Best Of", "album_type": "compilation"},
            null
        ]}"#;
        let (albums, singles) = parse_artist_albums(body).unwrap();
        assert_eq!(albums.len(), 2);
        assert_eq!(albums[0].subtitle, "2020");
        assert_eq!(singles.len(), 1);
        assert_eq!(singles[0].subtitle, "2021");
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse_search("q", b"{not json").is_err());
        assert!(parse_queue(b"[]").is_err());
    }

    #[test]
    fn library_kinds() {
        assert_eq!(library_kind("spotify:artist:a"), Some(MediaKind::Artist));
        assert_eq!(
            library_kind("spotify:playlist:p"),
            Some(MediaKind::Playlist)
        );
        assert_eq!(library_kind("spotify:user:me:collection"), None);
        assert_eq!(library_kind("preview:album:1"), None);
    }

    #[test]
    fn helpers() {
        assert_eq!(encode("a b&c/ü"), "a%20b%26c%2F%C3%BC");
        assert_eq!(
            uri_id("spotify:album:4aawyAB9vmqN3uQ7FjRGTy"),
            Some("4aawyAB9vmqN3uQ7FjRGTy")
        );
        assert_eq!(uri_id("spotify:album:../x"), None);
        assert_eq!(
            strip_html("Hi <a href=\"x\">there</a> &amp; more"),
            "Hi there & more"
        );
    }
}
