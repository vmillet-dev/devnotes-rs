//! The URL encoder, both ways.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use specta::Type;

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
}
