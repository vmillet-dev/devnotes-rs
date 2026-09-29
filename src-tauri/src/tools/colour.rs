//! Colours in HEX, RGB, HSL and OKLCH, and the WCAG contrast between two of them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// sRGB, each channel 0–1 — or past it, for an OKLCH colour sRGB cannot show.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Notation {
    Hex,
    Rgb,
    Hsl,
    Oklch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Notations {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
    pub oklch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ColourReading {
    Read {
        given: Notation,
        notations: Notations,
        /// What the window paints: in sRGB, always.
        swatch: String,
        /// An OKLCH colour sRGB cannot show, brought inside at the same lightness and hue.
        out_of_gamut: bool,
    },
    Invalid,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextSizes {
    pub normal: bool,
    pub large: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Contrast {
    pub ratio: f64,
    pub aa: TextSizes,
    pub aaa: TextSizes,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColourRequest {
    pub colour: String,
    /// The background the contrast is measured on; empty for none.
    pub against: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColourAnswer {
    pub colour: ColourReading,
    pub against: ColourReading,
    pub contrast: Option<Contrast>,
}

// --- Parsing: liberal on the way in, CSS Color 4 on the way out.

fn hex_channel(text: &str) -> Option<f64> {
    u8::from_str_radix(text, 16)
        .ok()
        .map(|value| f64::from(value) / 255.0)
}

fn parse_hex(text: &str) -> Option<Rgba> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let long: String = match digits.len() {
        3 | 4 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return None,
    };
    let channel = |index: usize| hex_channel(&long[index * 2..index * 2 + 2]);
    Some(Rgba {
        r: channel(0)?,
        g: channel(1)?,
        b: channel(2)?,
        a: if long.len() == 8 { channel(3)? } else { 1.0 },
    })
}

/// A number, a percentage of `whole`, or an angle in degrees.
fn value(text: &str, whole: f64) -> Option<f64> {
    if let Some(percent) = text.strip_suffix('%') {
        return percent
            .parse::<f64>()
            .ok()
            .map(|value| value / 100.0 * whole);
    }
    let text = text.strip_suffix("deg").unwrap_or(text);
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn alpha(text: Option<&&str>) -> Option<f64> {
    text.map_or(Some(1.0), |text| value(text, 1.0))
        .map(|alpha| alpha.clamp(0.0, 1.0))
}

fn parse_function(text: &str) -> Option<(Notation, Rgba)> {
    let (name, rest) = text.split_once('(')?;
    let inside = rest.strip_suffix(')')?;
    let parts: Vec<&str> = inside
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if !(3..=4).contains(&parts.len()) {
        return None;
    }

    match name.trim() {
        "rgb" | "rgba" => Some((
            Notation::Rgb,
            Rgba {
                r: value(parts[0], 255.0)? / 255.0,
                g: value(parts[1], 255.0)? / 255.0,
                b: value(parts[2], 255.0)? / 255.0,
                a: alpha(parts.get(3))?,
            },
        )),
        "hsl" | "hsla" => {
            let (r, g, b) = hsl_to_rgb(
                value(parts[0], 360.0)?,
                value(parts[1], 1.0)?,
                value(parts[2], 1.0)?,
            );
            Some((
                Notation::Hsl,
                Rgba {
                    r,
                    g,
                    b,
                    a: alpha(parts.get(3))?,
                },
            ))
        }
        "oklch" => {
            // Lightness 0–1 or a percentage; chroma's 100% is 0.4, as CSS has it.
            let (r, g, b) = oklch_to_rgb(
                value(parts[0], 1.0)?,
                value(parts[1], 0.4)?,
                value(parts[2], 360.0)?,
            );
            Some((
                Notation::Oklch,
                Rgba {
                    r,
                    g,
                    b,
                    a: alpha(parts.get(3))?,
                },
            ))
        }
        _ => None,
    }
}

fn parse(text: &str) -> Option<(Notation, Rgba)> {
    let text = text.trim().to_ascii_lowercase();
    if text.contains('(') {
        parse_function(&text)
    } else {
        parse_hex(&text).map(|rgba| (Notation::Hex, rgba))
    }
}

// --- Spaces.

fn to_linear(channel: f64) -> f64 {
    if channel.abs() <= 0.04045 {
        channel / 12.92
    } else {
        channel.signum() * ((channel.abs() + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(channel: f64) -> f64 {
    if channel.abs() <= 0.003_130_8 {
        channel * 12.92
    } else {
        channel.signum() * (1.055 * channel.abs().powf(1.0 / 2.4) - 0.055)
    }
}

/// Björn Ottosson's `OKLab`, through the cone responses of linear sRGB.
fn rgb_to_oklab(red: f64, green: f64, blue: f64) -> (f64, f64, f64) {
    let (red, green, blue) = (to_linear(red), to_linear(green), to_linear(blue));
    let long = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
    let medium = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
    let short = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();
    (
        0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short,
        1.977_998_495_1 * long - 2.428_592_205_0 * medium + 0.450_593_709_9 * short,
        0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766_0 * short,
    )
}

fn oklab_to_rgb(lightness: f64, green_red: f64, blue_yellow: f64) -> (f64, f64, f64) {
    let long = (lightness + 0.396_337_777_4 * green_red + 0.215_803_757_3 * blue_yellow).powi(3);
    let medium = (lightness - 0.105_561_345_8 * green_red - 0.063_854_172_8 * blue_yellow).powi(3);
    let short = (lightness - 0.089_484_177_5 * green_red - 1.291_485_548_0 * blue_yellow).powi(3);
    (
        from_linear(4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short),
        from_linear(-1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short),
        from_linear(-0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short),
    )
}

fn oklch_to_rgb(lightness: f64, chroma: f64, hue: f64) -> (f64, f64, f64) {
    let radians = hue.to_radians();
    oklab_to_rgb(lightness, chroma * radians.cos(), chroma * radians.sin())
}

fn rgb_to_oklch(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (lightness, a, b) = rgb_to_oklab(r, g, b);
    let chroma = a.hypot(b);
    (lightness, chroma, b.atan2(a).to_degrees().rem_euclid(360.0))
}

fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
    let (saturation, lightness) = (saturation.clamp(0.0, 1.0), lightness.clamp(0.0, 1.0));
    let channel = |n: f64| {
        let k = (n + hue.rem_euclid(360.0) / 30.0).rem_euclid(12.0);
        let a = saturation * lightness.min(1.0 - lightness);
        lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    (channel(0.0), channel(8.0), channel(4.0))
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let lightness = f64::midpoint(max, min);
    let delta = max - min;
    if delta < 1e-9 {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if (max - r).abs() < f64::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (hue * 60.0, saturation, lightness)
}

fn in_gamut(r: f64, g: f64, b: f64) -> bool {
    const SLACK: f64 = 1e-4;
    [r, g, b]
        .iter()
        .all(|&channel| (-SLACK..=1.0 + SLACK).contains(&channel))
}

/// Chroma given up until sRGB can show it, at the same lightness and hue: the colour visibly
/// moves, which is the point — it is said, not hidden.
fn into_gamut(colour: Rgba) -> Rgba {
    if in_gamut(colour.r, colour.g, colour.b) {
        return colour;
    }
    let (lightness, chroma, hue) = rgb_to_oklch(colour.r, colour.g, colour.b);
    let (mut low, mut high) = (0.0, chroma);
    for _ in 0..30 {
        let middle = f64::midpoint(low, high);
        let (r, g, b) = oklch_to_rgb(lightness, middle, hue);
        if in_gamut(r, g, b) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let (r, g, b) = oklch_to_rgb(lightness, low, hue);
    Rgba {
        r: r.clamp(0.0, 1.0),
        g: g.clamp(0.0, 1.0),
        b: b.clamp(0.0, 1.0),
        a: colour.a,
    }
}

// --- Writing.

/// At most `decimals` places, and none that are zeros.
fn number(value: f64, decimals: i32) -> String {
    let scale = 10f64.powi(decimals);
    let rounded = (value * scale).round() / scale;
    let text = format!(
        "{rounded:.prec$}",
        prec = usize::try_from(decimals).unwrap_or(0)
    );
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    };
    if text == "-0" { "0".to_owned() } else { text }
}

fn byte(channel: f64) -> u8 {
    // Clamped first, so the cast can neither wrap nor truncate a sign away.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    value
}

fn alpha_suffix(alpha: f64) -> String {
    if alpha >= 1.0 {
        String::new()
    } else {
        format!(" / {}%", number(alpha * 100.0, 0))
    }
}

fn write_hex(colour: Rgba) -> String {
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        byte(colour.r),
        byte(colour.g),
        byte(colour.b)
    );
    if colour.a < 1.0 {
        format!("{hex}{:02x}", byte(colour.a))
    } else {
        hex
    }
}

fn notations(shown: Rgba, oklch_of: Rgba) -> Notations {
    let (hue, saturation, lightness) = rgb_to_hsl(shown.r, shown.g, shown.b);
    let (ok_lightness, chroma, ok_hue) = rgb_to_oklch(oklch_of.r, oklch_of.g, oklch_of.b);
    let alpha = alpha_suffix(shown.a);
    Notations {
        hex: write_hex(shown),
        rgb: format!(
            "rgb({} {} {}{alpha})",
            byte(shown.r),
            byte(shown.g),
            byte(shown.b)
        ),
        hsl: format!(
            "hsl({} {}% {}%{alpha})",
            number(hue, 1),
            number(saturation * 100.0, 1),
            number(lightness * 100.0, 1)
        ),
        oklch: format!(
            "oklch({}% {} {}{alpha})",
            number(ok_lightness * 100.0, 1),
            number(chroma, 3),
            if chroma < 0.0005 {
                "0".to_owned()
            } else {
                number(ok_hue, 1)
            }
        ),
    }
}

fn read(text: &str) -> (ColourReading, Option<Rgba>) {
    if text.trim().is_empty() {
        return (ColourReading::Empty, None);
    }
    let Some((given, colour)) = parse(text) else {
        return (ColourReading::Invalid, None);
    };
    let shown = into_gamut(colour);
    (
        ColourReading::Read {
            given,
            notations: notations(shown, colour),
            swatch: write_hex(shown),
            out_of_gamut: shown != colour,
        },
        Some(shown),
    )
}

// --- Contrast, as WCAG 2 and `scripts/palette.test.mjs` compute it.

fn luminance(colour: Rgba) -> f64 {
    let channel = |value: f64| {
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(colour.r) + 0.7152 * channel(colour.g) + 0.0722 * channel(colour.b)
}

/// A colour with alpha, laid on what is behind it; the background itself, on white.
fn over(front: Rgba, back: Rgba) -> Rgba {
    let mix = |f: f64, b: f64| f * front.a + b * (1.0 - front.a);
    Rgba {
        r: mix(front.r, back.r),
        g: mix(front.g, back.g),
        b: mix(front.b, back.b),
        a: 1.0,
    }
}

fn contrast(text: Rgba, background: Rgba) -> Contrast {
    let white = Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    let background = over(background, white);
    let text = over(text, background);
    let (lighter, darker) = {
        let (a, b) = (luminance(text), luminance(background));
        (a.max(b), a.min(b))
    };
    let ratio = (lighter + 0.05) / (darker + 0.05);
    Contrast {
        ratio,
        aa: TextSizes {
            normal: ratio >= 4.5,
            large: ratio >= 3.0,
        },
        aaa: TextSizes {
            normal: ratio >= 7.0,
            large: ratio >= 4.5,
        },
    }
}

pub fn describe(request: &ColourRequest) -> ColourAnswer {
    let (colour, text) = read(&request.colour);
    let (against, background) = read(&request.against);
    ColourAnswer {
        contrast: text
            .zip(background)
            .map(|(text, background)| contrast(text, background)),
        colour,
        against,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notations_of(text: &str) -> (Notation, Notations, bool) {
        match read(text).0 {
            ColourReading::Read {
                given,
                notations,
                out_of_gamut,
                ..
            } => (given, notations, out_of_gamut),
            other => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn a_hex_colour_is_written_in_every_notation() {
        let (given, notations, out) = notations_of("#FF0000");

        assert_eq!(given, Notation::Hex);
        assert_eq!(notations.hex, "#ff0000");
        assert_eq!(notations.rgb, "rgb(255 0 0)");
        assert_eq!(notations.hsl, "hsl(0 100% 50%)");
        assert_eq!(notations.oklch, "oklch(62.8% 0.258 29.2)");
        assert!(!out);
    }

    #[test]
    fn every_notation_reads_back_to_the_same_colour() {
        let oklch = notations_of("#1e90ff").1.oklch;
        for text in [
            "#1e90ff",
            "1e90ff",
            "rgb(30, 144, 255)",
            "rgba(30 144 255 / 100%)",
            "hsl(209.6 100% 55.9%)",
            oklch.as_str(),
        ] {
            assert_eq!(notations_of(text).1.hex, "#1e90ff", "{text}");
        }
    }

    #[test]
    fn alpha_is_kept_in_every_notation() {
        let (_, notations, _) = notations_of("#1e90ff80");

        assert_eq!(notations.rgb, "rgb(30 144 255 / 50%)");
        assert!(notations.oklch.ends_with(" / 50%)"));
        assert_eq!(notations_of("rgb(30 144 255 / 0.5)").1.hex, "#1e90ff80");
        assert_eq!(notations_of("#f008").1.hex, "#ff000088");
    }

    #[test]
    fn a_grey_has_no_hue() {
        let (_, notations, _) = notations_of("#777777");

        assert_eq!(notations.hsl, "hsl(0 0% 46.7%)");
        assert!(notations.oklch.ends_with(" 0 0)"), "{}", notations.oklch);
    }

    #[test]
    fn an_oklch_colour_outside_srgb_is_brought_in_and_says_so() {
        let (_, notations, out) = notations_of("oklch(70% 0.4 150)");

        assert!(out);
        assert_eq!(
            notations.oklch, "oklch(70% 0.4 150)",
            "the colour asked for stays written"
        );
        let ColourReading::Read { swatch, .. } = read("oklch(70% 0.4 150)").0 else {
            panic!()
        };
        let (lightness, chroma, hue) = {
            let rgba = parse_hex(&swatch).unwrap();
            rgb_to_oklch(rgba.r, rgba.g, rgba.b)
        };
        assert!((lightness - 0.70).abs() < 0.01 && chroma < 0.4 && (hue - 150.0).abs() < 2.0);
    }

    #[test]
    fn what_is_no_colour_is_said_to_be_none() {
        assert_eq!(read("#12345").0, ColourReading::Invalid);
        assert_eq!(read("rgb(1 2)").0, ColourReading::Invalid);
        assert_eq!(read("tomato").0, ColourReading::Invalid);
        assert_eq!(read("  ").0, ColourReading::Empty);
    }

    fn measured(text: &str, against: &str) -> Contrast {
        describe(&ColourRequest {
            colour: text.to_owned(),
            against: against.to_owned(),
        })
        .contrast
        .unwrap()
    }

    #[test]
    fn contrast_is_the_wcag_ratio_with_its_verdicts() {
        let black = measured("#000", "#fff");
        assert!((black.ratio - 21.0).abs() < 1e-9);
        assert_eq!(
            black.aaa,
            TextSizes {
                normal: true,
                large: true
            }
        );

        let grey = measured("#777777", "#ffffff");
        assert!((grey.ratio - 4.48).abs() < 0.01);
        assert_eq!(
            grey.aa,
            TextSizes {
                normal: false,
                large: true
            }
        );
    }

    #[test]
    fn a_translucent_text_is_measured_on_its_background() {
        let faded = measured("rgb(0 0 0 / 50%)", "#ffffff");

        assert!(faded.ratio < 5.0 && faded.ratio > 3.0, "{}", faded.ratio);
    }

    #[test]
    fn no_contrast_without_a_second_colour() {
        let answer = describe(&ColourRequest {
            colour: "#fff".to_owned(),
            against: String::new(),
        });

        assert_eq!(answer.contrast, None);
        assert_eq!(answer.against, ColourReading::Empty);
    }
}
