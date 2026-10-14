# ADR 0001 — Qt Quick (QML) UI with a Rust core via cxx-qt

**Status:** accepted · 2026-10-14

## Context

Bigify needs a fluid, GPU-accelerated 10-foot UI with Material 3 Expressive
motion (shape morphing, springs, blur, wavy progress), strong keyboard and
focus support, low overhead on Steam Deck-class hardware, and first-class
integration with librespot, which is a Rust library.

## Options considered

| Option | Strengths | Weaknesses |
|---|---|---|
| **Qt Quick 6 + Rust via cxx-qt** | Mature scene graph, shaders and `MultiEffect` blur/shadow, `QtQuick.Shapes` curve renderer, `FocusScope`/`KeyNavigation`, accessibility, well proven on embedded and TV | Three toolchains (Rust, C++, Qt); larger runtime (KDE Flatpak runtime) |
| Slint (pure Rust) | One `cargo build`, small footprint, GPL/royalty-free licensing | No runtime blur or shaders; younger focus and animation system; fewer ready-made effects for expressive motion |
| GTK 4 + libadwaita (gtk-rs) | Native GNOME look, Rust bindings | Desktop-oriented widgets and focus model; Adwaita styling fights a custom M3 Expressive look |
| Web stack (Tauri/Electron) | Fast iteration | Higher memory and CPU, extra browser runtime; poor fit for SteamOS Gaming Mode |

## Decision

Qt Quick 6.8+ for the UI, written in QML. All application logic lives in
Rust crates, bridged with cxx-qt 0.10. Qt Quick Controls are **not** used:
Bigify's components are built on plain Qt Quick so the Material 3 Expressive
design system is fully under our control and no Android or Material runtime
is pulled in.

## Consequences

- The minimum Qt is 6.8 (Debian 13, Fedora 41+, Arch, KDE Flatpak runtime 6.8).
  APIs newer than that (e.g. `RectangularShadow`, Qt 6.9) are avoided.
- Contributors need Qt dev packages; `ci/Dockerfile` provides a reproducible
  environment.
- Qt 6 has no gamepad module, so controllers are read in Rust (gilrs) and
  injected as key events.
- Known Qt 6.8 pitfalls are documented in `qml/theme/Theme.qml`: `Behavior`
  inside QML singletons crashes, and QML reserves `onFoo` property names.
