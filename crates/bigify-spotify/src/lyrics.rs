//! Lyrics from LRCLIB (<https://lrclib.net>), an open, community-maintained
//! lyrics database with a free, keyless API. Synced lyrics come as LRC
//! (`[mm:ss.xx] line`); plain lyrics are used when no timing exists.
//!
//! Only the track's title, artists, album and duration are sent, and only
//! when the user opens the lyrics view.

use bigify_core::error::{ErrorKind, ServiceError, ServiceResult};
use bigify_core::model::{LyricLine, Lyrics};
use bytes::Bytes;
use librespot::core::session::Session;
use serde::Deserialize;

use crate::errors::from_librespot;
use crate::webapi::encode;

const API: &str = "https://lrclib.net/api";
/// Releases of the same song differ by a second or two.
const DURATION_TOLERANCE_SECS: f64 = 3.0;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

pub struct Query<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub duration_ms: u32,
}

async fn get(session: &Session, path_and_query: &str) -> ServiceResult<Bytes> {
    let req = http::Request::get(format!("{API}{path_and_query}"))
        .header(http::header::ACCEPT, "application/json")
        .body(Bytes::new())
        .map_err(|e| ServiceError::internal(format!("Invalid lyrics request ({e})")))?;
    session
        .http_client()
        .request_body(req)
        .await
        .map_err(from_librespot)
}

fn not_found() -> ServiceError {
    ServiceError::new(ErrorKind::NotFound, "No lyrics found for this song.")
}

pub async fn fetch(session: &Session, q: &Query<'_>) -> ServiceResult<Lyrics> {
    if q.title.trim().is_empty() {
        return Err(not_found());
    }
    let secs = (f64::from(q.duration_ms) / 1000.0).round() as u64;
    // Exact lookup first; LRCLIB answers unknown songs with 404 or 503.
    let exact = format!(
        "/get?track_name={}&artist_name={}&album_name={}&duration={secs}",
        encode(q.title),
        encode(q.artist),
        encode(q.album),
    );
    match get(session, &exact).await {
        Ok(body) => {
            let record: Record = serde_json::from_slice(&body)
                .map_err(|_| ServiceError::internal("Unexpected response from LRCLIB"))?;
            if let Some(lyrics) = to_lyrics(&record) {
                return Ok(lyrics);
            }
        }
        Err(e) if e.kind == ErrorKind::Network => return Err(e),
        Err(e) => log::debug!("lyrics exact lookup: {e}"),
    }
    let search = format!(
        "/search?track_name={}&artist_name={}",
        encode(q.title),
        encode(q.artist)
    );
    let body = get(session, &search).await?;
    let records: Vec<Record> = serde_json::from_slice(&body)
        .map_err(|_| ServiceError::internal("Unexpected response from LRCLIB"))?;
    best_match(&records, q.duration_ms)
        .and_then(to_lyrics)
        .ok_or_else(not_found)
}

/// Closest duration within tolerance, preferring synced lyrics.
fn best_match(records: &[Record], duration_ms: u32) -> Option<&Record> {
    let target = f64::from(duration_ms) / 1000.0;
    records
        .iter()
        .filter(|r| {
            duration_ms == 0
                || r.duration
                    .is_none_or(|d| (d - target).abs() <= DURATION_TOLERANCE_SECS)
        })
        .filter(|r| r.instrumental || r.synced_lyrics.is_some() || r.plain_lyrics.is_some())
        .min_by_key(|r| {
            let synced = r
                .synced_lyrics
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
            let diff = r.duration.map_or(999.0, |d| (d - target).abs());
            (!synced, (diff * 10.0) as i64)
        })
}

fn to_lyrics(r: &Record) -> Option<Lyrics> {
    if r.instrumental {
        return Some(Lyrics {
            lines: vec![],
            synced: false,
            instrumental: true,
            source: "LRCLIB".into(),
        });
    }
    if let Some(lines) = r
        .synced_lyrics
        .as_deref()
        .map(parse_lrc)
        .filter(|l| !l.is_empty())
    {
        return Some(Lyrics {
            lines,
            synced: true,
            instrumental: false,
            source: "LRCLIB".into(),
        });
    }
    let plain = r.plain_lyrics.as_deref()?.trim();
    if plain.is_empty() {
        return None;
    }
    Some(Lyrics {
        lines: plain
            .lines()
            .map(|t| LyricLine {
                time_ms: None,
                text: t.trim().to_owned(),
            })
            .collect(),
        synced: false,
        instrumental: false,
        source: "LRCLIB".into(),
    })
}

/// Parse `[mm:ss.xx]` LRC. Lines may carry several timestamps; metadata
/// tags such as `[ar:…]` are ignored. Output is sorted by time.
pub fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let tag = &stripped[..end];
            match parse_timestamp(tag) {
                Some(ms) => times.push(ms),
                None => {
                    times.clear();
                    break; // metadata tag: skip line
                }
            }
            rest = stripped[end + 1..].trim_start();
        }
        for ms in times {
            out.push(LyricLine {
                time_ms: Some(ms),
                text: rest.trim().to_owned(),
            });
        }
    }
    out.sort_by_key(|l| l.time_ms);
    out
}

fn parse_timestamp(tag: &str) -> Option<u32> {
    let (min, sec) = tag.split_once(':')?;
    let min: u32 = min.trim().parse().ok()?;
    let sec: f64 = sec.trim().parse().ok()?;
    if !(0.0..60.0).contains(&sec) {
        return None;
    }
    Some(min * 60_000 + (sec * 1000.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_parsing() {
        let lines = parse_lrc(
            "[ar:Someone]\n[00:17.12] First line\n[00:05.00][01:02.5]Chorus\n[00:20.00]\nnot lrc",
        );
        let times: Vec<_> = lines.iter().map(|l| l.time_ms.unwrap()).collect();
        assert_eq!(times, vec![5_000, 17_120, 20_000, 62_500]);
        assert_eq!(lines[0].text, "Chorus");
        assert_eq!(lines[1].text, "First line");
        assert_eq!(lines[2].text, "", "empty timed line keeps the gap");
    }

    fn rec(duration: f64, synced: bool) -> Record {
        Record {
            duration: Some(duration),
            instrumental: false,
            plain_lyrics: Some("a\nb".into()),
            synced_lyrics: synced.then(|| "[00:01.00]a".into()),
        }
    }

    #[test]
    fn best_match_prefers_synced_within_tolerance() {
        let records = vec![
            rec(200.0, false),
            rec(370.0, true),
            rec(368.0, false),
            rec(500.0, true),
        ];
        let best = best_match(&records, 369_000).unwrap();
        assert_eq!(best.duration, Some(370.0));
        assert!(best_match(&[rec(100.0, true)], 369_000).is_none());
    }

    #[test]
    fn plain_and_instrumental() {
        let plain = to_lyrics(&rec(1.0, false)).unwrap();
        assert!(!plain.synced);
        assert_eq!(plain.lines.len(), 2);
        let inst = to_lyrics(&Record {
            duration: None,
            instrumental: true,
            plain_lyrics: None,
            synced_lyrics: None,
        })
        .unwrap();
        assert!(inst.instrumental);
        assert!(to_lyrics(&Record {
            duration: None,
            instrumental: false,
            plain_lyrics: Some("  ".into()),
            synced_lyrics: None,
        })
        .is_none());
    }
}
