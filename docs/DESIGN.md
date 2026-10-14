# Design system

Bigify follows **Material 3 Expressive**, adapted for viewing from a sofa
2–4 m away. Material 3 is used as a visual and interaction specification;
no Android or Material runtime is involved. All tokens live in
`crates/bigify-app/qml/theme/Theme.qml`.

## Canvas and scale

- Everything is authored for a **1920 × 1080** logical canvas. `Theme.dp(x)`
  scales by `min(height/1080, width/1920) × user scale`, so 720p, 1080p, 4K
  and the 16:10 Steam Deck screen keep identical proportions.
- Title-safe margins: 64 px horizontal, 48 px vertical (`safeH`, `safeV`).
- Minimum text size is 18 px at 1080p.

## Color

Material 3 color roles, generated in Rust (`bigify_core::color`):

- **Brand seed** `#7C5CFF` (electric violet). It is deliberately not Spotify
  green, so Bigify never looks like an official Spotify product.
- **Dynamic color.** A seed is extracted from the current artwork (hue
  buckets weighted by chroma). Monochrome artwork keeps the brand scheme
  rather than inventing an accent.
- **Tones** use CIELAB L* (the M3 "tone"), with hue and chroma in OKLCH and
  gamut mapping that reduces chroma. Accent chroma is clamped so artwork can
  neither wash out nor overpower the UI.
- **Contrast** is unit-tested: body text ≥ 7:1, secondary text and accents
  ≥ 4.5:1 on every surface level, and the focus ring ≥ 7:1.
- Scheme changes cross-fade over 900 ms (instant with reduced motion).
- QML reserves `onFoo` names, so M3 "on-X" roles are named `XContent`:
  `onSurface` → `surfaceContent`, `onPrimary` → `primaryContent`, and so on.

Surfaces: `surface` (T6), then `surfaceContainerLowest/Low/…/Highest`
(T4–T22) for layering. Depth comes from tonal contrast; shadows are used
only for Now Playing artwork.

## Typography

M3 type scale enlarged about 1.4× for distance, with Expressive emphasis
(bold display and headlines). Font preference: Google Sans Flex, Roboto
Flex, Roboto, Inter, Noto Sans, Cantarell, then the system default.

| Role | px @1080p |
|---|---|
| displayLarge / Medium / Small | 88 / 64 / 52 |
| headlineLarge / Medium / Small | 44 / 38 / 32 |
| titleLarge / Medium / Small | 30 / 25 / 22 |
| bodyLarge / Medium | 24 / 21 |
| labelLarge / Medium / Small | 22 / 19 / 18 |

## Shape

The M3 Expressive corner scale is 4, 8, 12, 16, 20, 28, 32 and 48 px, plus
"full" pills. Expressive shape changes:

- Play/pause: a circle when paused, a rounded square when playing.
- Toggle buttons (shuffle/repeat) become rounded squares when on.
- Pills flatten slightly when pressed, and selected chips square up.
- The nav rail indicator stretches into a pill.

## Motion

| Token | Duration | Use |
|---|---|---|
| quick | 150 ms | press and focus scale |
| medium | 300 ms | shape morphs, indicators |
| slow | 500 ms | list scrolling, sheets |
| fade / fadeLong | 200 / 350 ms | opacity |

Spatial motion uses a gentle back-out overshoot (an approximation of M3
Expressive "fast spatial" springs). Effects such as color and opacity never
overshoot. With **Reduce motion** on, spatial durations drop to 0, focus no
longer scales, the loading indicator becomes a pulsing dot, and the wave in
the progress bar flattens.

## Focus

Focus is the most important state on a TV:

- A 4 px light ring in the `focusColor` role, offset 4 px, with a thin dark
  inner halo so it stays visible on any artwork.
- Focused items scale to 1.06.
- Titles of focused cards become bold, and focused rows get a highlighted
  surface.

## Signature components

| Component | Notes |
|---|---|
| `PlayPauseButton` | 128 px, shape morph, loading ring while a request is unconfirmed |
| `WavyProgress` | Waves while playing, flat when paused; focusable for ±10 s seeking |
| `LoadingIndicator` | Morphing scalloped polygon (M3 Expressive loading indicator) |
| `MediaCard` | 248 px artwork, circle for artists, lift and ring on focus |
| `NavRail` | M3 rail with pill indicator |
| `OnScreenKeyboard` | Alphabetical grid, faster than QWERTY with a D-pad |

## Icons

Original 24-px-grid stroke icons (`components/Icon.qml`) with 2.2 px rounded
strokes, rendered as vector paths so they stay sharp at 4K.
