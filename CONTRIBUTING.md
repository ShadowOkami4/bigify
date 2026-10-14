# Contributing to Bigify

Thanks for helping. Bigify aims to be the nicest way to play music on a Linux
TV, so correctness, usability and design quality come before feature count.

## Before you start

- Read [ARCHITECTURE.md](docs/ARCHITECTURE.md) and
  [LIMITATIONS.md](docs/LIMITATIONS.md). Features must be backed by an
  interface that really provides them. Never fabricate Spotify data or
  pretend an operation succeeded.
- For larger changes, open an issue first to agree on the approach.

## Ground rules

- **Layering:** `bigify-core` has no Qt and no network; `bigify-spotify` has
  no Qt; QML talks only to the `Backend` singleton.
- **Confirmed state:** UI state changes only on engine events. Requests go
  through `PlaybackState::request` and show as pending.
- **Controller first:** every new screen must be fully usable with D-pad, A
  and B, show a visible `FocusRing`, and restore focus on return. Test with
  the keyboard arrows, Enter and Esc.
- **Design tokens:** use `Theme.*` for every size, color, radius and
  duration; no raw numbers in QML. Use `Theme.dp()` for any new dimension.
- **Qt 6.8 compatibility:** no APIs newer than 6.8; no `Behavior` inside QML
  singletons; no properties named `onSomething`.
- **No secrets:** never log tokens, credentials or search queries, and never
  commit credentials.
- **Dependencies:** add one only with a clear reason, stated in the PR.

## Workflow

```sh
ci/run.sh cargo fmt --all
ci/run.sh cargo clippy --workspace --all-targets   # must be warning-free
ci/run.sh cargo test --workspace
ci/run.sh ci/screenshot.sh <screen> 1920x1080       # attach for UI changes
```

UI changes should include screenshots at 1080p, plus 4K or the Deck's
1280×800 when the layout changes.

## Reporting bugs

Include your distribution or SteamOS version, Bigify version, Desktop or
Gaming Mode, controller model, and `~/.local/state/bigify/bigify.log`. The
log contains no credentials, but skim it before posting.

Report security issues privately to the maintainers instead of filing a
public issue.

## License

By contributing, you agree that your contributions are licensed under
GPL-3.0-or-later.
