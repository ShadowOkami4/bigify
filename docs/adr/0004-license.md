# ADR 0004 — License: GPL-3.0-or-later

**Status:** accepted · 2026-10-14

Dependencies and their licenses:

- librespot: MIT
- cxx and cxx-qt: MIT OR Apache-2.0
- Qt 6: LGPL-3.0, dynamically linked
- gilrs: MIT OR Apache-2.0
- tokio, serde, image: MIT and/or Apache-2.0

All are compatible with GPL-3.0. Bigify is licensed **GPL-3.0-or-later** so
that redistributed builds and forks of a user-facing application stay open.
Qt is used under the LGPL through dynamic linking, which Flatpak and distro
packages satisfy.

Bigify's icons are original vector paths drawn for the project, and no fonts
are bundled. No third-party artwork ships in the repository.
