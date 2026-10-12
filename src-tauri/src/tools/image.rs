//! What kind of image some bytes are, and how large, read from their first bytes alone: a
//! decoding that is a picture is shown, not only offered as a file. No decoder: a signature and
//! a header say enough, and nothing here draws a pixel.

use serde::Serialize;
use specta::Type;

use crate::closed_enum::closed_enum;

closed_enum! {
    /// The five the page draws from a `data:` URL. Its name is the extension it is saved under.
    pub enum ImageFormat {
        #[default]
        Png = "png",
        Jpeg = "jpg",
        Gif = "gif",
        Webp = "webp",
        Svg = "svg",
    }
}

impl ImageFormat {
    /// What a `data:` URL names it.
    #[must_use]
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Svg => "image/svg+xml",
        }
    }
}

/// `None` for a size its header does not state: an SVG drawn to whatever box it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Recognised {
    pub format: ImageFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

fn be16(bytes: &[u8], at: usize) -> Option<u32> {
    let pair = bytes.get(at..at + 2)?;
    Some(u32::from(u16::from_be_bytes([pair[0], pair[1]])))
}

fn le16(bytes: &[u8], at: usize) -> Option<u32> {
    let pair = bytes.get(at..at + 2)?;
    Some(u32::from(u16::from_le_bytes([pair[0], pair[1]])))
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    let four = bytes.get(at..at + 4)?;
    Some(u32::from_be_bytes([four[0], four[1], four[2], four[3]]))
}

fn le24(bytes: &[u8], at: usize) -> Option<u32> {
    let three = bytes.get(at..at + 3)?;
    Some(u32::from_le_bytes([three[0], three[1], three[2], 0]))
}

fn sized(format: ImageFormat, size: Option<(u32, u32)>) -> Recognised {
    Recognised {
        format,
        width: size.map(|(width, _)| width),
        height: size.map(|(_, height)| height),
    }
}

/// `IHDR` is the first chunk, by the specification: its width and height follow its name.
fn png(bytes: &[u8]) -> Option<(u32, u32)> {
    (bytes.get(12..16)? == b"IHDR").then_some(())?;
    Some((be32(bytes, 16)?, be32(bytes, 20)?))
}

/// The frame header is the first segment whose marker is a start of frame: the ones before
/// it — the tables, the application's own data — are stepped over by their length.
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // A fill byte before a marker.
            0xFF => at += 1,
            // Markers that stand alone.
            0x01 | 0xD0..=0xD8 => at += 2,
            // The scan, or the end: no frame header came before.
            0xD9 | 0xDA => return None,
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                return Some((be16(bytes, at + 7)?, be16(bytes, at + 5)?));
            }
            _ => at += 2 + usize::try_from(be16(bytes, at + 2)?).ok()?,
        }
    }
}

/// Three ways a WebP states its size, by the name of its first chunk.
fn webp(bytes: &[u8]) -> Option<(u32, u32)> {
    match bytes.get(12..16)? {
        b"VP8 " => {
            (bytes.get(23..26)? == [0x9D, 0x01, 0x2A]).then_some(())?;
            Some((le16(bytes, 26)? & 0x3FFF, le16(bytes, 28)? & 0x3FFF))
        }
        b"VP8L" => {
            (*bytes.get(20)? == 0x2F).then_some(())?;
            let bits = u32::from_le_bytes(bytes.get(21..25)?.try_into().ok()?);
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        b"VP8X" => Some((le24(bytes, 24)? + 1, le24(bytes, 27)? + 1)),
        _ => None,
    }
}

/// The value of `name="…"` in a tag, either quote.
fn attribute<'tag>(tag: &'tag str, name: &str) -> Option<&'tag str> {
    let mut rest = tag;
    while let Some(found) = rest.find(name) {
        let before = rest[..found].chars().next_back();
        let after = rest[found + name.len()..].trim_start();
        if before.is_some_and(char::is_whitespace) && after.starts_with('=') {
            let value = after[1..].trim_start();
            let quote = value
                .chars()
                .next()
                .filter(|quote| matches!(quote, '"' | '\''))?;
            return value[1..].split(quote).next();
        }
        rest = &rest[found + name.len()..];
    }
    None
}

