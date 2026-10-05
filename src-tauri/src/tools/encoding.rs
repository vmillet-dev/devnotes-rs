//! The URL and Base64 encoders, both ways.

use base64::DecodeError;
use base64::Engine;
use base64::alphabet;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::files::{self, FileProblem};
use crate::count::saturating_u32;

/// What `encodeURIComponent` leaves alone: letters, digits and `-_.~`.
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// What `encodeURI` leaves alone besides: the characters that give a URL its structure.
const WHOLE: &AsciiSet = &COMPONENT
    .remove(b';')
    .remove(b',')
    .remove(b'/')
    .remove(b'?')
    .remove(b':')
    .remove(b'@')
    .remove(b'&')
    .remove(b'=')
    .remove(b'+')
    .remove(b'$')
    .remove(b'!')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'#');

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UrlScope {
    /// One value of a query or of a path: its `/`, `?` and `&` are escaped too.
    Component,
    /// A URL as a whole, whose structure stays readable.
    Whole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CodecDirection {
    Encode,
    Decode,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlCodecRequest {
    pub text: String,
    pub direction: CodecDirection,
    pub scope: UrlScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UrlCodecAnswer {
    Done {
        text: String,
    },
    /// `at` counts characters: a `%` without two hexadecimal digits after it.
    MalformedEscape {
        at: u32,
    },
    /// The escapes are well formed, but the bytes they spell are no UTF-8 text.
    NotUtf8,
}

pub fn url_codec(request: &UrlCodecRequest) -> UrlCodecAnswer {
    match request.direction {
        CodecDirection::Encode => {
            let set = match request.scope {
                UrlScope::Component => COMPONENT,
                UrlScope::Whole => WHOLE,
            };
            UrlCodecAnswer::Done {
                text: utf8_percent_encode(&request.text, set).to_string(),
            }
        }
        CodecDirection::Decode => decode_url(&request.text),
    }
}

fn hex(digit: u8) -> Option<u8> {
    char::from(digit)
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

fn decode_url(text: &str) -> UrlCodecAnswer {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).copied().and_then(hex);
            let low = bytes.get(index + 2).copied().and_then(hex);
            let (Some(high), Some(low)) = (high, low) else {
                return UrlCodecAnswer::MalformedEscape {
                    at: saturating_u32(text[..index].chars().count()),
                };
            };
            decoded.push(high << 4 | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map_or(UrlCodecAnswer::NotUtf8, |text| UrlCodecAnswer::Done {
        text,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Base64Alphabet {
    Standard,
    /// `-` and `_` for `+` and `/`: what a URL or a JWT carries.
    UrlSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Base64Options {
    pub alphabet: Base64Alphabet,
    /// Encoding only: a decoding takes a text padded or not.
    pub padded: bool,
}

impl Base64Options {
    fn engine(self) -> GeneralPurpose {
        let alphabet = match self.alphabet {
            Base64Alphabet::Standard => &alphabet::STANDARD,
            Base64Alphabet::UrlSafe => &alphabet::URL_SAFE,
        };
        GeneralPurpose::new(
            alphabet,
            GeneralPurposeConfig::new()
                .with_encode_padding(self.padded)
                .with_decode_padding_mode(DecodePaddingMode::Indifferent),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Base64FileAnswer {
    Encoded {
        name: String,
        bytes: u32,
        text: String,
    },
    Failed {
        problem: FileProblem,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Base64Problem {
    /// A character outside the alphabet, or padding in the middle.
    Character,
    /// A final group one character long: nothing can end that way.
    Length,
    /// A last character whose bits say the text was cut.
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Base64Decoded {
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
        problem: Base64Problem,
        at: Option<u32>,
        character: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Base64Saved {
    Saved { bytes: u32 },
    Invalid,
    Failed { problem: FileProblem },
}

/// How many bytes of a binary decoding are shown.
const PREVIEW_BYTES: usize = 48;

pub fn encode_base64(text: &str, options: Base64Options) -> String {
    options.engine().encode(text.as_bytes())
}

pub fn encode_base64_file(path: &str, options: Base64Options) -> Base64FileAnswer {
    match files::read_limited(path) {
        Ok(bytes) => Base64FileAnswer::Encoded {
            name: files::name_of(path),
            bytes: saturating_u32(bytes.len()),
            text: options.engine().encode(&bytes),
        },
        Err(problem) => Base64FileAnswer::Failed { problem },
    }
}

/// The characters that count, each with where it stood: a text wrapped at 76 columns decodes.
fn significant(text: &str) -> (String, Vec<usize>) {
    text.chars()
        .enumerate()
        .filter(|(_, character)| !character.is_whitespace())
        .map(|(position, character)| (character, position))
        .unzip()
}

fn bytes_of(text: &str, options: Base64Options) -> Result<Vec<u8>, Base64Decoded> {
    let (kept, positions) = significant(text);
    options.engine().decode(kept.as_bytes()).map_err(|error| {
        let (problem, offset) = match error {
            DecodeError::InvalidByte(offset, _) => (Base64Problem::Character, Some(offset)),
            DecodeError::InvalidLastSymbol { offset, .. } => {
                (Base64Problem::Truncated, Some(offset))
            }
            DecodeError::InvalidLength(_) => (Base64Problem::Length, None),
            DecodeError::InvalidPadding => (Base64Problem::Character, kept.find('=')),
        };
        // An offset is in bytes of `kept`: every byte that can be refused is ASCII, and so is
        // every one before it, so it is also a character index.
        let character = offset
            .and_then(|offset| kept[offset..].chars().next())
            .map(String::from);
        Base64Decoded::Invalid {
            problem,
            at: offset
                .and_then(|offset| positions.get(offset))
                .map(|&at| saturating_u32(at)),
            character,
        }
    })
}

pub fn decode_base64(text: &str, options: Base64Options) -> Base64Decoded {
    match bytes_of(text, options) {
        Ok(bytes) => {
            let count = saturating_u32(bytes.len());
            match String::from_utf8(bytes) {
                Ok(text) => Base64Decoded::Text { text, bytes: count },
                Err(error) => {
                    let bytes = error.into_bytes();
                    let preview = bytes
                        .iter()
                        .take(PREVIEW_BYTES)
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<Vec<_>>();
                    Base64Decoded::Binary {
                        bytes: count,
                        preview: preview.join(" "),
                    }
                }
            }
        }
        Err(invalid) => invalid,
    }
}

pub fn save_base64(text: &str, options: Base64Options, path: &str) -> Base64Saved {
    match bytes_of(text, options) {
        Ok(bytes) => match files::write(path, &bytes) {
            Ok(()) => Base64Saved::Saved {
                bytes: saturating_u32(bytes.len()),
            },
            Err(problem) => Base64Saved::Failed { problem },
        },
        Err(_) => Base64Saved::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codec(text: &str, direction: CodecDirection, scope: UrlScope) -> UrlCodecAnswer {
        url_codec(&UrlCodecRequest {
            text: text.to_owned(),
            direction,
            scope,
        })
    }

    fn done(text: &str) -> UrlCodecAnswer {
        UrlCodecAnswer::Done {
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_component_escapes_its_structure_and_a_whole_url_keeps_it() {
        let text = "https://exemple.fr/a b?q=café&x=1";

        assert_eq!(
            codec(text, CodecDirection::Encode, UrlScope::Component),
            done("https%3A%2F%2Fexemple.fr%2Fa%20b%3Fq%3Dcaf%C3%A9%26x%3D1")
        );
        assert_eq!(
            codec(text, CodecDirection::Encode, UrlScope::Whole),
            done("https://exemple.fr/a%20b?q=caf%C3%A9&x=1")
        );
    }

    #[test]
    fn decoding_reads_escapes_of_either_case_back_into_text() {
        assert_eq!(
            codec(
                "caf%C3%a9%20cr%C3%A8me",
                CodecDirection::Decode,
                UrlScope::Component
            ),
            done("café crème")
        );
    }

    #[test]
    fn a_malformed_escape_says_where_in_characters() {
        assert_eq!(
            codec("é/%2G", CodecDirection::Decode, UrlScope::Whole),
            UrlCodecAnswer::MalformedEscape { at: 2 }
        );
        assert_eq!(
            codec("50%", CodecDirection::Decode, UrlScope::Whole),
            UrlCodecAnswer::MalformedEscape { at: 2 }
        );
    }

    #[test]
    fn escapes_that_spell_no_text_say_so() {
        assert_eq!(
            codec("%C3%28", CodecDirection::Decode, UrlScope::Whole),
            UrlCodecAnswer::NotUtf8
        );
    }

    const STANDARD_PADDED: Base64Options = Base64Options {
        alphabet: Base64Alphabet::Standard,
        padded: true,
    };
    const URL_BARE: Base64Options = Base64Options {
        alphabet: Base64Alphabet::UrlSafe,
        padded: false,
    };

    #[test]
    fn base64_takes_its_alphabet_and_its_padding() {
        assert_eq!(encode_base64("été?>", STANDARD_PADDED), "w6l0w6k/Pg==");
        assert_eq!(encode_base64("été?>", URL_BARE), "w6l0w6k_Pg");
    }

    #[test]
    fn a_decoding_takes_padding_or_none_and_ignores_line_breaks() {
        assert_eq!(
            decode_base64("w6l0\nw6k_Pg", URL_BARE),
            Base64Decoded::Text {
                text: "été?>".to_owned(),
                bytes: 7
            }
        );
        assert_eq!(
            decode_base64(
                "w6l0w6k/Pg==",
                Base64Options {
                    padded: false,
                    ..STANDARD_PADDED
                }
            ),
            Base64Decoded::Text {
                text: "été?>".to_owned(),
                bytes: 7
            }
        );
    }

    #[test]
    fn bytes_that_are_no_text_are_shown_in_hexadecimal() {
        assert_eq!(
            decode_base64("AP8Q", STANDARD_PADDED),
            Base64Decoded::Binary {
                bytes: 3,
                preview: "00 ff 10".to_owned()
            }
        );
    }

    #[test]
    fn a_refused_decoding_names_the_character_and_where_it_stood() {
        assert_eq!(
            decode_base64("QUJD\nR#==", STANDARD_PADDED),
            Base64Decoded::Invalid {
                problem: Base64Problem::Character,
                at: Some(6),
                character: Some("#".to_owned())
            }
        );
        assert_eq!(
            decode_base64("w6l0w6k_Pg", STANDARD_PADDED),
            Base64Decoded::Invalid {
                problem: Base64Problem::Character,
                at: Some(7),
                character: Some("_".to_owned())
            }
        );
        assert!(matches!(
            decode_base64("QUJDR", STANDARD_PADDED),
            Base64Decoded::Invalid {
                problem: Base64Problem::Length,
                ..
            }
        ));
    }

    #[test]
    fn a_file_is_encoded_where_it_lies_and_a_decoding_written_to_one() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("logo.bin");
        std::fs::write(&source, [0u8, 255, 16]).unwrap();

        assert_eq!(
            encode_base64_file(&source.to_string_lossy(), STANDARD_PADDED),
            Base64FileAnswer::Encoded {
                name: "logo.bin".to_owned(),
                bytes: 3,
                text: "AP8Q".to_owned()
            }
        );

        let target = folder.path().join("back.bin");
        assert_eq!(
            save_base64("AP8Q", STANDARD_PADDED, &target.to_string_lossy()),
            Base64Saved::Saved { bytes: 3 }
        );
        assert_eq!(std::fs::read(&target).unwrap(), [0, 255, 16]);
        assert_eq!(
            save_base64("A#", STANDARD_PADDED, &target.to_string_lossy()),
            Base64Saved::Invalid
        );
    }
}
