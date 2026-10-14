//! Artwork download and dynamic-color extraction.

use std::collections::{HashMap, VecDeque};

use bigify_core::color::{seed_from_pixels, Scheme};
use bigify_core::error::{ServiceError, ServiceResult};
use bytes::Bytes;
use librespot::core::session::Session;

use crate::errors::from_librespot;

/// Only Spotify's image CDN is fetched; artwork URLs come from Spotify
/// responses, but we still refuse to fetch arbitrary hosts.
fn is_allowed(url: &str) -> bool {
    url.starts_with("https://i.scdn.co/")
        || url.starts_with("https://mosaic.scdn.co/")
        || url.starts_with("https://image-cdn-ak.spotifycdn.com/")
        || url.starts_with("https://image-cdn-fa.spotifycdn.com/")
}

async fn download(session: &Session, url: &str) -> ServiceResult<Bytes> {
    if !is_allowed(url) {
        return Err(ServiceError::unsupported("Artwork host not allowed"));
    }
    let req = http::Request::get(url)
        .body(Bytes::new())
        .map_err(|e| ServiceError::internal(format!("Invalid artwork URL ({e})")))?;
    session
        .http_client()
        .request_body(req)
        .await
        .map_err(from_librespot)
}

/// Decode and reduce artwork to a seed color. CPU-bound; call from a
/// blocking task.
fn scheme_from_bytes(bytes: &[u8]) -> ServiceResult<Option<Scheme>> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| ServiceError::internal(format!("Unreadable artwork ({e})")))?;
    let small = img.thumbnail(64, 64).to_rgb8();
    Ok(seed_from_pixels(small.as_raw()).map(Scheme::dark_from_seed))
}

/// Small LRU of computed schemes, keyed by artwork URL.
pub struct PaletteCache {
    map: HashMap<String, Scheme>,
    order: VecDeque<String>,
    capacity: usize,
}

impl PaletteCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn get(&self, url: &str) -> Option<Scheme> {
        self.map.get(url).cloned()
    }

    pub fn insert(&mut self, url: String, scheme: Scheme) {
        if self.map.insert(url.clone(), scheme).is_none() {
            self.order.push_back(url);
            while self.order.len() > self.capacity {
                if let Some(old) = self.order.pop_front() {
                    self.map.remove(&old);
                }
            }
        }
    }
}

/// Compute the scheme for an artwork URL. Monochrome artwork yields the brand
/// scheme rather than an invented accent.
pub async fn scheme_for(session: &Session, url: &str) -> ServiceResult<Scheme> {
    let bytes = download(session, url).await?;
    let scheme = tokio::task::spawn_blocking(move || scheme_from_bytes(&bytes))
        .await
        .map_err(|e| ServiceError::internal(format!("Palette task failed ({e})")))??;
    Ok(scheme.unwrap_or_else(Scheme::brand))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_spotify_cdn_is_allowed() {
        assert!(is_allowed("https://i.scdn.co/image/ab67616d0000b273"));
        assert!(!is_allowed("http://i.scdn.co/image/x"));
        assert!(!is_allowed("https://evil.example/i.scdn.co/"));
    }

    #[test]
    fn lru_evicts_oldest() {
        let mut c = PaletteCache::new(2);
        c.insert("a".into(), Scheme::brand());
        c.insert("b".into(), Scheme::brand());
        c.insert("c".into(), Scheme::brand());
        assert!(c.get("a").is_none());
        assert!(c.get("c").is_some());
    }
}
