//! Bytes written in Base64, Base64 URL, Base32, hexadecimal, binary and decimal — all at once —
//! and read back from whichever of them a pasted text looks like, or from the one asked.

use std::fmt::Write;

use base64::DecodeError;
use base64::Engine;
use base64::alphabet;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::files::{self, FileProblem};
use crate::count::saturating_u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ByteEncoding {
    Base64,
    /// `-` and `_` for `+` and `/`, without padding: what a URL or a JWT carries.
    Base64Url,
    Base32,
    Hex,
    Binary,
    Decimal,
}

/// The order a pasted text is tried in: the narrowest alphabet first.
const RECOGNITION: [ByteEncoding; 6] = [
    ByteEncoding::Binary,
    ByteEncoding::Hex,
    ByteEncoding::Decimal,
    ByteEncoding::Base32,
    ByteEncoding::Base64Url,
    ByteEncoding::Base64,
];

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base64_engine(url: bool) -> GeneralPurpose {
    GeneralPurpose::new(
        if url {
            &alphabet::URL_SAFE
        } else {
            &alphabet::STANDARD
        },
        GeneralPurposeConfig::new()
            .with_encode_padding(!url)
            .with_decode_padding_mode(DecodePaddingMode::Indifferent),
    )
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EncodeRequest {
    pub text: String,
    /// Hexadecimal and binary, a space between bytes. Decimal always has one.
    pub spaced: bool,
    pub uppercase: bool,
}

/// The first character UTF-8 writes in more than one byte: « é » takes 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WideCharacter {
    pub character: String,
    pub bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Encodings {
    pub characters: u32,
    pub bytes: u32,
    pub wide: Option<WideCharacter>,
    pub base64: String,
    pub base64_url: String,
    pub base32: String,
    pub hex: String,
    pub binary: String,
    pub decimal: String,
}

fn base32(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(5) * 8);
    for chunk in bytes.chunks(5) {
        let mut group = [0u8; 5];
        group[..chunk.len()].copy_from_slice(chunk);
        let bits = group
            .iter()
            .fold(0u64, |bits, byte| (bits << 8) | u64::from(*byte));
        // 8 characters for 5 bytes; fewer bytes, fewer characters, the rest padding.
        let written = (chunk.len() * 8).div_ceil(5);
        for at in 0..8 {
            if at < written {
                let index = (bits >> (35 - at * 5)) & 0x1f;
                text.push(char::from(BASE32[usize::try_from(index).unwrap_or(0)]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

fn joined(parts: impl Iterator<Item = String>, spaced: bool) -> String {
    parts
        .collect::<Vec<_>>()
        .join(if spaced { " " } else { "" })
}

pub fn encode(request: &EncodeRequest) -> Encodings {
    let bytes = request.text.as_bytes();
    let wide = request
        .text
        .chars()
        .find(|character| character.len_utf8() > 1)
        .map(|character| WideCharacter {
            character: character.to_string(),
            bytes: saturating_u32(character.len_utf8()),
        });
    Encodings {
        characters: saturating_u32(request.text.chars().count()),
        bytes: saturating_u32(bytes.len()),
        wide,
        base64: base64_engine(false).encode(bytes),
        base64_url: base64_engine(true).encode(bytes),
        base32: base32(bytes),
        hex: joined(
            bytes.iter().map(|byte| {
                if request.uppercase {
                    format!("{byte:02X}")
                } else {
                    format!("{byte:02x}")
                }
            }),
            request.spaced,
        ),
        binary: joined(
            bytes.iter().map(|byte| format!("{byte:08b}")),
            request.spaced,
        ),
        decimal: joined(bytes.iter().map(u8::to_string), true),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Base64FileAnswer {
    /// A file keeps the two Base64 alone: the other encodings of a megabyte are noise.
    Encoded {
        name: String,
        bytes: u32,
        base64: String,
        base64_url: String,
    },
    Failed {
        problem: FileProblem,
    },
}

pub fn encode_file(path: &str) -> Base64FileAnswer {
    match files::read_limited(path) {
        Ok(bytes) => Base64FileAnswer::Encoded {
            name: files::name_of(path),
            bytes: saturating_u32(bytes.len()),
            base64: base64_engine(false).encode(&bytes),
            base64_url: base64_engine(true).encode(&bytes),
        },
        Err(problem) => Base64FileAnswer::Failed { problem },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DecodeProblem {
    /// A character outside the alphabet, or padding in the middle.
    Character,
    /// A length nothing of this encoding ends on: one Base64 character past a group, an odd
    /// number of hex digits, bits that are no whole byte.
    Length,
    /// A last character whose bits say the text was cut.
    Truncated,
    /// A decimal byte past 255.
    TooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DecodedBytes {
    Text {
        text: String,
        bytes: u32,
    },
    /// Not UTF-8: what it starts with, in hexadecimal, and an offer to save it.
    Binary {
        bytes: u32,
        preview: String,
    },
    /// `at` counts characters of the text as given, spaces and line breaks included.
    Invalid {
        problem: DecodeProblem,
        at: Option<u32>,
        character: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DecodeRequest {
    pub text: String,
    /// `None` reads it by its shape.
    pub reading: Option<ByteEncoding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DecodeAnswer {
    pub read_as: ByteEncoding,
    /// Read by its shape rather than as asked.
    pub guessed: bool,
    /// The other readings its shape allows: `deadbeef` is hexadecimal and Base64 alike.
    pub also: Vec<ByteEncoding>,
    pub decoded: DecodedBytes,
}

/// A refusal before any byte: where, in the text as given, and which character.
#[derive(Clone, Copy)]
struct Refused {
    problem: DecodeProblem,
    at: Option<usize>,
}

/// The characters that count, each with where it stood: a text wrapped at 76 columns decodes.
fn significant(text: &str, separators: &[char]) -> (String, Vec<usize>) {
    text.chars()
        .enumerate()
        .filter(|(_, character)| !character.is_whitespace() && !separators.contains(character))
        .map(|(position, character)| (character, position))
        .unzip()
}

fn first_outside(
    kept: &str,
    positions: &[usize],
    allowed: impl Fn(char) -> bool,
) -> Option<Refused> {
    kept.chars()
        .position(|character| !allowed(character))
        .map(|index| Refused {
            problem: DecodeProblem::Character,
            at: positions.get(index).copied(),
        })
}

fn decode_base64(text: &str, url: bool) -> Result<Vec<u8>, Refused> {
    let (kept, positions) = significant(text, &[]);
    base64_engine(url).decode(kept.as_bytes()).map_err(|error| {
        let (problem, offset) = match error {
            DecodeError::InvalidByte(offset, _) => (DecodeProblem::Character, Some(offset)),
            DecodeError::InvalidLastSymbol { offset, .. } => {
                (DecodeProblem::Truncated, Some(offset))
            }
            DecodeError::InvalidLength(_) => (DecodeProblem::Length, None),
            DecodeError::InvalidPadding => (DecodeProblem::Character, kept.find('=')),
        };
        // An offset is in bytes of `kept`: every byte refused is ASCII, as is every one
        // before it, so it is a character index too.
        Refused {
            problem,
            at: offset.and_then(|offset| positions.get(offset).copied()),
        }
    })
}

fn decode_base32(text: &str) -> Result<Vec<u8>, Refused> {
    let (kept, positions) = significant(text, &[]);
    let body = kept.trim_end_matches('=');
    if let Some(refused) = first_outside(body, &positions, |c| {
        BASE32.contains(&u8::try_from(c.to_ascii_uppercase()).unwrap_or(0))
    }) {
        return Err(refused);
    }
    if matches!(body.len() % 8, 1 | 3 | 6) {
        return Err(Refused {
            problem: DecodeProblem::Length,
            at: None,
        });
    }
    let mut bytes = Vec::with_capacity(body.len() * 5 / 8);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for character in body.bytes() {
        let value = BASE32
            .iter()
            .position(|letter| *letter == character.to_ascii_uppercase())
            .unwrap_or(0);
        buffer = (buffer << 5) | u32::try_from(value).unwrap_or(0);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((buffer >> bits) & 0xff).unwrap_or(0));
        }
    }
    Ok(bytes)
}

/// `0x` before the digits, and `:`, `-` or spaces between bytes, are left out.
fn decode_hex(text: &str) -> Result<Vec<u8>, Refused> {
    let stripped = text.replace("0x", "  ").replace("0X", "  ");
    let (kept, positions) = significant(&stripped, &[':', '-', ',']);
    if let Some(refused) = first_outside(&kept, &positions, |c| c.is_ascii_hexdigit()) {
        return Err(refused);
    }
    // Apart, each group is whole bytes: `67 97 102` is decimal, not hexadecimal.
    let odd_group = stripped
        .split(|c: char| c.is_whitespace() || matches!(c, ':' | '-' | ','))
        .any(|group| group.len() % 2 != 0);
    if kept.is_empty() || kept.len() % 2 != 0 || odd_group {
        return Err(Refused {
            problem: DecodeProblem::Length,
            at: None,
        });
    }
    Ok(kept
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16).unwrap_or(0))
        .collect())
}

fn decode_binary(text: &str) -> Result<Vec<u8>, Refused> {
    let (kept, positions) = significant(text, &[]);
    if let Some(refused) = first_outside(&kept, &positions, |c| c == '0' || c == '1') {
        return Err(refused);
    }
    if kept.is_empty() || kept.len() % 8 != 0 {
        return Err(Refused {
            problem: DecodeProblem::Length,
            at: None,
        });
    }
    Ok(kept
        .as_bytes()
        .chunks(8)
        .map(|byte| {
            byte.iter()
                .fold(0u8, |value, bit| (value << 1) | (bit - b'0'))
        })
        .collect())
}

/// Numbers from 0 to 255, apart by spaces or commas.
fn decode_decimal(text: &str) -> Result<Vec<u8>, Refused> {
    let mut bytes = Vec::new();
    let mut start: Option<usize> = None;
    let characters: Vec<char> = text.chars().collect();
    for (at, character) in characters.iter().chain([&' ']).enumerate() {
        if character.is_ascii_digit() {
            start.get_or_insert(at);
        } else if character.is_whitespace() || *character == ',' {
            if let Some(begin) = start.take() {
                let digits: String = characters[begin..at].iter().collect();
                match digits.parse::<u8>() {
                    Ok(byte) => bytes.push(byte),
                    Err(_) => {
                        return Err(Refused {
                            problem: DecodeProblem::TooLarge,
                            at: Some(begin),
                        });
                    }
                }
            }
        } else {
            return Err(Refused {
                problem: DecodeProblem::Character,
                at: Some(at),
            });
        }
    }
    if bytes.is_empty() {
        return Err(Refused {
            problem: DecodeProblem::Length,
            at: None,
        });
    }
    Ok(bytes)
}

fn decode_as(text: &str, encoding: ByteEncoding) -> Result<Vec<u8>, Refused> {
    match encoding {
        ByteEncoding::Base64 => decode_base64(text, false),
        ByteEncoding::Base64Url => decode_base64(text, true),
        ByteEncoding::Base32 => decode_base32(text),
        ByteEncoding::Hex => decode_hex(text),
        ByteEncoding::Binary => decode_binary(text),
        ByteEncoding::Decimal => decode_decimal(text),
    }
}

/// What a text's shape allows, narrowest first. Base32 is recognised in capitals alone, Base64
/// URL by a `-` or a `_`: a forced reading takes either case, and either alphabet.
fn shapes(text: &str) -> Vec<ByteEncoding> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    RECOGNITION
        .into_iter()
        .filter(|encoding| match encoding {
            ByteEncoding::Base32 => {
                compact
                    .trim_end_matches('=')
                    .bytes()
                    .all(|b| BASE32.contains(&b))
                    && decode_base32(text).is_ok()
            }
            ByteEncoding::Base64Url => {
                compact.contains(['-', '_']) && decode_base64(text, true).is_ok()
            }
            ByteEncoding::Base64 => {
                !compact.contains(['-', '_']) && decode_base64(text, false).is_ok()
            }
            other => decode_as(text, *other).is_ok(),
        })
        .collect()
}

/// How many bytes of a binary decoding are shown.
const PREVIEW_BYTES: usize = 48;

fn as_decoded(bytes: Vec<u8>) -> DecodedBytes {
    let count = saturating_u32(bytes.len());
    match String::from_utf8(bytes) {
        Ok(text) => DecodedBytes::Text { text, bytes: count },
        Err(error) => {
            let preview = error.into_bytes().iter().take(PREVIEW_BYTES).fold(
                String::new(),
                |mut hex, byte| {
                    let _ = write!(hex, "{}{byte:02x}", if hex.is_empty() { "" } else { " " });
                    hex
                },
            );
            DecodedBytes::Binary {
                bytes: count,
                preview,
            }
        }
    }
}

fn refused(text: &str, refusal: Refused) -> DecodedBytes {
    DecodedBytes::Invalid {
        problem: refusal.problem,
        at: refusal.at.map(saturating_u32),
        character: refusal
            .at
            .and_then(|at| text.chars().nth(at))
            .map(String::from),
    }
}

pub fn decode(request: &DecodeRequest) -> DecodeAnswer {
    let shapes = shapes(&request.text);
    let (read_as, guessed) = match request.reading {
        Some(encoding) => (encoding, false),
        // Nothing fits: Base64, the commonest, says where it breaks.
        None => (
            shapes.first().copied().unwrap_or(ByteEncoding::Base64),
            true,
        ),
    };
    let decoded = match decode_as(&request.text, read_as) {
        Ok(bytes) => as_decoded(bytes),
        Err(refusal) => refused(&request.text, refusal),
    };
    DecodeAnswer {
        read_as,
        guessed,
        also: shapes
            .into_iter()
            .filter(|encoding| *encoding != read_as)
            .collect(),
        decoded,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SavedBytes {
    Saved { bytes: u32 },
    Invalid,
    Failed { problem: FileProblem },
}

/// The bytes a decoding gave, written where the user chose: never a note.
pub fn save(request: &DecodeRequest, path: &str) -> SavedBytes {
    let reading = request
        .reading
        .or_else(|| shapes(&request.text).first().copied())
        .unwrap_or(ByteEncoding::Base64);
    match decode_as(&request.text, reading) {
        Ok(bytes) => match files::write(path, &bytes) {
            Ok(()) => SavedBytes::Saved {
                bytes: saturating_u32(bytes.len()),
            },
            Err(problem) => SavedBytes::Failed { problem },
        },
        Err(_) => SavedBytes::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(text: &str) -> Encodings {
        encode(&EncodeRequest {
            text: text.to_owned(),
            spaced: true,
            uppercase: false,
        })
    }

    fn read(text: &str, reading: Option<ByteEncoding>) -> DecodeAnswer {
        decode(&DecodeRequest {
            text: text.to_owned(),
            reading,
        })
    }

    fn text_of(answer: &DecodeAnswer) -> &str {
        match &answer.decoded {
            DecodedBytes::Text { text, .. } => text,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_text_is_written_in_every_encoding_at_once() {
        let all = encoded("Café");

        assert_eq!((all.characters, all.bytes), (4, 5));
        assert_eq!(
            all.wide,
            Some(WideCharacter {
                character: "é".into(),
                bytes: 2
            })
        );
        assert_eq!(all.base64, "Q2Fmw6k=");
        assert_eq!(all.base64_url, "Q2Fmw6k");
        assert_eq!(all.base32, "INQWNQ5J");
        assert_eq!(all.hex, "43 61 66 c3 a9");
        assert_eq!(all.binary, "01000011 01100001 01100110 11000011 10101001");
        assert_eq!(all.decimal, "67 97 102 195 169");
        assert_eq!(encoded("abc").wide, None);
    }

    #[test]
    fn hexadecimal_and_binary_follow_their_options() {
        let tight = encode(&EncodeRequest {
            text: "Café".into(),
            spaced: false,
            uppercase: true,
        });

        assert_eq!(tight.hex, "436166C3A9");
        assert_eq!(tight.binary, "0100001101100001011001101100001110101001");
        assert_eq!(
            tight.decimal, "67 97 102 195 169",
            "decimal needs its spaces"
        );
    }

    /// RFC 4648, section 10.
    #[test]
    fn the_rfc_vectors_go_both_ways() {
        for (plain, base64, base32) in [
            ("", "", ""),
            ("f", "Zg==", "MY======"),
            ("fo", "Zm8=", "MZXQ===="),
            ("foo", "Zm9v", "MZXW6==="),
            ("foob", "Zm9vYg==", "MZXW6YQ="),
            ("fooba", "Zm9vYmE=", "MZXW6YTB"),
            ("foobar", "Zm9vYmFy", "MZXW6YTBOI======"),
        ] {
            let all = encoded(plain);
            assert_eq!(
                (all.base64.as_str(), all.base32.as_str()),
                (base64, base32),
                "{plain}"
            );
            if !plain.is_empty() {
                assert_eq!(text_of(&read(base64, Some(ByteEncoding::Base64))), plain);
                assert_eq!(text_of(&read(base32, Some(ByteEncoding::Base32))), plain);
            }
        }
    }

    #[test]
    fn a_pasted_text_is_read_by_its_shape() {
        let cases = [
            ("01000011 01100001", ByteEncoding::Binary, "Ca"),
            ("0x43 0x61", ByteEncoding::Hex, "Ca"),
            ("43:61:66", ByteEncoding::Hex, "Caf"),
            ("67 97 102", ByteEncoding::Decimal, "Caf"),
            ("INQWM===", ByteEncoding::Base32, "Caf"),
            ("w6l0w6k_Pg", ByteEncoding::Base64Url, "été?>"),
            ("Q2Fmw6k=", ByteEncoding::Base64, "Café"),
        ];
        for (pasted, expected, plain) in cases {
            let answer = read(pasted, None);
            assert_eq!(answer.read_as, expected, "{pasted}");
            assert!(answer.guessed);
            assert_eq!(text_of(&answer), plain, "{pasted}");
        }
    }

    #[test]
    fn an_ambiguous_text_says_what_else_it_could_be_and_takes_a_forced_reading() {
        let answer = read("deadbeef", None);
        assert_eq!(answer.read_as, ByteEncoding::Hex);
        assert!(
            answer.also.contains(&ByteEncoding::Base64),
            "{:?}",
            answer.also
        );

        let forced = read("deadbeef", Some(ByteEncoding::Base64));
        assert_eq!(forced.read_as, ByteEncoding::Base64);
        assert!(!forced.guessed);
        assert!(forced.also.contains(&ByteEncoding::Hex));
        assert!(matches!(
            forced.decoded,
            DecodedBytes::Binary { bytes: 6, .. }
        ));
    }

    #[test]
    fn utf8_of_several_bytes_comes_back_whole() {
        let all = encoded("Привет 👋");
        assert_eq!(text_of(&read(&all.hex, None)), "Привет 👋");
        assert_eq!(text_of(&read(&all.base32, None)), "Привет 👋");
        assert_eq!(text_of(&read(&all.decimal, None)), "Привет 👋");
    }

    #[test]
    fn bytes_that_are_no_text_are_shown_in_hexadecimal() {
        assert_eq!(
            read("AP8Q", Some(ByteEncoding::Base64)).decoded,
            DecodedBytes::Binary {
                bytes: 3,
                preview: "00 ff 10".into()
            }
        );
    }

    #[test]
    fn a_refused_reading_names_the_character_and_where_it_stood() {
        assert_eq!(
            read("QUJD\nR#==", Some(ByteEncoding::Base64)).decoded,
            DecodedBytes::Invalid {
                problem: DecodeProblem::Character,
                at: Some(6),
                character: Some("#".into())
            }
        );
        assert_eq!(
            read("4g 61", Some(ByteEncoding::Hex)).decoded,
            DecodedBytes::Invalid {
                problem: DecodeProblem::Character,
                at: Some(1),
                character: Some("g".into())
            }
        );
        assert_eq!(
            read("12 300", Some(ByteEncoding::Decimal)).decoded,
            DecodedBytes::Invalid {
                problem: DecodeProblem::TooLarge,
                at: Some(3),
                character: Some("3".into())
            }
        );
        assert!(matches!(
            read("0100", Some(ByteEncoding::Binary)).decoded,
            DecodedBytes::Invalid {
                problem: DecodeProblem::Length,
                ..
            }
        ));
        assert_eq!(
            read("QUJD#", None).read_as,
            ByteEncoding::Base64,
            "nothing fits"
        );
    }

    #[test]
    fn a_file_is_encoded_where_it_lies_and_a_decoding_written_to_one() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("bytes.bin");
        std::fs::write(&source, [0u8, 255, 16]).unwrap();

        assert_eq!(
            encode_file(&source.to_string_lossy()),
            Base64FileAnswer::Encoded {
                name: "bytes.bin".into(),
                bytes: 3,
                base64: "AP8Q".into(),
                base64_url: "AP8Q".into(),
            }
        );
        let target = folder.path().join("out.bin");
        let request = |text: &str| DecodeRequest {
            text: text.to_owned(),
            reading: None,
        };
        assert_eq!(
            save(&request("00 ff 10"), &target.to_string_lossy()),
            SavedBytes::Saved { bytes: 3 }
        );
        assert_eq!(std::fs::read(&target).unwrap(), [0, 255, 16]);
        assert_eq!(
            save(&request("A#"), &target.to_string_lossy()),
            SavedBytes::Invalid
        );
    }
}
