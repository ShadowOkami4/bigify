//! Dynamic color: derive a Material 3–style dark color scheme from artwork.
//!
//! Material's reference implementation uses the HCT color space. This module
//! uses the same *tone* definition (CIELAB L*, which is what makes M3 tone
//! pairs meet contrast targets) combined with OKLCH hue/chroma, which is
//! perceptually uniform enough for picking accents and is far smaller to
//! implement. Every generated role is gamut-mapped to sRGB by reducing chroma,
//! never by clipping, so hues stay stable.
//!
//! Text/background role pairs are verified against WCAG contrast in tests.

use serde::Serialize;

/// Bigify's brand seed: an electric violet. Deliberately not Spotify green so
/// the app never looks like an official Spotify product.
pub const BRAND_SEED: Rgb = Rgb(0x7C, 0x5C, 0xFF);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    fn to_linear(self) -> [f64; 3] {
        [self.0, self.1, self.2].map(|c| srgb_to_linear(c as f64 / 255.0))
    }

    /// WCAG relative luminance.
    pub fn luminance(self) -> f64 {
        let [r, g, b] = self.to_linear();
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// WCAG 2.x contrast ratio between two colors (1.0 ..= 21.0).
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// OKLCH color: lightness 0..1, chroma ≥ 0, hue in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

impl Oklch {
    pub fn from_rgb(rgb: Rgb) -> Self {
        let [r, g, b] = rgb.to_linear();
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        let ok_l = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
        let a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s;
        let bb = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s;
        let c = (a * a + bb * bb).sqrt();
        let h = bb.atan2(a).to_degrees().rem_euclid(360.0);
        Oklch { l: ok_l, c, h }
    }

    /// Linear sRGB components; may be outside 0..1 when out of gamut.
    fn to_linear(self) -> [f64; 3] {
        let (a, b) = (
            self.c * self.h.to_radians().cos(),
            self.c * self.h.to_radians().sin(),
        );
        let l = self.l + 0.3963377774 * a + 0.2158037573 * b;
        let m = self.l - 0.1055613458 * a - 0.0638541728 * b;
        let s = self.l - 0.0894841775 * a - 1.2914855480 * b;
        let (l, m, s) = (l * l * l, m * m * m, s * s * s);
        [
            4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
        ]
    }

    fn in_gamut(self) -> bool {
        const EPS: f64 = 1e-6;
        self.to_linear()
            .iter()
            .all(|&c| (-EPS..=1.0 + EPS).contains(&c))
    }

    fn to_rgb(self) -> Rgb {
        let [r, g, b] = self
            .to_linear()
            .map(|c| (linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0).round() as u8);
        Rgb(r, g, b)
    }

    /// CIELAB L* of this color (the M3 "tone").
    fn tone(self) -> f64 {
        let [r, g, b] = self.to_linear().map(|c| c.clamp(0.0, 1.0));
        y_to_lstar(0.2126 * r + 0.7152 * g + 0.0722 * b)
    }
}

fn y_to_lstar(y: f64) -> f64 {
    let e = 216.0 / 24389.0;
    let k = 24389.0 / 27.0;
    if y <= e {
        k * y
    } else {
        116.0 * y.cbrt() - 16.0
    }
}

/// Produce the in-gamut color with the given hue, the requested chroma (or
/// as much as fits) and the given tone (CIELAB L*, 0..100).
pub fn color_at_tone(hue: f64, chroma: f64, tone: f64) -> Rgb {
    let tone = tone.clamp(0.0, 100.0);
    let solve_l = |c: f64| {
        // Tone increases monotonically with OKLab L for fixed hue/chroma.
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for _ in 0..40 {
            let mid = (lo + hi) / 2.0;
            if (Oklch { l: mid, c, h: hue }).tone() < tone {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Oklch {
            l: (lo + hi) / 2.0,
            c,
            h: hue,
        }
    };
    let full = solve_l(chroma);
    if full.in_gamut() {
        return full.to_rgb();
    }
    // Reduce chroma until the color fits sRGB.
    let (mut lo, mut hi) = (0.0_f64, chroma);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if solve_l(mid).in_gamut() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    solve_l(lo).to_rgb()
}

/// The color roles QML uses. Names follow the Material 3 role names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scheme {
    pub seed: String,
    pub primary: String,
    pub on_primary: String,
    pub primary_container: String,
    pub on_primary_container: String,
    pub secondary: String,
    pub secondary_container: String,
    pub on_secondary_container: String,
    pub tertiary: String,
    pub tertiary_container: String,
    pub on_tertiary_container: String,
    pub surface: String,
    pub surface_container_lowest: String,
    pub surface_container_low: String,
    pub surface_container: String,
    pub surface_container_high: String,
    pub surface_container_highest: String,
    pub on_surface: String,
    pub on_surface_variant: String,
    pub outline: String,
    pub outline_variant: String,
    pub error: String,
    pub on_error: String,
    /// High-visibility focus ring color.
    pub focus: String,
}

/// Raw role colors, for tests and for building [`Scheme`].
#[derive(Debug, Clone, Copy)]
struct Roles {
    primary: Rgb,
    on_primary: Rgb,
    primary_container: Rgb,
    on_primary_container: Rgb,
    secondary: Rgb,
    secondary_container: Rgb,
    on_secondary_container: Rgb,
    tertiary: Rgb,
    tertiary_container: Rgb,
    on_tertiary_container: Rgb,
    surface: Rgb,
    surface_lowest: Rgb,
    surface_low: Rgb,
    surface_mid: Rgb,
    surface_high: Rgb,
    surface_highest: Rgb,
    on_surface: Rgb,
    on_surface_variant: Rgb,
    outline: Rgb,
    outline_variant: Rgb,
    focus: Rgb,
}

fn dark_roles(seed: Rgb) -> Roles {
    let s = Oklch::from_rgb(seed);
    let h = s.h;
    // Keep accents vivid but bounded so washed-out artwork still yields a
    // usable accent and neon artwork does not produce garish UI.
    let accent_c = s.c.clamp(0.09, 0.16);
    let secondary_c = (accent_c * 0.35).max(0.03);
    let tertiary_h = (h + 60.0).rem_euclid(360.0);
    let neutral_c = 0.012;
    let neutral_variant_c = 0.025;
    let p = |t| color_at_tone(h, accent_c, t);
    let sec = |t| color_at_tone(h, secondary_c, t);
    let ter = |t| color_at_tone(tertiary_h, accent_c * 0.8, t);
    let n = |t| color_at_tone(h, neutral_c, t);
    let nv = |t| color_at_tone(h, neutral_variant_c, t);
    Roles {
        primary: p(80.0),
        on_primary: p(20.0),
        primary_container: p(30.0),
        on_primary_container: p(90.0),
        secondary: sec(80.0),
        secondary_container: sec(30.0),
        on_secondary_container: sec(90.0),
        tertiary: ter(80.0),
        tertiary_container: ter(30.0),
        on_tertiary_container: ter(90.0),
        surface: n(6.0),
        surface_lowest: n(4.0),
        surface_low: n(10.0),
        surface_mid: n(12.0),
        surface_high: n(17.0),
        surface_highest: n(22.0),
        on_surface: n(92.0),
        on_surface_variant: nv(80.0),
        outline: nv(60.0),
        outline_variant: nv(30.0),
        // Very light primary: must stand out against every surface and
        // against artwork; paired with a dark inner halo in QML.
        focus: p(92.0),
    }
}

impl Scheme {
    pub fn dark_from_seed(seed: Rgb) -> Self {
        let r = dark_roles(seed);
        Scheme {
            seed: seed.hex(),
            primary: r.primary.hex(),
            on_primary: r.on_primary.hex(),
            primary_container: r.primary_container.hex(),
            on_primary_container: r.on_primary_container.hex(),
            secondary: r.secondary.hex(),
            secondary_container: r.secondary_container.hex(),
            on_secondary_container: r.on_secondary_container.hex(),
            tertiary: r.tertiary.hex(),
            tertiary_container: r.tertiary_container.hex(),
            on_tertiary_container: r.on_tertiary_container.hex(),
            surface: r.surface.hex(),
            surface_container_lowest: r.surface_lowest.hex(),
            surface_container_low: r.surface_low.hex(),
            surface_container: r.surface_mid.hex(),
            surface_container_high: r.surface_high.hex(),
            surface_container_highest: r.surface_highest.hex(),
            on_surface: r.on_surface.hex(),
            on_surface_variant: r.on_surface_variant.hex(),
            outline: r.outline.hex(),
            outline_variant: r.outline_variant.hex(),
            error: "#ffb4ab".into(),
            on_error: "#690005".into(),
            focus: r.focus.hex(),
        }
    }

    pub fn brand() -> Self {
        Self::dark_from_seed(BRAND_SEED)
    }
}

/// Pick a seed color from artwork pixels (packed RGB, 3 bytes per pixel).
///
/// Pixels are grouped into 24 hue buckets; a bucket's score favours both
/// coverage and colorfulness, similar in spirit to Material's `Score`.
/// Returns `None` for near-monochrome artwork so the caller can keep the
/// brand palette instead of inventing a color.
pub fn seed_from_pixels(rgb: &[u8]) -> Option<Rgb> {
    const BUCKETS: usize = 24;
    #[derive(Default, Clone, Copy)]
    struct Bucket {
        weight: f64,
        count: u32,
        sum: [f64; 3],
    }
    let mut buckets = [Bucket::default(); BUCKETS];
    let mut total = 0u32;
    for px in rgb.chunks_exact(3) {
        total += 1;
        let c = Rgb(px[0], px[1], px[2]);
        let lch = Oklch::from_rgb(c);
        if lch.c < 0.04 || !(0.25..=0.92).contains(&lch.l) {
            continue;
        }
        let idx = ((lch.h / 360.0 * BUCKETS as f64) as usize).min(BUCKETS - 1);
        let b = &mut buckets[idx];
        b.weight += lch.c;
        b.count += 1;
        b.sum[0] += px[0] as f64;
        b.sum[1] += px[1] as f64;
        b.sum[2] += px[2] as f64;
    }
    if total == 0 {
        return None;
    }
    let best = buckets
        .iter()
        .filter(|b| b.count > 0)
        .max_by(|a, b| a.weight.total_cmp(&b.weight))?;
    // Ignore accents that cover less than 1.5% of the image.
    if (best.count as f64) < total as f64 * 0.015 {
        return None;
    }
    let n = best.count as f64;
    Some(Rgb(
        (best.sum[0] / n).round() as u8,
        (best.sum[1] / n).round() as u8,
        (best.sum[2] / n).round() as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_contrast(fg: Rgb, bg: Rgb, min: f64, what: &str) {
        let ratio = contrast_ratio(fg, bg);
        assert!(
            ratio >= min,
            "{what}: {} on {} has contrast {ratio:.2} < {min}",
            fg.hex(),
            bg.hex()
        );
    }

    #[test]
    fn oklch_round_trip() {
        for c in [
            Rgb(255, 0, 0),
            Rgb(12, 200, 90),
            Rgb(30, 30, 30),
            BRAND_SEED,
        ] {
            assert_eq!(Oklch::from_rgb(c).to_rgb(), c);
        }
    }

    #[test]
    fn tones_hit_target_lightness() {
        let rgb = color_at_tone(250.0, 0.12, 50.0);
        let lstar = y_to_lstar(rgb.luminance());
        assert!((lstar - 50.0).abs() < 1.0, "got L* {lstar}");
    }

    #[test]
    fn generated_schemes_meet_contrast_for_many_seeds() {
        let seeds = (0..36).map(|i| {
            let h = i as f64 * 10.0;
            color_at_tone(h, 0.2, 55.0)
        });
        for seed in seeds.chain([Rgb(128, 128, 128), Rgb(255, 255, 0), Rgb(0, 0, 255)]) {
            let r = dark_roles(seed);
            for (bg, name) in [
                (r.surface, "surface"),
                (r.surface_low, "surfaceLow"),
                (r.surface_mid, "surfaceMid"),
                (r.surface_high, "surfaceHigh"),
                (r.surface_highest, "surfaceHighest"),
            ] {
                assert_contrast(r.on_surface, bg, 7.0, name);
                assert_contrast(r.on_surface_variant, bg, 4.5, name);
                assert_contrast(r.primary, bg, 4.5, name);
                assert_contrast(r.focus, bg, 7.0, name);
            }
            assert_contrast(r.on_primary, r.primary, 4.5, "onPrimary");
            assert_contrast(r.on_primary_container, r.primary_container, 4.5, "onPC");
            assert_contrast(r.on_secondary_container, r.secondary_container, 4.5, "onSC");
            assert_contrast(r.on_tertiary_container, r.tertiary_container, 4.5, "onTC");
        }
    }

    #[test]
    fn seed_ignores_monochrome_art() {
        let gray: Vec<u8> = (0..1000).flat_map(|i| [(i % 256) as u8; 3]).collect();
        assert_eq!(seed_from_pixels(&gray), None);
    }

    #[test]
    fn seed_prefers_colorful_region() {
        let mut px = Vec::new();
        for _ in 0..900 {
            px.extend_from_slice(&[20, 20, 20]); // dark background
        }
        for _ in 0..100 {
            px.extend_from_slice(&[230, 60, 40]); // red accent
        }
        let seed = seed_from_pixels(&px).expect("accent found");
        assert_eq!(seed, Rgb(230, 60, 40));
    }
}
