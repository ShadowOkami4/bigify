# Development

## Requirements

- Rust 1.90 (pinned in `rust-toolchain.toml`)
- Qt **6.8 or newer**: Base, Declarative (Qt Quick, QML), ShaderTools/Effects
  (for `QtQuick.Effects`), Shapes
- A C++17 compiler; `lld` or `mold` is recommended for linking
- PulseAudio and ALSA development headers, and libudev (for controllers)

### Debian 13 / Ubuntu 25.04+

```sh
sudo apt install build-essential clang lld pkg-config qmake6 \
  qt6-base-dev qt6-declarative-dev qt6-declarative-private-dev \
  qml6-module-qtquick qml6-module-qtquick-window qml6-module-qtquick-layouts \
  qml6-module-qtquick-effects qml6-module-qtquick-shapes qml6-module-qtqml-workerscript \
  libpulse-dev libasound2-dev libudev-dev
export QMAKE=/usr/bin/qmake6
```

### Fedora 41+

```sh
sudo dnf install gcc-c++ clang lld qt6-qtbase-devel qt6-qtdeclarative-devel \
  pulseaudio-libs-devel alsa-lib-devel systemd-devel
```

### Arch / SteamOS (Desktop Mode, developer setup)

```sh
sudo pacman -S base-devel clang lld qt6-base qt6-declarative libpulse alsa-lib systemd-libs
```

On SteamOS the root filesystem is read-only. Build in a container
(Distrobox/Toolbox) or with the Docker image below, and use the Flatpak for
day-to-day use.

## Build and run

```sh
cargo build --release
./target/release/bigify
```

### Reproducible container build (any OS with Docker)

```sh
docker build -t bigify-dev -f ci/Dockerfile .
ci/run.sh cargo test --workspace
ci/run.sh cargo clippy --workspace --all-targets
ci/run.sh cargo build --release
```

`ci/run.sh` mounts the source tree and keeps the cargo registry and `target/`
in Docker volumes.

## Tests

```sh
cargo test --workspace
```

| Area | Where |
|---|---|
| Playback state machine (requested vs confirmed, expiry, stale metadata) | `bigify-core/src/playback.rs` |
| Dynamic color and WCAG contrast over 39 seeds | `bigify-core/src/color.rs` |
| Settings persistence and validation | `bigify-core/src/settings.rs` |
| Web API parsing (nulls, renamed fields, malformed JSON) | `bigify-spotify/src/webapi.rs` |
| Controller repeat, hysteresis, direction replacement | `bigify-app/src/input.rs` |

## Design preview and screenshots

Review layouts without a Spotify account. The preview fills the UI with
clearly labelled sample content and never contacts Spotify.

```sh
BIGIFY_DESIGN_PREVIEW=1 ./target/debug/bigify
```

Headless screenshots (Xvfb + Mesa) for any screen and resolution:

```sh
ci/run.sh ci/screenshot.sh home 1920x1080
ci/run.sh ci/screenshot.sh nowPlaying 3840x2160
ci/run.sh ci/screenshot.sh queue 1280x800    # Steam Deck
# screens: home, search, library, settings, nowPlaying, queue, rail, artist, album
```

Images are written to `screenshots/`, which git ignores.

## Environment variables

| Variable | Purpose |
|---|---|
| `BIGIFY_LOG` | Log filter, env_logger syntax (default `info`). Token-handling librespot modules are capped at `info`. |
| `BIGIFY_DESIGN_PREVIEW=1` | Sample-content mode; never connects to Spotify |
| `BIGIFY_START_SCREEN` | Open a given screen at start |
| `BIGIFY_WINDOW_SIZE=WxH` | Windowed at a fixed size (testing) |
| `BIGIFY_SCREENSHOT=path.png` | Capture after 3.5 s and quit |

## Files at runtime

| Path | Content |
|---|---|
| `$XDG_CONFIG_HOME/bigify/settings.toml` | Preferences |
| `$XDG_STATE_HOME/bigify/credentials.json` | Reusable Spotify credentials (mode 0600). Delete to sign out. |
| `$XDG_STATE_HOME/bigify/bigify.log` | Log (rotated at 4 MiB) |
| `$XDG_CACHE_HOME/bigify/audio` | librespot audio cache (≤ 1 GiB, safe to delete) |

## Manual test checklist

These need real hardware or accounts and cannot run in CI:

- [ ] Sign in from the Spotify app on Android and on iOS (same Wi-Fi)
- [ ] Browser sign-in in Steam Deck Desktop Mode; Cancel unblocks a retry
- [ ] Restart: signs in automatically from stored credentials
- [ ] Play an album, playlist (owned and followed), artist and Liked Songs
- [ ] Play/pause/next/previous/seek/shuffle/repeat/volume from the controller and from the phone; Bigify reflects both
- [ ] Pull the network cable mid-song: banner appears, playback recovers after reconnect
- [ ] Suspend and resume (Steam Deck): playback resumes or reconnects
- [ ] Unplug the audio device or switch the default sink
- [ ] Gaming Mode: controller works, there is no double input, Exit returns to Steam
- [ ] 1080p TV and 4K TV at 2–3 m: all text readable and focus always visible
- [ ] Large library (2000+ liked songs) scrolls smoothly
- [ ] `bigify.log` contains no tokens, credentials or search queries