/// A length in pixels, or no unit at all: `24`, `24px`, `24.5`. A percentage is no size.
fn pixels(value: &str) -> Option<u32> {
    let number: f64 = value.trim().trim_end_matches("px").trim().parse().ok()?;
    if !(1.0..=f64::from(u32::MAX)).contains(&number) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rounded = number.round() as u32;
    Some(rounded)
}

/// The root element, past a declaration, a doctype and comments; its `width` and `height`, or
/// the last two numbers of its `viewBox`.
fn svg(bytes: &[u8]) -> Option<Recognised> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("<?") {
            rest = after.split_once("?>")?.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.split_once("-->")?.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!") {
            rest = after.split_once('>')?.1.trim_start();
        } else {
            break;
        }
    }
    let tag = rest.strip_prefix("<svg")?;
    tag.starts_with(|next: char| next.is_whitespace() || next == '>' || next == '/')
        .then_some(())?;
    let tag = tag.split('>').next().unwrap_or_default();

    let stated = attribute(tag, "width")
        .and_then(pixels)
        .zip(attribute(tag, "height").and_then(pixels));
    let boxed = || {
        let mut numbers = attribute(tag, "viewBox")?
            .split([' ', ','])
            .filter(|part| !part.is_empty())
            .skip(2)
            .map(pixels);
        numbers.next()?.zip(numbers.next()?)
    };
    Some(sized(ImageFormat::Svg, stated.or_else(boxed)))
}

/// By its signature. A header cut short or unlike what the format says still names the
/// format: what the bytes are is known, how large is not.
#[must_use]
pub fn recognise(bytes: &[u8]) -> Option<Recognised> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(sized(ImageFormat::Png, png(bytes)));
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(sized(ImageFormat::Jpeg, jpeg(bytes)));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(sized(ImageFormat::Gif, le16(bytes, 6).zip(le16(bytes, 8))));
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(&b"WEBP"[..]) {
        return Some(sized(ImageFormat::Webp, webp(bytes)));
    }
    svg(bytes)
}

