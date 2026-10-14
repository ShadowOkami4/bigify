# Capabilities and limitations

Bigify promises only what its interfaces can deliver. This page is the
source of truth for what works, what works with caveats, and what cannot be
built today. Last verified: **October 2026** against librespot 0.8.0 and
Spotify's February 2026 Web API changes.

## The two interfaces

| Interface | Used for | Auth |
|---|---|---|
| **librespot 0.8** (Rust crate, linked in) | Session, Spotify Connect receiver, audio playback, player events, track/playlist metadata | OAuth (browser) or Spotify Connect discovery (phone); reusable credentials cached locally |
| **Spotify Web API** | Home shelves, search, library listings, queue view, add-to-queue | **The user's own Spotify app** (one-time setup, OAuth PKCE). Without it, tokens from the librespot session are tried, but Spotify rate-limits those on the Web API (HTTP 429, retry times of hours) |

### Why the session token for the Web API

Since February 2026, apps in Spotify's *Development Mode* need a Premium owner,
are limited to **5 users per Client ID**, return at most **10 search results**
per type, and receive playlist contents only for playlists the user owns. A
single Bigify Client ID could therefore serve only five people.

Bigify reuses the librespot session's token instead, as other open-source
clients have done (the maintainers chose this option; see
[ADR 0003](adr/0003-web-api-access.md)). Trade-offs:

- **Risk:** this borrows Spotify's own client identity. Spotify can rate-limit
  or block it at any time, and it is a grey area under Spotify's developer
  terms. Bigify keeps request volume low (no polling; data loads on demand).
- **Mitigation path:** the Web API client is isolated in
  `crates/bigify-spotify/src/webapi.rs`. Supporting a user-supplied Client ID
  is a contained change if the session-token approach stops working.

## Feature matrix

✅ works · ⚠️ works with caveats · ❌ not possible with current interfaces

| Feature | Status | Source | Notes |
|---|---|---|---|
| Sign in from phone (Connect discovery) | ✅ | librespot-discovery (mDNS) | Needs phone and Bigify on the same LAN; mDNS must not be blocked |
| Sign in with browser on this device | ✅ | librespot-oauth | Opens the system browser; redirect to `127.0.0.1:8898` |
| Stay signed in | ✅ | librespot cache | Reusable credentials in `$XDG_STATE_HOME/bigify/credentials.json` (mode 0600) |
| Play / pause / next / previous / seek | ✅ | Spirc | Bigify is the playing device |
| Shuffle, repeat (context / track) | ✅ | Spirc | |
| Volume | ✅ | Spirc + softmixer | Software volume on Bigify's own output |
| Track metadata and artwork | ✅ | librespot player events | Title, artists, album, duration, cover |
| Confirmed playback position | ✅ | player events | UI extrapolates between confirmed samples only while playing |
| Controlled from phone/desktop app | ✅ | Spotify Connect | Bigify appears as a TV device |
| Show playback happening on *other* devices | ❌ | — | Bigify is a Connect receiver; it shows its own playback |
| Home: Jump back in, recently played, your playlists, top artists, saved albums | ⚠️ | Web API | Each shelf loads independently; failing shelves are hidden |
| Home: editorial / recommended / "Made for you" | ❌ | — | Browse, recommendations and related endpoints are removed for third-party access; Bigify never invents them |
| Search (tracks, albums, artists, playlists) | ⚠️ | Web API | Bigify requests 10 results per type (the Development Mode cap), keeping request volume low |
| Library: playlists, albums, followed artists, liked songs | ⚠️ | Web API | Paged 50 at a time on demand |
| Album detail | ✅ | Web API | |
| Playlist detail (owned) | ✅ | Web API | First 100 tracks are listed; Play plays everything |
| Playlist detail (followed / editorial) | ⚠️ | librespot metadata fallback | Web API withholds contents; Bigify reads the playlist via librespot and lists the first 100 tracks |
| Artist page: portrait, popular tracks | ⚠️ | librespot metadata | `artists/{id}/top-tracks` was removed from the Web API; librespot reads the same data Spotify clients use (top 5 for your country) |
| Artist page: albums, singles & EPs | ⚠️ | Web API `artists/{id}/albums`, librespot fallback | Falls back to librespot album metadata if the Web API refuses |
| Queue: view | ⚠️ | Web API `GET /me/player/queue` | Read-only |
| Queue: add | ⚠️ | Web API `POST /me/player/queue` | |
| Queue: reorder / remove | ❌ | — | No public interface |
| Save / unsave albums, playlists, tracks; follow artists | ⚠️ | Web API `PUT/DELETE /me/library` (new) with fallback to type-specific endpoints; `/me/following` for artists | Buttons appear only after Spotify reports the current state; changes show as pending until confirmed |
| Lyrics (synced and plain) | ✅ | [LRCLIB](https://lrclib.net) (open, keyless API) | Fetched only when you open lyrics; sends title, artist, album and duration to lrclib.net. Coverage depends on the community database |
| Desktop integration (MPRIS): media keys, KDE/GNOME media widgets, `playerctl`, headset buttons | ✅ | D-Bus (zbus, pure Rust) | Reports confirmed state; controls go through the same pending/confirm path as the UI. Disabled without a session bus |
| Audio quality 96/160/320 kbit/s | ✅ | librespot `PlayerConfig` | Applies after restart |
| Normalisation, gapless | ✅ | librespot `PlayerConfig` | Applies after restart |
| Output: PulseAudio / PipeWire, ALSA | ✅ | librespot backends | Device follows the system default; no per-device picker yet |
| Podcasts / audiobooks | ⚠️ | librespot | Episodes play and show show-name; no podcast browsing UI |
| Offline downloads | ❌ | — | Not offered by librespot; would violate Spotify terms |
| Free (non-Premium) accounts | ❌ | — | librespot requires Premium |

## Error vocabulary

The UI distinguishes errors so it never offers a pointless retry:

| Kind | Meaning | UI |
|---|---|---|
| `network`, `rateLimited`, `internal` | Temporary | Message + **Try again** |
| `unauthorized` | Session rejected | Prompts to sign in again |
| `unsupported` | Not possible through Bigify's interfaces | Message only, no retry |
| `notFound` | Item unavailable (e.g. region) | Message only |
| `notReady` | Not signed in yet | Message + retry |

Playback requests that are not confirmed by the engine within 8 seconds are
reported as failures ("Playback didn't start") instead of silently showing a
playing state.

## Things that need real-world verification

These cannot be verified in CI and are tracked in the manual test checklist
([DEVELOPMENT.md](DEVELOPMENT.md#manual-test-checklist)):

- Discovery sign-in from iOS/Android Spotify apps on a typical home network.
- Session-token Web API calls succeeding for search, library and queue on
  real accounts (and how rate limits behave).
- Audio output on SteamOS (PipeWire) in Gaming Mode and Desktop Mode.
- Gamepad input under Steam Input in Gaming Mode.
