//! Colour maths: conversions, parsing/formatting and WCAG 2.x contrast.

/// An sRGB colour with straight (non-premultiplied) alpha.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rgba {
    pub rgb: [u8; 3],
    pub a: f32,
}

impl Rgba {
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { rgb: [r, g, b], a: 1.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Hex,
    Rgb,
    Hsl,
    Hsv,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Hex, Format::Rgb, Format::Hsl, Format::Hsv];

    pub fn label(self) -> &'static str {
        match self {
            Format::Hex => "HEX",
            Format::Rgb => "RGB",
            Format::Hsl => "HSL",
            Format::Hsv => "HSV",
        }
    }
}

/// Colour model used by the sliders.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Model {
    Rgb,
    Hsl,
    Hsv,
}

impl Model {
    pub const ALL: [Model; 3] = [Model::Rgb, Model::Hsl, Model::Hsv];

    pub fn label(self) -> &'static str {
        match self {
            Model::Rgb => "RGB",
            Model::Hsl => "HSL",
            Model::Hsv => "HSV",
        }
    }

    /// Short label and upper bound of each of the three channels.
    pub fn channels(self) -> [(&'static str, f32); 3] {
        match self {
            Model::Rgb => [("R", 255.0), ("G", 255.0), ("B", 255.0)],
            Model::Hsl => [("H", 360.0), ("S", 100.0), ("L", 100.0)],
            Model::Hsv => [("H", 360.0), ("S", 100.0), ("V", 100.0)],
        }
    }

    pub fn channels_of(self, rgb: [u8; 3]) -> [f32; 3] {
        match self {
            Model::Rgb => rgb.map(f32::from),
            Model::Hsl => rgb_to_hsl(rgb),
            Model::Hsv => rgb_to_hsv(rgb),
        }
    }

    pub fn to_rgb(self, v: [f32; 3]) -> [u8; 3] {
        match self {
            Model::Rgb => v.map(|c| c.round().clamp(0.0, 255.0) as u8),
            Model::Hsl => hsl_to_rgb(v),
            Model::Hsv => hsv_to_rgb(v),
        }
    }
}

fn unit(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|c| f32::from(c) / 255.0)
}

fn quantize(rgb: [f32; 3]) -> [u8; 3] {
    rgb.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Returns (hue in degrees, max, min, chroma).
fn hue_parts(rgb: [u8; 3]) -> (f32, f32, f32, f32) {
    let [r, g, b] = unit(rgb);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, max, min, d)
}

/// `[h 0..360, s 0..100, l 0..100]`
pub fn rgb_to_hsl(rgb: [u8; 3]) -> [f32; 3] {
    let (h, max, min, d) = hue_parts(rgb);
    let l = (max + min) / 2.0;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    [h, s * 100.0, l * 100.0]
}

/// `[h 0..360, s 0..100, v 0..100]`
pub fn rgb_to_hsv(rgb: [u8; 3]) -> [f32; 3] {
    let (h, max, _, d) = hue_parts(rgb);
    let s = if max == 0.0 { 0.0 } else { d / max };
    [h, s * 100.0, max * 100.0]
}

fn from_hue(h: f32, c: f32, m: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    quantize([r + m, g + m, b + m])
}

pub fn hsl_to_rgb([h, s, l]: [f32; 3]) -> [u8; 3] {
    let (s, l) = ((s / 100.0).clamp(0.0, 1.0), (l / 100.0).clamp(0.0, 1.0));
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    from_hue(h, c, l - c / 2.0)
}

pub fn hsv_to_rgb([h, s, v]: [f32; 3]) -> [u8; 3] {
    let (s, v) = ((s / 100.0).clamp(0.0, 1.0), (v / 100.0).clamp(0.0, 1.0));
    let c = v * s;
    from_hue(h, c, v - c)
}

/// The opaque colour seen when `fg` is drawn over `bg`.
pub fn blend(fg: Rgba, bg: [u8; 3]) -> [f32; 3] {
    let (f, b) = (unit(fg.rgb), unit(bg));
    let a = fg.a.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| f[i] * a + b[i] * (1.0 - a))
}