/// A PNG's signature, then `IHDR` with its two sizes: what the tests of this module and of
/// the decoder take for a picture.
#[cfg(test)]
pub(crate) fn png_of(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(bytes: &[u8]) -> Option<(ImageFormat, Option<u32>, Option<u32>)> {
        recognise(bytes).map(|image| (image.format, image.width, image.height))
    }

    #[test]
    fn a_png_is_known_by_its_signature_and_sized_by_its_first_chunk() {
        assert_eq!(
            size(&png_of(32, 20)),
            Some((ImageFormat::Png, Some(32), Some(20)))
        );
        // Cut before its sizes: a PNG still, of a size nobody can say.
        assert_eq!(
            size(&png_of(32, 20)[..14]),
            Some((ImageFormat::Png, None, None))
        );
    }

    #[test]
    fn a_gif_states_its_size_after_its_version() {
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[0x40, 0x01, 0xF0, 0x00, 0, 0, 0]);

        assert_eq!(size(&gif), Some((ImageFormat::Gif, Some(320), Some(240))));
        assert_eq!(
            size(b"GIF87a\x02\x00\x03\x00"),
            Some((ImageFormat::Gif, Some(2), Some(3)))
        );
    }

    /// The frame header comes after the application's own segments, each stepped over by its
    /// length; a progressive JPEG names its frame `SOF2`.
    #[test]
    fn a_jpeg_is_sized_by_its_frame_header_wherever_it_stands() {
        let frame = |marker: u8| {
            let mut jpeg = vec![0xFF, 0xD8];
            // APP0, sixteen bytes long, then a quantisation table of four.
            jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
            jpeg.extend_from_slice(&[0; 14]);
            jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x04, 0, 0]);
            // A fill byte, then the frame: precision 8, height 600, width 800.
            jpeg.extend_from_slice(&[0xFF, 0xFF, marker, 0x00, 0x11, 8, 0x02, 0x58, 0x03, 0x20]);
            jpeg
        };

        assert_eq!(
            size(&frame(0xC0)),
            Some((ImageFormat::Jpeg, Some(800), Some(600)))
        );
        assert_eq!(
            size(&frame(0xC2)),
            Some((ImageFormat::Jpeg, Some(800), Some(600)))
        );
        // `DHT` is no frame, and a scan reached first means none was found.
        assert_eq!(
            size(&[0xFF, 0xD8, 0xFF, 0xC4, 0x00, 0x02, 0xFF, 0xDA]),
            Some((ImageFormat::Jpeg, None, None))
        );
        assert_eq!(
            size(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some((ImageFormat::Jpeg, None, None))
        );
    }

    #[test]
    fn a_webp_states_its_size_in_three_ways() {
        let riff = |chunk: &[u8; 4], rest: &[u8]| {
            let mut webp = b"RIFF\0\0\0\0WEBP".to_vec();
            webp.extend_from_slice(chunk);
            webp.extend_from_slice(&[0; 4]);
            webp.extend_from_slice(rest);
            webp
        };

        // Lossy: a frame tag, the start code, then 16 by 9.
        assert_eq!(
            size(&riff(b"VP8 ", &[0, 0, 0, 0x9D, 0x01, 0x2A, 16, 0, 9, 0])),
            Some((ImageFormat::Webp, Some(16), Some(9)))
        );
        // Lossless: fourteen bits each, less one.
        let bits: u32 = 15 | (8 << 14);
        let mut lossless = vec![0x2F];
        lossless.extend_from_slice(&bits.to_le_bytes());
        assert_eq!(
            size(&riff(b"VP8L", &lossless)),
            Some((ImageFormat::Webp, Some(16), Some(9)))
        );
        // Extended: flags, then the canvas, twenty-four bits each, less one.
        assert_eq!(
            size(&riff(
                b"VP8X",
                &[0, 0, 0, 0, 0xFF, 0x03, 0x00, 0xFF, 0x01, 0x00]
            )),
            Some((ImageFormat::Webp, Some(1024), Some(512)))
        );
        assert_eq!(
            size(&riff(b"ALPH", &[])),
            Some((ImageFormat::Webp, None, None))
        );
    }

    #[test]
    fn an_svg_is_known_by_its_root_element_and_sized_by_its_attributes_or_its_view_box() {
        assert_eq!(
            size(br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16px"/>"#),
            Some((ImageFormat::Svg, Some(24), Some(16)))
        );
        assert_eq!(
            size(
                b"\xEF\xBB\xBF<?xml version=\"1.0\"?>\n<!-- drawn by hand -->\n<!DOCTYPE svg>\n<svg viewBox='0 0 120.4, 80'>\n<path/></svg>"
            ),
            Some((ImageFormat::Svg, Some(120), Some(80)))
        );
        // A percentage is no size, and neither is nothing at all.
        assert_eq!(
            size(br#"<svg width="100%" height="100%"></svg>"#),
            Some((ImageFormat::Svg, None, None))
        );
        // `stroke-width` is no `width`.
        assert_eq!(
            size(br#"<svg stroke-width="3"><rect/></svg>"#),
            Some((ImageFormat::Svg, None, None))
        );
    }

    #[test]
    fn what_is_no_image_is_none() {
        for bytes in [
            &b""[..],
            b"Caf\xC3\xA9",
            b"<svgx/>",
            b"<html><svg/></html>",
            b"RIFF\0\0\0\0WAVE",
            b"GIF90a",
            &[0x89, b'P', b'N'],
            &[0xFF, 0xD8],
        ] {
            assert_eq!(recognise(bytes), None, "{bytes:?}");
        }
    }

    #[test]
    fn each_format_names_its_media_type_and_its_extension() {
        let named: Vec<(&str, &str)> = [
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Gif,
            ImageFormat::Webp,
            ImageFormat::Svg,
        ]
        .into_iter()
        .map(|format| (format.as_str(), format.media_type()))
        .collect();

        assert_eq!(
            named,
            [
                ("png", "image/png"),
                ("jpg", "image/jpeg"),
                ("gif", "image/gif"),
                ("webp", "image/webp"),
                ("svg", "image/svg+xml")
            ]
        );
    }
}
