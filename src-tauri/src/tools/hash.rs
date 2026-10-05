//! Digests and HMACs of a text or of a file, which of them a pasted signature is, and what it
//! looks like when there is nothing to compare it with.

use std::fmt::Write;
use std::io::Read;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{KeyInit, Mac, SimpleHmac};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use specta::Type;
use zeroize::Zeroize;

use super::files::{self, FileProblem};
use crate::count::saturating_u32;

/// In the order the tool lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
pub enum HashAlgorithm {
    #[serde(rename = "md5")]
    Md5,
    #[serde(rename = "sha1")]
    Sha1,
    #[serde(rename = "sha256")]
    Sha256,
    #[serde(rename = "sha384")]
    Sha384,
    #[serde(rename = "sha512")]
    Sha512,
    #[serde(rename = "sha3-256")]
    Sha3_256,
}

const ALGORITHMS: [HashAlgorithm; 6] = [
    HashAlgorithm::Md5,
    HashAlgorithm::Sha1,
    HashAlgorithm::Sha256,
    HashAlgorithm::Sha384,
    HashAlgorithm::Sha512,
    HashAlgorithm::Sha3_256,
];

impl HashAlgorithm {
    const fn bytes(self) -> usize {
        self.bits() as usize / 8
    }

    const fn bits(self) -> u32 {
        match self {
            Self::Md5 => 128,
            Self::Sha1 => 160,
            Self::Sha256 | Self::Sha3_256 => 256,
            Self::Sha384 => 384,
            Self::Sha512 => 512,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DigestEncoding {
    Hex,
    Base64,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HashInput {
    Text {
        text: String,
    },
    /// Read here, a block at a time: its bytes never cross the bridge, whatever its size.
    File {
        path: String,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HashRequest {
    /// `None` to read a pasted digest's shape alone.
    pub input: Option<HashInput>,
    pub algorithms: Vec<HashAlgorithm>,
    pub encoding: DigestEncoding,
    /// An HMAC when present. ⚠️ Zeroed once the digests are computed, and never stored.
    pub key: Option<String>,
    /// A digest to recognise, in any of the algorithms and either encoding.
    pub expected: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DigestValue {
    pub algorithm: HashAlgorithm,
    pub bits: u32,
    pub value: String,
}

/// What a digest is from its length alone: an HMAC has the shape of the hash it uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DigestShape {
    Hex {
        characters: u32,
        /// Empty when no digest has this length.
        algorithms: Vec<HashAlgorithm>,
    },
    Base64 {
        bytes: u32,
        algorithms: Vec<HashAlgorithm>,
    },
    /// Neither alphabet.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DigestVerdict {
    Matches {
        algorithm: HashAlgorithm,
        encoding: DigestEncoding,
    },
    NoMatch {
        shape: DigestShape,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HashAnswer {
    Hashed {
        bytes: u32,
        ends_with_newline: bool,
        digests: Vec<DigestValue>,
        /// `None` when no digest was pasted.
        verdict: Option<DigestVerdict>,
    },
    /// No input: what the pasted digest is, by its shape.
    Shaped {
        shape: DigestShape,
    },
    Failed {
        problem: FileProblem,
    },
}

/// One digest being computed, keyed or not: the input is read once for all of them.
trait Sink {
    fn update(&mut self, data: &[u8]);
    fn finish(self: Box<Self>) -> Vec<u8>;
}

struct Plain<D>(D);

impl<D: Digest> Sink for Plain<D> {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

struct Keyed<M>(M);

impl<M: Mac> Sink for Keyed<M> {
    fn update(&mut self, data: &[u8]) {
        Mac::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().into_bytes().to_vec()
    }
}

/// `SimpleHmac` rather than `Hmac`: it takes every one of these, SHA-3 included.
fn sink(algorithm: HashAlgorithm, key: Option<&[u8]>) -> Box<dyn Sink> {
    macro_rules! either {
        ($digest:ty) => {
            match key {
                None => Box::new(Plain(<$digest>::new())),
                // A `SimpleHmac` takes a key of any length, so this cannot fail.
                Some(key) => Box::new(Keyed(
                    SimpleHmac::<$digest>::new_from_slice(key).expect("any key length"),
                )),
            }
        };
    }
    match algorithm {
        HashAlgorithm::Md5 => either!(md5::Md5),
        HashAlgorithm::Sha1 => either!(sha1::Sha1),
        HashAlgorithm::Sha256 => either!(sha2::Sha256),
        HashAlgorithm::Sha384 => either!(sha2::Sha384),
        HashAlgorithm::Sha512 => either!(sha2::Sha512),
        HashAlgorithm::Sha3_256 => either!(sha3::Sha3_256),
    }
}

fn encode(bytes: &[u8], encoding: DigestEncoding) -> String {
    match encoding {
        DigestEncoding::Hex => {
            bytes
                .iter()
                .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
                    let _ = write!(hex, "{byte:02x}");
                    hex
                })
        }
        DigestEncoding::Base64 => STANDARD.encode(bytes),
    }
}

/// Hex in either case, spaced or split by colons (`openssl -c`); Base64 with or without padding.
fn as_hex(expected: &str) -> Option<String> {
    let hex: String = expected
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    (!hex.is_empty() && hex.len().is_multiple_of(2) && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then_some(hex)
}

fn as_base64(expected: &str) -> Option<String> {
    let base64: String = expected.chars().filter(|c| !c.is_whitespace()).collect();
    let base64 = base64.trim_end_matches('=');
    let alphabet = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_');
    // One character past a group of four carries six bits: not a whole byte.
    (!base64.is_empty() && base64.len() % 4 != 1 && base64.bytes().all(alphabet))
        .then(|| base64.replace('-', "+").replace('_', "/"))
}

fn of_length(bytes: usize) -> Vec<HashAlgorithm> {
    ALGORITHMS
        .into_iter()
        .filter(|algorithm| algorithm.bytes() == bytes)
        .collect()
}

/// Hex first: a hex digest is also valid Base64, and hex is what digests are pasted as.
fn shape(expected: &str) -> DigestShape {
    if let Some(hex) = as_hex(expected) {
        DigestShape::Hex {
            characters: saturating_u32(hex.len() as u64),
            algorithms: of_length(hex.len() / 2),
        }
    } else if let Some(base64) = as_base64(expected) {
        let bytes = base64.len() * 3 / 4;
        DigestShape::Base64 {
            bytes: saturating_u32(bytes as u64),
            algorithms: of_length(bytes),
        }
    } else {
        DigestShape::Unknown
    }
}

fn judge(expected: &str, computed: &[(HashAlgorithm, Vec<u8>)]) -> DigestVerdict {
    let hex = as_hex(expected);
    let base64 = as_base64(expected);
    computed
        .iter()
        .find_map(|(algorithm, bytes)| {
            let matches = |encoding, wanted: &Option<String>| {
                wanted
                    .as_deref()
                    .is_some_and(|wanted| encode(bytes, encoding).trim_end_matches('=') == wanted)
            };
            if matches(DigestEncoding::Hex, &hex) {
                Some(DigestEncoding::Hex)
            } else if matches(DigestEncoding::Base64, &base64) {
                Some(DigestEncoding::Base64)
            } else {
                None
            }
            .map(|encoding| DigestVerdict::Matches {
                algorithm: *algorithm,
                encoding,
            })
        })
        .unwrap_or_else(|| DigestVerdict::NoMatch {
            shape: shape(expected),
        })
}

/// Every algorithm asked for, and all of them when a digest is to be recognised.
fn feed(input: &HashInput, sinks: &mut [Box<dyn Sink>]) -> Result<(u64, bool), FileProblem> {
    match input {
        HashInput::Text { text } => {
            for sink in sinks.iter_mut() {
                sink.update(text.as_bytes());
            }
            Ok((text.len() as u64, text.ends_with('\n')))
        }
        HashInput::File { path } => {
            let mut file = files::open(path)?;
            let mut buffer = vec![0u8; 64 * 1024];
            let (mut total, mut last) = (0u64, None);
            loop {
                let read = file
                    .read(&mut buffer)
                    .map_err(|_| FileProblem::Unreadable)?;
                if read == 0 {
                    break;
                }
                for sink in sinks.iter_mut() {
                    sink.update(&buffer[..read]);
                }
                total += read as u64;
                last = buffer[..read].last().copied();
            }
            Ok((total, last == Some(b'\n')))
        }
    }
}

pub fn hash(mut request: HashRequest) -> HashAnswer {
    let expected = request
        .expected
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let Some(input) = &request.input else {
        if let Some(key) = request.key.as_mut() {
            key.zeroize();
        }
        return HashAnswer::Shaped {
            shape: expected.map_or(DigestShape::Unknown, shape),
        };
    };
    let computed_for: Vec<HashAlgorithm> = if expected.is_some() {
        ALGORITHMS.to_vec()
    } else {
        ALGORITHMS
            .into_iter()
            .filter(|algorithm| request.algorithms.contains(algorithm))
            .collect()
    };

    let key = request.key.as_deref().map(str::as_bytes);
    let mut sinks: Vec<Box<dyn Sink>> = computed_for
        .iter()
        .map(|&algorithm| sink(algorithm, key))
        .collect();
    let fed = feed(input, &mut sinks);
    if let Some(key) = request.key.as_mut() {
        key.zeroize();
    }

    let (bytes, ends_with_newline) = match fed {
        Ok(fed) => fed,
        Err(problem) => return HashAnswer::Failed { problem },
    };
    let computed: Vec<(HashAlgorithm, Vec<u8>)> = computed_for
        .into_iter()
        .zip(sinks)
        .map(|(algorithm, sink)| (algorithm, sink.finish()))
        .collect();

    HashAnswer::Hashed {
        bytes: saturating_u32(bytes),
        ends_with_newline,
        verdict: expected.map(|expected| judge(expected, &computed)),
        digests: computed
            .iter()
            .filter(|(algorithm, _)| request.algorithms.contains(algorithm))
            .map(|(algorithm, bytes)| DigestValue {
                algorithm: *algorithm,
                bits: algorithm.bits(),
                value: encode(bytes, request.encoding),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT: &str = r#"{"id":"evt_1PqX4c","type":"invoice.paid","created":1727517600}"#;

    fn text(algorithms: &[HashAlgorithm], key: Option<&str>, expected: Option<&str>) -> HashAnswer {
        hash(HashRequest {
            input: Some(HashInput::Text {
                text: EVENT.to_owned(),
            }),
            algorithms: algorithms.to_vec(),
            encoding: DigestEncoding::Hex,
            key: key.map(str::to_owned),
            expected: expected.map(str::to_owned),
        })
    }

    fn digests(answer: &HashAnswer) -> Vec<(HashAlgorithm, &str)> {
        match answer {
            HashAnswer::Hashed { digests, .. } => digests
                .iter()
                .map(|digest| (digest.algorithm, digest.value.as_str()))
                .collect(),
            other => panic!("not hashed: {other:?}"),
        }
    }

    #[test]
    fn plain_digests_match_the_references() {
        let answer = hash(HashRequest {
            input: Some(HashInput::Text {
                text: "abc".to_owned(),
            }),
            algorithms: ALGORITHMS.to_vec(),
            encoding: DigestEncoding::Hex,
            key: None,
            expected: None,
        });

        let found = digests(&answer);
        assert_eq!(
            found[0],
            (HashAlgorithm::Md5, "900150983cd24fb0d6963f7d28e17f72")
        );
        assert_eq!(
            found[1],
            (
                HashAlgorithm::Sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d"
            )
        );
        assert_eq!(
            found[2],
            (
                HashAlgorithm::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
        );
        assert_eq!(
            found[5],
            (
                HashAlgorithm::Sha3_256,
                "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
            )
        );
    }

    const FOX: &str = "The quick brown fox jumps over the lazy dog";
    const FOX_HMAC_SHA256: &str =
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8";

    fn fox(algorithms: &[HashAlgorithm], expected: Option<&str>) -> HashAnswer {
        hash(HashRequest {
            input: Some(HashInput::Text {
                text: FOX.to_owned(),
            }),
            algorithms: algorithms.to_vec(),
            encoding: DigestEncoding::Hex,
            key: Some("key".to_owned()),
            expected: expected.map(str::to_owned),
        })
    }

    #[test]
    fn an_hmac_matches_the_reference() {
        assert_eq!(
            digests(&fox(&[HashAlgorithm::Sha256, HashAlgorithm::Md5], None)),
            [
                (HashAlgorithm::Md5, "80070713463e7749b90c2dc24911e275"),
                (HashAlgorithm::Sha256, FOX_HMAC_SHA256),
            ]
        );
    }

    #[test]
    fn a_pasted_signature_is_recognised_in_any_algorithm_and_encoding() {
        let answer = fox(
            &[HashAlgorithm::Sha1],
            Some(&FOX_HMAC_SHA256.to_ascii_uppercase()),
        );

        let HashAnswer::Hashed {
            verdict, digests, ..
        } = answer
        else {
            panic!()
        };
        assert_eq!(
            verdict,
            Some(DigestVerdict::Matches {
                algorithm: HashAlgorithm::Sha256,
                encoding: DigestEncoding::Hex
            })
        );
        assert_eq!(digests.len(), 1, "only what was asked for is sent");

        let abc_sha256_base64 = "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0";
        let answer = hash(HashRequest {
            input: Some(HashInput::Text {
                text: "abc".to_owned(),
            }),
            algorithms: vec![],
            encoding: DigestEncoding::Hex,
            key: None,
            expected: Some(abc_sha256_base64.to_owned()),
        });
        let HashAnswer::Hashed { verdict, .. } = answer else {
            panic!()
        };
        assert_eq!(
            verdict,
            Some(DigestVerdict::Matches {
                algorithm: HashAlgorithm::Sha256,
                encoding: DigestEncoding::Base64
            })
        );
    }

    #[test]
    fn a_signature_split_by_colons_or_spaces_is_still_recognised() {
        let pairs: Vec<String> = FOX_HMAC_SHA256
            .as_bytes()
            .chunks(2)
            .map(|pair| String::from_utf8(pair.to_vec()).unwrap())
            .collect();

        for pasted in [pairs.join(":"), pairs.join(" ").to_ascii_uppercase()] {
            let HashAnswer::Hashed { verdict, .. } = fox(&[], Some(&pasted)) else {
                panic!()
            };
            assert!(
                matches!(
                    verdict,
                    Some(DigestVerdict::Matches {
                        algorithm: HashAlgorithm::Sha256,
                        ..
                    })
                ),
                "{pasted}"
            );
        }
    }

    #[test]
    fn a_signature_of_nothing_known_is_no_match_and_says_its_shape() {
        let HashAnswer::Hashed { verdict, .. } = text(&[], None, Some(FOX_HMAC_SHA256)) else {
            panic!()
        };

        assert_eq!(
            verdict,
            Some(DigestVerdict::NoMatch {
                shape: DigestShape::Hex {
                    characters: 64,
                    algorithms: vec![HashAlgorithm::Sha256, HashAlgorithm::Sha3_256]
                }
            })
        );
    }

    fn shaped(expected: &str) -> DigestShape {
        match hash(HashRequest {
            input: None,
            algorithms: ALGORITHMS.to_vec(),
            encoding: DigestEncoding::Hex,
            key: None,
            expected: Some(expected.to_owned()),
        }) {
            HashAnswer::Shaped { shape } => shape,
            other => panic!("{other:?}"),
        }
    }

    fn hex(characters: u32, algorithms: &[HashAlgorithm]) -> DigestShape {
        DigestShape::Hex {
            characters,
            algorithms: algorithms.to_vec(),
        }
    }

    fn base64(bytes: u32, algorithms: &[HashAlgorithm]) -> DigestShape {
        DigestShape::Base64 {
            bytes,
            algorithms: algorithms.to_vec(),
        }
    }

    #[test]
    fn with_nothing_to_compare_a_hex_digest_is_read_by_its_length() {
        use HashAlgorithm::{Md5, Sha1, Sha3_256, Sha256, Sha384, Sha512};

        assert_eq!(shaped(&"a".repeat(32)), hex(32, &[Md5]));
        assert_eq!(shaped(&"B".repeat(40)), hex(40, &[Sha1]));
        assert_eq!(shaped(FOX_HMAC_SHA256), hex(64, &[Sha256, Sha3_256]));
        assert_eq!(shaped(&"0".repeat(96)), hex(96, &[Sha384]));
        assert_eq!(shaped(&"f".repeat(128)), hex(128, &[Sha512]));
        assert_eq!(shaped("de:ad:BE:EF"), hex(8, &[]), "a length no digest has");
    }

    #[test]
    fn with_nothing_to_compare_a_base64_digest_is_read_by_its_decoded_length() {
        use HashAlgorithm::{Md5, Sha1, Sha3_256, Sha256, Sha384, Sha512};

        assert_eq!(shaped("kAFQmDzST7DWlj99KOF/cg=="), base64(16, &[Md5]));
        assert_eq!(shaped("qZk+NkcGgWq6PiVxeFDCbJzQ2J0="), base64(20, &[Sha1]));
        assert_eq!(
            shaped("ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0"),
            base64(32, &[Sha256, Sha3_256]),
            "unpadded"
        );
        assert_eq!(shaped(&"x".repeat(64)), base64(48, &[Sha384]));
        assert_eq!(
            shaped(&format!("{}==", "_".repeat(86))),
            base64(64, &[Sha512])
        );
        assert_eq!(
            shaped("abcde"),
            DigestShape::Unknown,
            "a sixth of a byte left over"
        );
        assert_eq!(shaped("not a digest!"), DigestShape::Unknown);
    }

    #[test]
    fn nothing_to_hash_and_nothing_pasted_is_an_unknown_shape() {
        let answer = hash(HashRequest {
            input: None,
            algorithms: vec![],
            encoding: DigestEncoding::Hex,
            key: Some("key".to_owned()),
            expected: None,
        });

        assert_eq!(
            answer,
            HashAnswer::Shaped {
                shape: DigestShape::Unknown
            }
        );
    }

    #[test]
    fn the_input_is_measured_and_its_final_newline_seen() {
        let HashAnswer::Hashed {
            bytes,
            ends_with_newline,
            ..
        } = text(&[], None, None)
        else {
            panic!()
        };
        assert_eq!((bytes, ends_with_newline), (62, false));
    }

    #[test]
    fn a_file_is_read_where_it_lies_and_digested_like_its_text() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("event.json");
        std::fs::write(&path, format!("{EVENT}\n")).unwrap();

        let answer = hash(HashRequest {
            input: Some(HashInput::File {
                path: path.to_string_lossy().into_owned(),
            }),
            algorithms: vec![HashAlgorithm::Md5],
            encoding: DigestEncoding::Base64,
            key: None,
            expected: None,
        });
        let HashAnswer::Hashed {
            bytes,
            ends_with_newline,
            digests,
            ..
        } = answer
        else {
            panic!()
        };
        assert_eq!((bytes, ends_with_newline), (63, true));
        let text_answer = hash(HashRequest {
            input: Some(HashInput::Text {
                text: format!("{EVENT}\n"),
            }),
            algorithms: vec![HashAlgorithm::Md5],
            encoding: DigestEncoding::Base64,
            key: None,
            expected: None,
        });
        let HashAnswer::Hashed {
            digests: from_text, ..
        } = text_answer
        else {
            panic!()
        };
        assert_eq!(digests, from_text);
    }

    #[test]
    fn a_missing_file_says_so() {
        let answer = hash(HashRequest {
            input: Some(HashInput::File {
                path: tempfile::tempdir()
                    .unwrap()
                    .path()
                    .join("absent.bin")
                    .to_string_lossy()
                    .into_owned(),
            }),
            algorithms: vec![HashAlgorithm::Md5],
            encoding: DigestEncoding::Hex,
            key: None,
            expected: None,
        });

        assert_eq!(
            answer,
            HashAnswer::Failed {
                problem: FileProblem::NotFound
            }
        );
    }
}
