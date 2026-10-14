# ADR 0003 — Web API access via the librespot session token

**Status:** superseded by the user-app approach (see Update) · 2026-10-14

## Context

Home, search, library and queue need the Spotify Web API. Since February 2026,
Development Mode apps require a Premium owner, allow 5 users per Client ID,
cap search at 10 results and withhold contents of playlists the user doesn't
own. Extended quota is only available to established organisations.

## Options considered

1. **User-supplied Client ID.** Compliant, but every user must register a
   developer app, which is significant friction on a TV.
2. **Session token (chosen by the maintainers).** No extra setup. It uses the
   token librespot obtains for its session (Spotify's desktop client ID).
3. Both, preferring the user's Client ID.

## Decision

Option 2. All Web API access goes through
`crates/bigify-spotify/src/webapi.rs`, which obtains tokens from
`session.token_provider()`.

## Consequences

- This is a grey area under Spotify's terms. Spotify may rate-limit or block
  it, and the README states the risk.
- Request volume is kept low: no polling, on-demand paging, a single home
  fetch per sign-in, and librespot's client honours `Retry-After`.
- Switching to option 1 or 3 means adding a token source to `WebApi`; no UI
  code depends on how tokens are obtained.
- Followed and editorial playlists are read through librespot's metadata API
  when the Web API withholds them.

## Update — 2026-10-14: session tokens are rate-limited

Field testing on a Steam Deck showed the session-token approach failing:

1. librespot's `TokenProvider` uses the Mercury `hm://keymaster` endpoint,
   which Spotify is phasing out — token requests failed outright.
2. Switching to login5 tokens obtained a token, but `api.spotify.com`
   answers requests made with it with HTTP 429 and `Retry-After` values far
   beyond librespot's maximum wait. Spotify throttles third-party use of
   its own client's tokens on the Web API.

**New decision (option 3, "both, user app preferred"):** Bigify offers a
one-time *library access* setup in which the user registers a free Spotify
developer app and authorises it (OAuth 2.0 authorization code + PKCE; no
client secret). The setup page is served by Bigify on
`http://127.0.0.1:8899/` so all typing happens in a browser. The resulting
refresh token is stored in `$XDG_STATE_HOME/bigify/webapi-token.json`
(mode 0600) and is used exclusively for Web API calls; session tokens
remain only as a fallback, and a 429 on them now tells the user to set up
library access. Development Mode limits (search capped at 10 results,
playlist contents only for owned playlists — librespot fallback in place)
apply as documented in LIMITATIONS.md.
