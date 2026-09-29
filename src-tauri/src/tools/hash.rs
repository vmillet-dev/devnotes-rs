//! Digests and HMACs of a text or of a file, and which of them a pasted signature is.

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
    fn bits(self) -> u32 {
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
    pub input: HashInput,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Recognised {
    pub algorithm: HashAlgorithm,
    pub encoding: DigestEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HashAnswer {
    Hashed {
        bytes: u32,
        ends_with_newline: bool,
        digests: Vec<DigestValue>,
        /// `None` with an expected digest given: it is none of them.
        recognised: Option<Recognised>,
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

/// Hex in either case, Base64 with or without its padding.
fn recognise(expected: &str, computed: &[(HashAlgorithm, Vec<u8>)]) -> Option<Recognised> {
    let expected = expected.trim();
    let hex = expected.to_ascii_lowercase();
    let base64 = expected.trim_end_matches('=');

    computed.iter().find_map(|(algorithm, bytes)| {
        if encode(bytes, DigestEncoding::Hex) == hex {
            Some(Recognised {
                algorithm: *algorithm,
                encoding: DigestEncoding::Hex,
            })
        } else if encode(bytes, DigestEncoding::Base64).trim_end_matches('=') == base64 {
            Some(Recognised {
                algorithm: *algorithm,
                encoding: DigestEncoding::Base64,
            })
        } else {
            None
        }
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
    let fed = feed(&request.input, &mut sinks);
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
        recognised: request
            .expected
            .as_deref()
            .and_then(|expected| recognise(expected, &computed)),
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
            input: HashInput::Text {
                text: EVENT.to_owned(),
            },
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
            HashAnswer::Failed { problem } => panic!("failed: {problem:?}"),
        }
    }

    #[test]
    fn plain_digests_match_the_references() {
        let answer = hash(HashRequest {
            input: HashInput::Text {
                text: "abc".to_owned(),
            },
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
            input: HashInput::Text {
                text: FOX.to_owned(),
            },
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
            recognised,
            digests,
            ..
        } = answer
        else {
            panic!()
        };
        assert_eq!(
            recognised,
            Some(Recognised {
                algorithm: HashAlgorithm::Sha256,
                encoding: DigestEncoding::Hex
            })
        );
        assert_eq!(digests.len(), 1, "only what was asked for is sent");

        let abc_sha256_base64 = "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0";
        let answer = hash(HashRequest {
            input: HashInput::Text {
                text: "abc".to_owned(),
            },
            algorithms: vec![],
            encoding: DigestEncoding::Hex,
            key: None,
            expected: Some(abc_sha256_base64.to_owned()),
        });
        let HashAnswer::Hashed { recognised, .. } = answer else {
            panic!()
        };
        assert_eq!(
            recognised,
            Some(Recognised {
                algorithm: HashAlgorithm::Sha256,
                encoding: DigestEncoding::Base64
            })
        );
    }

    #[test]
    fn a_signature_of_nothing_known_is_said_to_be_none() {
        let HashAnswer::Hashed { recognised, .. } = text(&[], None, Some("deadbeef")) else {
            panic!()
        };

        assert_eq!(recognised, None);
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
            input: HashInput::File {
                path: path.to_string_lossy().into_owned(),
            },
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
            input: HashInput::Text {
                text: format!("{EVENT}\n"),
            },
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
            input: HashInput::File {
                path: tempfile::tempdir()
                    .unwrap()
                    .path()
                    .join("absent.bin")
                    .to_string_lossy()
                    .into_owned(),
            },
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
