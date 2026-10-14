# Controls and navigation

## Controller map

Controllers are read directly via evdev (gilrs). Labels below use Xbox
naming; PlayStation and Steam Deck buttons map by position.

| Input | Action | Keyboard equivalent |
|---|---|---|
| D-pad / left stick | Move focus (repeats while held) | Arrow keys |
| A | Select / activate | Enter, Space |
| B | Back | Esc |
| X | Play / pause (anywhere) | Media Play/Pause |
| Y | More: add the focused track to the queue | Menu key |
| LB / RB | Previous / next track | Media Previous / Next |
| LT / RT | Volume down / up | Volume keys |
| Start (☰) | Open / close Now Playing | F2 |
| Select / View (⧉) | Open / close Up next (queue) | F3 |
| Guide / Steam | Not used; left to the system | — |

The mouse works everywhere (click to select) but is never required.

## Steam Input

- **Gaming Mode:** Steam exposes the Deck controls (or an attached controller)
  as a virtual Xbox controller, which Bigify reads directly. Use the
  "Gamepad" controller template for Bigify in Steam.
- **Desktop templates** (keyboard/mouse emulation) also work, because every
  action has a keyboard equivalent. If buttons then trigger twice, turn off
  *Settings → Controls → Read controllers directly*.
- On-screen text entry: Search has its own D-pad keyboard. Steam's keyboard
  (Steam + X) and physical keyboards type straight into the search field.

## Navigation model

- **Rail → content.** Moving focus over Home, Search, Library or Settings
  switches the section immediately, as on a TV. **Right** or **A** moves
  into the content. "Playing" opens only on Right/A.
- **Content → rail.** Pressing **Left** at the start of a row (or in a
  column without a left neighbour) returns to the rail.
- **Rows and grids.** Arrows move within a row. Up and Down move between
  rows, and each row remembers its position when you come back.
- **Back (B / Esc)** closes the topmost layer in this order: queue, Now
  Playing, album/playlist page, content (to the rail), section (to Home).
  Back never exits the app; use *Settings → Exit Bigify*.
- **Overlays** (Now Playing, queue) remember what had focus and restore it
  when closed.
- **Mini player.** Down from the bottom of a page focuses the mini player;
  A opens Now Playing; Up returns to the page.

## Implementation notes (for contributors)

- Controllers become Qt key events (`src/input.rs` → `cpp/input.cpp`), so all
  navigation logic is QML keyboard handling.
- Every focusable component draws `FocusRing` (a 4 px light ring with a dark
  inner halo) and scales up slightly. Don't use a hover-only focus style.
- Lists consume arrow keys only while they can move; unhandled keys bubble to
  parent `Keys` handlers. To link sections explicitly, use
  `KeyNavigation.*`, or call `forceActiveFocus()` in an `onXPressed` handler.
- Every page is a `FocusScope`. A child that should receive focus inside a
  scope must set `focus: true`, and list delegates use
  `focus: ListView.isCurrentItem`.
