# ADR 0003 — Web API access via the librespot session token

**Status:** accepted (with known risk) · 2026-10-14

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
