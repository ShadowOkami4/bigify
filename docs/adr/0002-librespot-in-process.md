# ADR 0002 — librespot linked in-process

**Status:** accepted · 2026-10-14

## Context

The playback engine must expose playback control, events, metadata,
artwork, authentication and device discovery to the UI.

## Options considered

- **librespot crate in-process (chosen).** Direct access to `Spirc`
  (Connect control: play, pause, seek, shuffle, repeat, volume, load
  context), the `PlayerEvent` stream (confirmed state, metadata, artwork),
  `Discovery` (phone sign-in), the OAuth client, session tokens and the
  metadata API.
- **librespot binary as a sidecar.** Only exposes `--onevent` hooks and has
  no control API, so it can't satisfy UI-driven playback control.
- **spotifyd / go-librespot daemons.** Add a separate process and runtime,
  plus an IPC layer, with nothing gained over in-process use.

## Decision

Link librespot 0.8 (`default-features = false`) with the `pulseaudio-backend`,
`alsa-backend`, `rustls-tls-webpki-roots` and `with-libmdns` features.

- PulseAudio covers PipeWire systems (SteamOS, modern desktops) via
  `pipewire-pulse`; ALSA serves minimal systems.
- rustls avoids an OpenSSL runtime dependency.
- libmdns is a pure-Rust mDNS responder, needing no Avahi daemon.

The engine loop follows librespot's reference binary to stay close to
upstream behaviour.

## Consequences

- `Cargo.lock` pins `vergen` to 9.0.x: librespot-core 0.8's build script
  fails to compile with vergen 9.1 (a trait mismatch with `vergen-lib`).
  Revisit when librespot updates.
- Spotify protocol changes are absorbed by updating librespot.