/// WCAG relative luminance of an sRGB colour with channels in 0..=1.
pub fn luminance(rgb: [f32; 3]) -> f32 {
    let [r, g, b] = rgb.map(|c| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio (1..=21) of `fg` composited over `bg`.
pub fn contrast(fg: Rgba, bg: [u8; 3]) -> f32 {
    let a = luminance(blend(fg, bg));
    let b = luminance(unit(bg));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Ratio text that never rounds up across a threshold (4.499 shows as 4.49).
pub fn ratio_text(ratio: f32) -> String {
    let t = (ratio * 100.0 + 1e-3).floor() / 100.0;
    let s = format!("{t:.2}");
    let s = s.strip_suffix('0').unwrap_or(&s);
    format!("{s}:1")
}

pub struct Wcag {
    pub aa_text: bool,
    pub aa_large: bool,
    pub aaa_text: bool,
    pub aaa_large: bool,
    pub non_text: bool,
}

impl Wcag {
    pub fn new(ratio: f32) -> Self {
        Self {
            aa_text: ratio >= 4.5,
            aa_large: ratio >= 3.0,
            aaa_text: ratio >= 7.0,
            aaa_large: ratio >= 4.5,
            non_text: ratio >= 3.0,
        }
    }
}

pub fn format(c: Rgba, fmt: Format) -> String {
    let [r, g, b] = c.rgb;
    let alpha = c.a < 1.0;
    let a = (c.a * 100.0).round() / 100.0;
    let suffix = if alpha { format!(", {a})") } else { ")".into() };
    let n = if alpha { "a" } else { "" };
    match fmt {
        Format::Hex if alpha => {
            format!("#{r:02X}{g:02X}{b:02X}{:02X}", (c.a * 255.0).round() as u8)
        }
        Format::Hex => format!("#{r:02X}{g:02X}{b:02X}"),
        Format::Rgb => format!("rgb{n}({r}, {g}, {b}{suffix}"),
        Format::Hsl => {
            let [h, s, l] = rgb_to_hsl(c.rgb).map(f32::round);
            format!("hsl{n}({h}, {s}%, {l}%{suffix}")
        }
        Format::Hsv => {
            let [h, s, v] = rgb_to_hsv(c.rgb).map(f32::round);
            format!("hsv{n}({h}, {s}%, {v}%{suffix}")
        }
    }
}

fn parse_hex(s: &str) -> Option<Rgba> {
    if !s.is_ascii() {
        return None;
    }
    let nib = |i: usize| u8::from_str_radix(&s[i..=i], 16).ok();
    let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    let (rgb, a) = match s.len() {
        3 | 4 => (
            [nib(0)? * 17, nib(1)? * 17, nib(2)? * 17],
            if s.len() == 4 { nib(3)? * 17 } else { 255 },
        ),
        6 | 8 => (
            [byte(0)?, byte(2)?, byte(4)?],
            if s.len() == 8 { byte(6)? } else { 255 },
        ),
        _ => return None,
    };
    Some(Rgba { rgb, a: f32::from(a) / 255.0 })
}

/// Parses hex (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `#` optional),
/// `rgb()/rgba()`, `hsl()/hsla()`, `hsv()/hsva()/hsb()` and bare `r, g, b`.
pub fn parse(input: &str) -> Option<Rgba> {
    let s = input.trim().to_ascii_lowercase();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(c) = parse_hex(&s) {
        return Some(c);
    }

    let (name, args) = match s.split_once('(') {
        Some((name, rest)) => (name.trim(), rest.trim().strip_suffix(')')?),
        None => ("rgb", s.as_str()),
    };
    // (value, was a percentage)
    let nums: Vec<(f32, bool)> = args
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| {
            let pct = t.ends_with('%');
            let t = t.trim_end_matches('%').trim_end_matches("deg");
            t.parse::<f32>().ok().filter(|v| v.is_finite()).map(|v| (v, pct))
        })
        .collect::<Option<_>>()?;
    if nums.len() != 3 && nums.len() != 4 {
        return None;
    }
    let a = match nums.get(3) {
        Some(&(v, pct)) => (if pct { v / 100.0 } else { v }).clamp(0.0, 1.0),
        None => 1.0,
    };
    let v = [nums[0], nums[1], nums[2]];
    let rgb = match name {
        "rgb" | "rgba" => v.map(|(v, pct)| {
            (if pct { v * 2.55 } else { v }).round().clamp(0.0, 255.0) as u8
        }),
        "hsl" | "hsla" => hsl_to_rgb(v.map(|(v, _)| v)),
        "hsv" | "hsva" | "hsb" | "hsba" => hsv_to_rgb(v.map(|(v, _)| v)),
        _ => return None,
    };
    Some(Rgba { rgb, a })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Rgba = Rgba::opaque(0, 0, 0);
    const WHITE: Rgba = Rgba::opaque(255, 255, 255);

    #[test]
    fn contrast_extremes() {
        assert!((contrast(BLACK, WHITE.rgb) - 21.0).abs() < 1e-3);
        assert!((contrast(WHITE, WHITE.rgb) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn contrast_matches_cca_screenshot() {
        // #57337A on #238391 is shown as 2.2:1 by CCA.
        let r = contrast(Rgba::opaque(0x57, 0x33, 0x7A), [0x23, 0x83, 0x91]);
        assert!((r - 2.2).abs() < 0.05, "{r}");
        assert_eq!(ratio_text(r), "2.18:1");
    }

    #[test]
    fn known_thresholds() {
        // #767676 on white is the classic "just passes AA" grey (4.54:1).
        let r = contrast(Rgba::opaque(0x76, 0x76, 0x76), WHITE.rgb);
        assert!((r - 4.54).abs() < 0.01, "{r}");
        let w = Wcag::new(r);
        assert!(w.aa_text && w.aa_large && w.aaa_large && w.non_text && !w.aaa_text);
        // #777777 just fails (4.48:1).
        let r = contrast(Rgba::opaque(0x77, 0x77, 0x77), WHITE.rgb);
        assert!(!Wcag::new(r).aa_text);
        assert_eq!(ratio_text(r), "4.47:1");
    }

    #[test]
    fn alpha_blends_over_background() {
        let half = Rgba { rgb: [0, 0, 0], a: 0.5 };
        let grey = contrast(Rgba::opaque(128, 128, 128), WHITE.rgb);
        assert!((contrast(half, WHITE.rgb) - grey).abs() < 0.03);
        assert!((contrast(Rgba { a: 0.0, ..BLACK }, WHITE.rgb) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ratio_text_formats() {
        assert_eq!(ratio_text(21.0), "21.0:1");
        assert_eq!(ratio_text(1.0), "1.0:1");
        assert_eq!(ratio_text(4.4999), "4.49:1");
        assert_eq!(ratio_text(4.5), "4.5:1");
    }

    #[test]
    fn hsl_hsv_round_trip_every_colour_coarsely() {
        for r in (0..=255).step_by(5) {
            for g in (0..=255).step_by(5) {
                for b in (0..=255).step_by(5) {
                    let rgb = [r as u8, g as u8, b as u8];
                    assert_eq!(hsl_to_rgb(rgb_to_hsl(rgb)), rgb);
                    assert_eq!(hsv_to_rgb(rgb_to_hsv(rgb)), rgb);
                }
            }
        }
    }

    #[test]
    fn known_conversions() {
        assert_eq!(rgb_to_hsl([255, 0, 0]), [0.0, 100.0, 50.0]);
        assert_eq!(rgb_to_hsv([0, 0, 255]), [240.0, 100.0, 100.0]);
        assert_eq!(hsl_to_rgb([120.0, 100.0, 25.0]), [0, 128, 0]);
        assert_eq!(hsv_to_rgb([360.0, 100.0, 100.0]), [255, 0, 0]);
    }

    #[test]
    fn parses_hex() {
        let c = Rgba::opaque(0x57, 0x33, 0x7A);
        assert_eq!(parse("#57337A"), Some(c));
        assert_eq!(parse(" 57337a "), Some(c));
        assert_eq!(parse("#fff"), Some(WHITE));
        assert_eq!(parse("000"), Some(BLACK));
        assert_eq!(parse("#00000080").unwrap().a, 128.0 / 255.0);
        assert_eq!(parse("#f008").unwrap().rgb, [255, 0, 0]);
        for bad in ["", "#", "#12", "#12345", "#gggggg", "#ééé", "nope"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn parses_functions() {
        let c = Rgba::opaque(87, 51, 122);
        assert_eq!(parse("rgb(87, 51, 122)"), Some(c));
        assert_eq!(parse("RGB(87 51 122)"), Some(c));
        assert_eq!(parse("87, 51, 122"), Some(c));
        assert_eq!(parse("rgba(87, 51, 122, 0.5)"), Some(Rgba { a: 0.5, ..c }));
        assert_eq!(parse("rgb(87 51 122 / 50%)"), Some(Rgba { a: 0.5, ..c }));
        assert_eq!(parse("rgb(100%, 0%, 0%)"), Some(Rgba::opaque(255, 0, 0)));
        assert_eq!(parse("hsl(120deg, 100%, 25%)"), Some(Rgba::opaque(0, 128, 0)));
        assert_eq!(parse("hsv(240, 100, 100)"), Some(Rgba::opaque(0, 0, 255)));
        assert_eq!(parse("rgb(300, -5, 0)"), Some(Rgba::opaque(255, 0, 0)));
        for bad in ["rgb(1, 2)", "rgb(1, 2, 3", "lab(1, 2, 3)", "rgb(a, b, c)", "1 2 3 4 5"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn format_round_trips_through_parse() {
        for c in [
            Rgba::opaque(0x57, 0x33, 0x7A),
            Rgba { rgb: [0x23, 0x83, 0x91], a: 0.5 },
        ] {
            for fmt in [Format::Hex, Format::Rgb] {
                let back = parse(&format(c, fmt)).unwrap();
                assert_eq!(back.rgb, c.rgb);
                assert!((back.a - c.a).abs() < 0.005);
            }
            for fmt in [Format::Hsl, Format::Hsv] {
                assert!(parse(&format(c, fmt)).is_some());
            }
        }
        assert_eq!(format(Rgba::opaque(87, 51, 122), Format::Hex), "#57337A");
        assert_eq!(format(Rgba::opaque(87, 51, 122), Format::Rgb), "rgb(87, 51, 122)");
        assert_eq!(format(Rgba::opaque(255, 0, 0), Format::Hsl), "hsl(0, 100%, 50%)");
        assert_eq!(
            format(Rgba { rgb: [255, 0, 0], a: 0.5 }, Format::Hsv),
            "hsva(0, 100%, 100%, 0.5)"
        );
    }
}
