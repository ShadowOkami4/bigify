# Architecture

```
┌──────────────────────────── bigify (binary) ────────────────────────────┐
│  QML UI (Qt Quick 6.8)                                                   │
│   Main.qml ─ NavRail · SectionPages · NowPlaying · Queue · SignIn        │
│   Theme (design tokens) · components/ · screens/                         │
│          ▲ properties / signals (JSON)        │ invokables               │
│          │                                    ▼                          │
│  bridge.rs — `Backend` QML singleton (cxx-qt 0.10)                       │
│   PlaybackState (requested vs confirmed) · Settings · palette selection  │
│          ▲ Event (queued onto Qt thread)      │ Command (mpsc)           │
│  input.rs — gilrs controller thread ──▶ key events (cpp/input.cpp)       │
└──────────┼────────────────────────────────────┼──────────────────────────┘
           │                                    ▼
┌──────────┴──────── bigify-spotify (own thread + Tokio) ─────────────────┐
│  engine.rs   select! loop: commands · discovery · OAuth · (re)connect    │
│  librespot: Session · Spirc (Connect) · Player · Discovery · OAuth       │
│  webapi.rs   Web API via session token (home, search, library, queue)    │
│  metadata.rs librespot metadata fallback for playlists                   │
│  artwork.rs  artwork download → seed color → Scheme                      │
└──────────────────────────────────────────────────────────────────────────┘
┌──────────────────────── bigify-core (no Qt, no network) ────────────────┐
│  model · playback state machine · Command/Event · settings · color · XDG│
└──────────────────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Responsibility | Depends on |
|---|---|---|
| `bigify-core` | Domain model, playback state machine, UI↔service contract, settings, dynamic color, XDG paths. Pure and unit-tested. | serde, toml |
| `bigify-spotify` | Everything that talks to Spotify: librespot session/Connect/player, discovery and OAuth sign-in, reconnection, Web API, artwork palettes. | bigify-core, librespot, tokio |
| `bigify` (`crates/bigify-app`) | Qt application: QML UI, the `Backend` bridge, controller input, logging, lifecycle. | bigify-core, bigify-spotify, cxx-qt, gilrs |

The UI never touches librespot types and the service never touches Qt.
Everything crosses the boundary as `bigify_core::Command` / `bigify_core::Event`.

## Threads

| Thread | Work |
|---|---|
| Qt GUI thread | QML, the `Backend` object, the playback state machine |
| `bigify-spotify` + 2 `bigify-net` Tokio workers | librespot, HTTP, artwork decoding (`spawn_blocking`) |
| librespot player thread | Audio decoding and output (owned by librespot) |
| `bigify-input` | Blocks on gilrs events; wakes only for input or key repeat |
| `bigify-mpris` | zbus MPRIS server on a local Tokio runtime; receives state snapshots, queues remote controls onto the Qt thread |
| `bigify-oauth` (transient) | librespot's blocking OAuth redirect listener |

Service events are delivered with `CxxQtThread::queue`, so QObject state is
only mutated on the GUI thread. Background services are shut down from
`main` after the event loop returns, never from a QObject destructor. That
avoids a deadlock between a dying object and a thread queueing onto it.

## Requested vs confirmed state

`bigify_core::playback::PlaybackState` keeps three things apart:

1. **User intents.** Pressing Play records `Intent::Play` as pending, and
   `Backend.pending` exposes it, so the button shows a loading indicator.
2. **Commands.** The intent is sent to Spirc as a `Command`.
3. **Confirmed state.** Only librespot `PlayerEvent`s (mapped to
   `PlayerUpdate`) change `isPlaying`, the track, the position and so on.
   Matching updates resolve the pending intent.

Intents not confirmed within 8 s expire and are reported to the user.
Track-change intents (next/previous) resolve only when the URI actually
changes, so a position correction can't be mistaken for success. Metadata of
a previous track is dropped as soon as a different URI is reported.

Playback position is never polled. The UI extrapolates from the last
confirmed sample only while the engine says it is playing, and only while a
progress indicator is visible (`PositionClock.qml`).

## Connection lifecycle

`engine.rs` mirrors the event loop of librespot's reference binary:

- Cached credentials go straight to connecting. Without them, Bigify
  advertises itself via mDNS (`SignedOut { discovery: true }`) and waits for
  a phone, or for the user to choose browser sign-in.
- Each connection attempt uses a fresh `Session`; the `Player` persists and
  is re-pointed with `set_session`.
- If the Spirc task ends unexpectedly, Bigify reconnects with exponential
  backoff (1 s … 30 s). The UI stays usable and shows a banner.
- `Unauthenticated` or `PermissionDenied` while connecting removes the
  stored credentials and returns to sign-in.
- A 10-second health tick recreates the audio player if librespot reports it
  invalid, for example after an audio device disappears.

## UI structure and focus

- `Main.qml` holds the rail and section pages. Each section page owns its
  root screen and an optional album/playlist detail page. Now Playing and
  the queue are overlays that remember and restore the previous focus.
- Navigation is keyboard-event based. Controllers are translated into key
  events, so controllers, TV remotes and keyboards share one code path. See
  [CONTROLS.md](CONTROLS.md).
- Browse data is delivered as JSON through signals. Small sets (shelves,
  search results) are used as JS arrays. The paged library uses `ListModel`s
  appended page by page, with delegate reuse. If profiling shows a need, a
  `QAbstractListModel` in Rust is the planned next step.

## Theming

`qml/theme/Theme.qml` holds all design tokens. See [DESIGN.md](DESIGN.md).
The color scheme comes from Rust (`bigify_core::color`): either the brand
seed or a seed extracted from the current artwork. Tones are generated so
text roles meet WCAG contrast, which is unit-tested over 39 seeds.

## Key decisions

- [ADR 0001 — Qt Quick with a Rust core](adr/0001-ui-stack.md)
- [ADR 0002 — librespot linked in-process](adr/0002-librespot-in-process.md)
- [ADR 0003 — Web API access via the session token](adr/0003-web-api-access.md)
- [ADR 0004 — GPL-3.0-or-later](adr/0004-license.md)
