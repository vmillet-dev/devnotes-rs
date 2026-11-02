//! A QR code: the payload a form describes, the modules `qrcode` computes, and the two files.

use std::fmt::Write as _;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use qrcode::bits::Bits;
use qrcode::{Color, EcLevel, QrCode, Version};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::files::{self, FileProblem};
use crate::count::saturating_u32;

/// The spec's quiet zone is 4; past 16 a margin only shrinks the code.
pub const MAX_MARGIN: u32 = 16;
const PNG_PIXELS_PER_MODULE: u32 = 10;
const SVG_PIXELS_PER_MODULE: u32 = 8;
const VCARD_LINE_OCTETS: usize = 75;

/// What `mailto:` leaves alone in a subject or a body: RFC 6068's unreserved characters.
const MAILTO: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QrCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

impl QrCorrection {
    fn level(self) -> EcLevel {
        match self {
            Self::Low => EcLevel::L,
            Self::Medium => EcLevel::M,
            Self::Quartile => EcLevel::Q,
            Self::High => EcLevel::H,
        }
    }

    /// The share of the code that can be lost and still read, as the spec rounds it.
    fn recoverable(self) -> u32 {
        match self {
            Self::Low => 7,
            Self::Medium => 15,
            Self::Quartile => 25,
            Self::High => 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WifiSecurity {
    Wpa,
    Wep,
    Open,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QrContent {
    Text {
        text: String,
    },
    Url {
        url: String,
    },
    Wifi {
        ssid: String,
        password: String,
        security: WifiSecurity,
        hidden: bool,
    },
    Email {
        to: String,
        subject: String,
        body: String,
    },
    Contact {
        name: String,
        phone: String,
        email: String,
        organisation: String,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QrRequest {
    pub content: QrContent,
    pub correction: QrCorrection,
    pub margin: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QrAnswer {
    Empty,
    TooLong {
        bytes: u32,
        maximum: u32,
    },
    Code {
        payload: String,
        version: u32,
        /// Modules on a side, margin left out.
        modules: u32,
        bytes: u32,
        capacity: u32,
        recoverable: u32,
        /// Modules on a side, margin included: the `viewBox` of `path`.
        side: u32,
        /// The dark modules as one SVG path, a rectangle per horizontal run.
        path: String,
        missing_scheme: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QrFormat {
    Svg,
    Png,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SavedCode {
    Saved { bytes: u32 },
    Nothing,
    Failed { problem: FileProblem },
}

/// The modules, read once: what the preview, the SVG file and the PNG all draw.
struct Drawn {
    dark: Vec<bool>,
    width: usize,
    margin: usize,
}

impl Drawn {
    fn side(&self) -> usize {
        self.width + 2 * self.margin
    }

    fn is_dark(&self, x: usize, y: usize) -> bool {
        self.dark[y * self.width + x]
    }

    fn path(&self) -> String {
        let mut path = String::new();
        for y in 0..self.width {
            let mut x = 0;
            while x < self.width {
                if !self.is_dark(x, y) {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < self.width && self.is_dark(x, y) {
                    x += 1;
                }
                let run = x - start;
                let _ = write!(
                    path,
                    "M{},{}h{run}v1h-{run}z",
                    start + self.margin,
                    y + self.margin
                );
            }
        }
        path
    }

    /// One grey byte per pixel, black on white.
    fn raster(&self, scale: usize) -> (u32, Vec<u8>) {
        let pixels = self.side() * scale;
        let mut grey = vec![u8::MAX; pixels * pixels];
        for y in 0..self.width {
            for x in 0..self.width {
                if !self.is_dark(x, y) {
                    continue;
                }
                for row in 0..scale {
                    let top = ((y + self.margin) * scale + row) * pixels;
                    let left = top + (x + self.margin) * scale;
                    grey[left..left + scale].fill(0);
                }
            }
        }
        (saturating_u32(pixels), grey)
    }
}

pub fn describe(request: &QrRequest) -> QrAnswer {
    let Some(payload) = payload(&request.content) else {
        return QrAnswer::Empty;
    };
    let level = request.correction.level();
    let bytes = payload.len();
    let Some(version) = (1..=40).find(|&version| byte_capacity(version, level) >= bytes) else {
        return QrAnswer::TooLong {
            bytes: saturating_u32(bytes),
            maximum: saturating_u32(byte_capacity(40, level)),
        };
    };
    let Some(drawn) = draw(&payload, version, request) else {
        return QrAnswer::TooLong {
            bytes: saturating_u32(bytes),
            maximum: saturating_u32(byte_capacity(40, level)),
        };
    };
    let missing_scheme = matches!(&request.content, QrContent::Url { .. }) && !has_scheme(&payload);
    QrAnswer::Code {
        version: version.unsigned_abs().into(),
        modules: saturating_u32(drawn.width),
        bytes: saturating_u32(bytes),
        capacity: saturating_u32(byte_capacity(version, level)),
        recoverable: request.correction.recoverable(),
        side: saturating_u32(drawn.side()),
        path: drawn.path(),
        payload,
        missing_scheme,
    }
}

/// Byte mode whatever the text: the capacity then reads in the bytes the user typed.
fn byte_capacity(version: i16, level: EcLevel) -> usize {
    let data_bits = Bits::new(Version::Normal(version))
        .max_len(level)
        .unwrap_or(0);
    let count_bits = if version <= 9 { 8 } else { 16 };
    data_bits.saturating_sub(4 + count_bits) / 8
}

fn draw(payload: &str, version: i16, request: &QrRequest) -> Option<Drawn> {
    let level = request.correction.level();
    let mut bits = Bits::new(Version::Normal(version));
    bits.push_byte_data(payload.as_bytes()).ok()?;
    bits.push_terminator(level).ok()?;
    let code = QrCode::with_bits(bits, level).ok()?;
    Some(Drawn {
        width: code.width(),
        dark: code
            .to_colors()
            .into_iter()
            .map(|colour| colour == Color::Dark)
            .collect(),
        margin: request.margin.min(MAX_MARGIN) as usize,
    })
}

fn drawn(request: &QrRequest) -> Option<Drawn> {
    let payload = payload(&request.content)?;
    let level = request.correction.level();
    let version = (1..=40).find(|&version| byte_capacity(version, level) >= payload.len())?;
    draw(&payload, version, request)
}

/// The file is the preview: the same path, on the same white.
fn svg(drawn: &Drawn) -> String {
    let side = drawn.side();
    let pixels = side * SVG_PIXELS_PER_MODULE as usize;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" width=\"{pixels}\" \
         height=\"{pixels}\" shape-rendering=\"crispEdges\">\
         <rect width=\"{side}\" height=\"{side}\" fill=\"#fff\"/>\
         <path d=\"{}\" fill=\"#000\"/></svg>\n",
        drawn.path()
    )
}

fn png(drawn: &Drawn) -> Option<Vec<u8>> {
    let (pixels, grey) = drawn.raster(PNG_PIXELS_PER_MODULE as usize);
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, pixels, pixels);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&grey).ok()?;
    writer.finish().ok()?;
    Some(png)
}

pub fn save(request: &QrRequest, format: QrFormat, path: &str) -> SavedCode {
    let Some(drawn) = drawn(request) else {
        return SavedCode::Nothing;
    };
    let bytes = match format {
        QrFormat::Svg => svg(&drawn).into_bytes(),
        QrFormat::Png => match png(&drawn) {
            Some(bytes) => bytes,
            None => return SavedCode::Nothing,
        },
    };
    match files::write(path, &bytes) {
        Ok(()) => SavedCode::Saved {
            bytes: saturating_u32(bytes.len()),
        },
        Err(problem) => SavedCode::Failed { problem },
    }
}

/// What the clipboard takes: RGBA at the PNG's scale, or nothing when there is no code.
pub fn rgba(request: &QrRequest) -> Option<(u32, Vec<u8>)> {
    let (pixels, grey) = drawn(request)?.raster(PNG_PIXELS_PER_MODULE as usize);
    let rgba = grey
        .into_iter()
        .flat_map(|value| [value, value, value, u8::MAX])
        .collect();
    Some((pixels, rgba))
}

/// `None` when the form holds nothing to encode.
pub(crate) fn payload(content: &QrContent) -> Option<String> {
    match content {
        QrContent::Text { text } => (!text.is_empty()).then(|| text.clone()),
        QrContent::Url { url } => {
            let url = url.trim();
            (!url.is_empty()).then(|| url.to_owned())
        }
        QrContent::Wifi {
            ssid,
            password,
            security,
            hidden,
        } => (!ssid.is_empty()).then(|| wifi(ssid, password, *security, *hidden)),
        QrContent::Email { to, subject, body } => {
            let to = to.trim();
            (!to.is_empty()).then(|| mailto(to, subject, body))
        }
        QrContent::Contact {
            name,
            phone,
            email,
            organisation,
        } => {
            let card = Card {
                name: name.trim(),
                phone: phone.trim(),
                email: email.trim(),
                organisation: organisation.trim(),
            };
            (!card.is_empty()).then(|| card.vcard())
        }
    }
}

/// The scheme a URL starts with, as RFC 3986 spells one: `exemple.fr/doc` has none.
fn has_scheme(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-'))
    })
}

/// The `ZXing` format every phone camera reads.
fn wifi(ssid: &str, password: &str, security: WifiSecurity, hidden: bool) -> String {
    let kind = match security {
        WifiSecurity::Wpa => "WPA",
        WifiSecurity::Wep => "WEP",
        WifiSecurity::Open => "nopass",
    };
    let mut payload = format!("WIFI:T:{kind};S:{};", escape_wifi(ssid));
    if security != WifiSecurity::Open {
        let _ = write!(payload, "P:{};", escape_wifi(password));
    }
    if hidden {
        payload.push_str("H:true;");
    }
    payload.push(';');
    payload
}

fn escape_wifi(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn mailto(to: &str, subject: &str, body: &str) -> String {
    let fields: Vec<String> = [("subject", subject), ("body", body)]
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| format!("{name}={}", utf8_percent_encode(value, MAILTO)))
        .collect();
    if fields.is_empty() {
        format!("mailto:{to}")
    } else {
        format!("mailto:{to}?{}", fields.join("&"))
    }
}

struct Card<'a> {
    name: &'a str,
    phone: &'a str,
    email: &'a str,
    organisation: &'a str,
}

impl Card<'_> {
    fn is_empty(&self) -> bool {
        [self.name, self.phone, self.email, self.organisation]
            .iter()
            .all(|value| value.is_empty())
    }

    /// `vCard` 3.0 (RFC 2426): `N` and `FN` are required, so a card without a name is named after
    /// what it has.
    fn vcard(&self) -> String {
        let shown = [self.name, self.organisation, self.email, self.phone]
            .into_iter()
            .find(|value| !value.is_empty())
            .unwrap_or_default();
        let (given, family) = match self.name.rsplit_once(' ') {
            Some((given, family)) => (given.trim(), family),
            None => ("", self.name),
        };
        let mut lines = vec![
            "BEGIN:VCARD".to_owned(),
            "VERSION:3.0".to_owned(),
            format!("N:{};{};;;", escape_vcard(family), escape_vcard(given)),
            format!("FN:{}", escape_vcard(shown)),
        ];
        for (property, value) in [
            ("ORG", self.organisation),
            ("TEL", self.phone),
            ("EMAIL", self.email),
        ] {
            if !value.is_empty() {
                lines.push(format!("{property}:{}", escape_vcard(value)));
            }
        }
        lines.push("END:VCARD".to_owned());
        lines
            .iter()
            .map(|line| fold(line))
            .collect::<Vec<_>>()
            .join("\r\n")
    }
}

fn escape_vcard(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' | ',' | ';' => {
                escaped.push('\\');
                escaped.push(c);
            }
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Past 75 octets a line goes on after a line break and a space, never inside a character.
fn fold(line: &str) -> String {
    let mut folded = String::with_capacity(line.len());
    let mut octets = 0;
    for c in line.chars() {
        if octets + c.len_utf8() > VCARD_LINE_OCTETS {
            folded.push_str("\r\n ");
            octets = 1;
        }
        folded.push(c);
        octets += c.len_utf8();
    }
    folded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content: QrContent, correction: QrCorrection) -> QrRequest {
        QrRequest {
            content,
            correction,
            margin: 4,
        }
    }

    fn url(url: &str) -> QrContent {
        QrContent::Url {
            url: url.to_owned(),
        }
    }

    fn text(text: &str) -> QrContent {
        QrContent::Text {
            text: text.to_owned(),
        }
    }

    fn wifi_payload(ssid: &str, password: &str, security: WifiSecurity, hidden: bool) -> String {
        payload(&QrContent::Wifi {
            ssid: ssid.to_owned(),
            password: password.to_owned(),
            security,
            hidden,
        })
        .unwrap()
    }

    fn contact(name: &str, phone: &str, email: &str, organisation: &str) -> Option<String> {
        payload(&QrContent::Contact {
            name: name.to_owned(),
            phone: phone.to_owned(),
            email: email.to_owned(),
            organisation: organisation.to_owned(),
        })
    }

    #[test]
    fn a_url_of_22_bytes_is_a_version_2_at_medium_with_26_bytes_of_room() {
        let answer = describe(&request(
            url("https://exemple.fr/doc"),
            QrCorrection::Medium,
        ));

        let QrAnswer::Code {
            version,
            modules,
            bytes,
            capacity,
            recoverable,
            side,
            missing_scheme,
            ..
        } = answer
        else {
            panic!("a code, not {answer:?}");
        };
        assert_eq!((version, modules, side), (2, 25, 33));
        assert_eq!((bytes, capacity, recoverable), (22, 26, 15));
        assert!(!missing_scheme);
    }

    #[test]
    fn the_modules_are_the_ones_the_crate_draws_for_that_version() {
        let drawn = drawn(&request(
            url("https://exemple.fr/doc"),
            QrCorrection::Medium,
        ))
        .unwrap();
        let reference =
            QrCode::with_version("https://exemple.fr/doc", Version::Normal(2), EcLevel::M).unwrap();

        let expected: Vec<bool> = reference
            .to_colors()
            .into_iter()
            .map(|colour| colour == Color::Dark)
            .collect();
        assert_eq!(drawn.dark, expected);
        // The top-left finder: a dark 7×7 ring around a light ring around a dark 3×3.
        assert!((0..7).all(|i| drawn.is_dark(i, 0) && drawn.is_dark(0, i)));
        assert!((1..6).all(|i| !drawn.is_dark(i, 1)));
        assert!(drawn.is_dark(3, 3));
    }

    #[test]
    fn each_level_has_its_byte_capacity_from_version_1_to_40() {
        let capacities = |version| {
            [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H]
                .map(|level| byte_capacity(version, level))
        };

        assert_eq!(capacities(1), [17, 14, 11, 7]);
        assert_eq!(capacities(10), [271, 213, 151, 119]);
        assert_eq!(capacities(40), [2953, 2331, 1663, 1273]);
    }

    #[test]
    fn a_higher_level_takes_a_larger_code_for_the_same_text() {
        let version =
            |correction| match describe(&request(url("https://exemple.fr/doc"), correction)) {
                QrAnswer::Code { version, .. } => version,
                other => panic!("a code, not {other:?}"),
            };

        assert_eq!(
            [
                QrCorrection::Low,
                QrCorrection::Medium,
                QrCorrection::Quartile,
                QrCorrection::High
            ]
            .map(version),
            [2, 2, 3, 3]
        );
    }

    #[test]
    fn a_payload_past_version_40_says_the_maximum() {
        let answer = describe(&request(text(&"a".repeat(1274)), QrCorrection::High));

        assert_eq!(
            answer,
            QrAnswer::TooLong {
                bytes: 1274,
                maximum: 1273
            }
        );
    }

    #[test]
    fn nothing_to_encode_is_empty_not_a_code() {
        assert_eq!(
            describe(&request(text(""), QrCorrection::Medium)),
            QrAnswer::Empty
        );
        assert_eq!(
            describe(&request(url("  "), QrCorrection::Medium)),
            QrAnswer::Empty
        );
        assert_eq!(contact(" ", "", "", ""), None);
    }

    #[test]
    fn a_url_without_a_scheme_is_said_not_mended() {
        let answer = describe(&request(url("exemple.fr/doc"), QrCorrection::Medium));

        let QrAnswer::Code {
            payload,
            missing_scheme,
            ..
        } = answer
        else {
            panic!("a code");
        };
        assert_eq!(payload, "exemple.fr/doc");
        assert!(missing_scheme);
        assert!(has_scheme("mailto:ada@exemple.fr"));
        assert!(!has_scheme("exemple.fr:8080/doc"));
    }

    #[test]
    fn a_wifi_network_escapes_what_its_format_reserves() {
        assert_eq!(
            wifi_payload("Café;Wi-Fi", r#"p\a:s,s"w"#, WifiSecurity::Wpa, false),
            r#"WIFI:T:WPA;S:Café\;Wi-Fi;P:p\\a\:s\,s\"w;;"#
        );
        assert_eq!(
            wifi_payload("Maison", "secret", WifiSecurity::Wep, true),
            "WIFI:T:WEP;S:Maison;P:secret;H:true;;"
        );
        assert_eq!(
            wifi_payload("Invités", "ignoré", WifiSecurity::Open, false),
            "WIFI:T:nopass;S:Invités;;"
        );
    }

    #[test]
    fn an_email_is_a_mailto_with_its_subject_and_body_encoded() {
        let email = |subject: &str, body: &str| {
            payload(&QrContent::Email {
                to: " ada@exemple.fr ".to_owned(),
                subject: subject.to_owned(),
                body: body.to_owned(),
            })
            .unwrap()
        };

        assert_eq!(email("", ""), "mailto:ada@exemple.fr");
        assert_eq!(
            email("Réunion & café", "À demain\n!"),
            "mailto:ada@exemple.fr?subject=R%C3%A9union%20%26%20caf%C3%A9&body=%C3%80%20demain%0A%21"
        );
    }

    #[test]
    fn a_contact_is_a_vcard_3_with_its_values_escaped() {
        assert_eq!(
            contact(
                "Ada Lovelace",
                "+33 6 12 34 56 78",
                "ada@exemple.fr",
                "Analytical; Engines"
            )
            .unwrap(),
            [
                "BEGIN:VCARD",
                "VERSION:3.0",
                "N:Lovelace;Ada;;;",
                "FN:Ada Lovelace",
                r"ORG:Analytical\; Engines",
                "TEL:+33 6 12 34 56 78",
                "EMAIL:ada@exemple.fr",
                "END:VCARD",
            ]
            .join("\r\n")
        );
        assert!(
            contact("", "", "", "DevNotes")
                .unwrap()
                .contains("N:;;;;\r\nFN:DevNotes")
        );
    }

    #[test]
    fn a_long_vcard_line_is_folded_at_75_octets_between_characters() {
        let card = contact("", "", "", &"é".repeat(60)).unwrap();
        let org = card
            .lines()
            .skip_while(|line| !line.starts_with("ORG:"))
            .collect::<Vec<_>>();

        assert!(
            card.lines()
                .all(|line| line.trim_end_matches('\r').len() <= 75)
        );
        assert!(org[1].starts_with(' '));
        assert_eq!(
            org.iter()
                .take_while(|line| !line.starts_with("END"))
                .map(|line| line
                    .trim_end_matches('\r')
                    .strip_prefix(' ')
                    .unwrap_or(line.trim_end_matches('\r')))
                .collect::<String>(),
            format!("ORG:{}", "é".repeat(60))
        );
    }

    #[test]
    fn the_path_covers_every_dark_module_inside_the_margin() {
        let drawn = drawn(&request(text("DevNotes"), QrCorrection::Low)).unwrap();
        let path = drawn.path();

        let covered: usize = path
            .split('h')
            .skip(1)
            .step_by(2)
            .map(|run| run.split('v').next().unwrap().parse::<usize>().unwrap())
            .sum();
        assert_eq!(covered, drawn.dark.iter().filter(|dark| **dark).count());
        assert!(path.starts_with("M4,4h7v1h-7z"));
    }

    #[test]
    fn the_svg_file_is_the_preview_on_white() {
        let drawn = drawn(&request(text("DevNotes"), QrCorrection::Low)).unwrap();
        let file = svg(&drawn);

        assert!(file.contains(r#"viewBox="0 0 29 29" width="232" height="232""#));
        assert!(file.contains(r##"<rect width="29" height="29" fill="#fff"/>"##));
        assert!(file.contains(&format!(r##"<path d="{}" fill="#000"/>"##, drawn.path())));
    }

    #[test]
    fn the_png_is_ten_pixels_a_module_with_its_margin_white() {
        let drawn = drawn(&request(text("DevNotes"), QrCorrection::Low)).unwrap();
        let bytes = png(&drawn).unwrap();

        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut grey = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut grey).unwrap();
        assert_eq!((info.width, info.height), (290, 290));
        assert_eq!(info.color_type, png::ColorType::Grayscale);
        // The margin is 40 pixels of white, the finder's corner black.
        assert!(grey[..290 * 40].iter().all(|value| *value == u8::MAX));
        assert_eq!(grey[290 * 40 + 40], 0);
    }

    #[test]
    fn a_margin_is_capped_and_zero_is_allowed() {
        let side = |margin| {
            let request = QrRequest {
                margin,
                ..request(text("DevNotes"), QrCorrection::Low)
            };
            match describe(&request) {
                QrAnswer::Code { side, .. } => side,
                other => panic!("a code, not {other:?}"),
            }
        };

        assert_eq!([side(0), side(4), side(99)], [21, 29, 21 + 2 * MAX_MARGIN]);
    }

    #[test]
    fn saving_writes_the_chosen_format_and_says_its_size() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("qr.svg").to_string_lossy().into_owned();
        let code = request(text("DevNotes"), QrCorrection::Low);

        let saved = save(&code, QrFormat::Svg, &path);

        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            saved,
            SavedCode::Saved {
                bytes: saturating_u32(written.len())
            }
        );
        assert!(written.starts_with("<svg"));
        assert_eq!(
            save(&request(text(""), QrCorrection::Low), QrFormat::Png, &path),
            SavedCode::Nothing
        );
    }

    #[test]
    fn the_clipboard_gets_opaque_rgba_at_the_png_scale() {
        let (pixels, rgba) = rgba(&request(text("DevNotes"), QrCorrection::Low)).unwrap();

        assert_eq!(pixels, 290);
        assert_eq!(rgba.len(), 290 * 290 * 4);
        assert!(rgba.chunks(4).all(|pixel| pixel[3] == u8::MAX));
    }
}
