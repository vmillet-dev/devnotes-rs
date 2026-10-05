//! Identifiers drawn from the system's generator — UUID v4 and v7, ULID, `NanoID`, CUID2, the
//! `ObjectId` of `MongoDB`, KSUID — and read back: a pasted identifier is recognised by its shape, and the
//! instant it carries is given.

use std::fmt::Write;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use specta::Type;
use uuid::{Uuid, Variant};

use super::random::{NoRandomness, uniform};

pub const MAX_IDENTIFIERS: u32 = 500;
pub const NANOID_LENGTHS: (u32, u32) = (2, 255);

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE36: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
/// KSUID counts its seconds from here, 2014-05-13, to last past 2150.
const KSUID_EPOCH: i64 = 1_400_000_000;
const CUID2_LENGTH: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IdKind {
    UuidV4,
    UuidV7,
    Ulid,
    NanoId,
    Cuid2,
    ObjectId,
    Ksuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NanoAlphabet {
    /// `A-Za-z0-9_-`, `NanoID`'s own.
    UrlSafe,
    Alphanumeric,
    HexLower,
    Digits,
}

impl NanoAlphabet {
    fn characters(self) -> &'static [u8] {
        match self {
            Self::UrlSafe => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-",
            Self::Alphanumeric => BASE62,
            Self::HexLower => b"0123456789abcdef",
            Self::Digits => b"0123456789",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiersRequest {
    pub kind: IdKind,
    pub count: u32,
    /// For the kinds whose case carries nothing: UUID, ULID, `ObjectId`.
    pub uppercase: bool,
    /// UUID only: `false` writes the 32 digits without their four hyphens.
    pub hyphens: bool,
    pub nano_length: u32,
    pub nano_alphabet: NanoAlphabet,
}

fn random_bytes<const N: usize>() -> Result<[u8; N], NoRandomness> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|_| NoRandomness)?;
    Ok(bytes)
}

/// Bytes as hexadecimal digits, in the case asked.
fn hex(bytes: &[u8], upper: bool) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = if upper {
            write!(text, "{byte:02X}")
        } else {
            write!(text, "{byte:02x}")
        };
        text
    })
}

/// A big-endian number written in `alphabet`'s base, at least `width` digits wide.
fn in_base(bytes: &[u8], alphabet: &[u8], width: usize) -> String {
    let base = u32::try_from(alphabet.len()).unwrap_or(u32::MAX);
    let mut number: Vec<u8> = bytes.to_vec();
    let mut digits = Vec::new();
    while number.iter().any(|byte| *byte != 0) {
        let mut remainder = 0u32;
        for byte in &mut number {
            let value = (remainder << 8) | u32::from(*byte);
            *byte = u8::try_from(value / base).unwrap_or(0);
            remainder = value % base;
        }
        digits.push(alphabet[remainder as usize]);
    }
    while digits.len() < width {
        digits.push(alphabet[0]);
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

/// `text` read in `alphabet`'s base as a big-endian number of `size` bytes, or `None` past it.
fn from_base(text: &str, alphabet: &[u8], size: usize) -> Option<Vec<u8>> {
    let base = u32::try_from(alphabet.len()).ok()?;
    let mut number = vec![0u8; size];
    for character in text.bytes() {
        let digit = u32::try_from(alphabet.iter().position(|a| *a == character)?).ok()?;
        let mut carry = digit;
        for byte in number.iter_mut().rev() {
            let value = u32::from(*byte) * base + carry;
            *byte = u8::try_from(value & 0xff).ok()?;
            carry = value >> 8;
        }
        if carry != 0 {
            return None;
        }
    }
    Some(number)
}

fn now_millis() -> u64 {
    u64::try_from(Utc::now().timestamp_millis()).unwrap_or(0)
}

/// A batch of ULIDs within one millisecond counts up, as the spec asks, so they sort as drawn.
fn ulids(count: u32) -> Result<Vec<String>, NoRandomness> {
    let mut previous: Option<(u64, u128)> = None;
    (0..count)
        .map(|_| {
            let time = now_millis() & 0xFFFF_FFFF_FFFF;
            let random = match previous {
                Some((last, random)) if last >= time => random.wrapping_add(1) & ((1 << 80) - 1),
                _ => {
                    let bytes: [u8; 10] = random_bytes()?;
                    bytes
                        .iter()
                        .fold(0u128, |sum, byte| (sum << 8) | u128::from(*byte))
                }
            };
            let time = previous.map_or(time, |(last, _)| last.max(time));
            previous = Some((time, random));
            let value = (u128::from(time) << 80) | random;
            Ok(in_base(&value.to_be_bytes(), CROCKFORD, 26)[..].to_owned())
        })
        .collect()
}

fn nanoids(count: u32, length: u32, alphabet: NanoAlphabet) -> Result<Vec<String>, NoRandomness> {
    let characters = alphabet.characters();
    let bound = u32::try_from(characters.len()).unwrap_or(u32::MAX);
    let length = length.clamp(NANOID_LENGTHS.0, NANOID_LENGTHS.1);
    (0..count)
        .map(|_| {
            (0..length)
                .map(|_| uniform(bound).map(|index| char::from(characters[index as usize])))
                .collect()
        })
        .collect()
}

/// A letter, then a SHA3-512 of the time, fresh entropy, a counter and a fingerprint, in base 36.
fn cuid2s(count: u32) -> Result<Vec<String>, NoRandomness> {
    let fingerprint: [u8; 32] = random_bytes()?;
    let start: [u8; 4] = random_bytes()?;
    let mut counter = u32::from_be_bytes(start) % 476_782_367;
    (0..count)
        .map(|_| {
            let entropy: [u8; 32] = random_bytes()?;
            counter = counter.wrapping_add(1);
            let mut hasher = Sha3_512::new();
            hasher.update(now_millis().to_be_bytes());
            hasher.update(entropy);
            hasher.update(counter.to_be_bytes());
            hasher.update(fingerprint);
            let digest = in_base(&hasher.finalize(), BASE36, 0);
            let letter = BASE36[10 + uniform(26)? as usize];
            Ok(format!(
                "{}{}",
                char::from(letter),
                &digest[1..CUID2_LENGTH]
            ))
        })
        .collect()
}

/// Four bytes of seconds, five drawn once for the batch, three of a counter from a random start.
fn object_ids(count: u32) -> Result<Vec<String>, NoRandomness> {
    let process: [u8; 5] = random_bytes()?;
    let start: [u8; 3] = random_bytes()?;
    let mut counter = u32::from_be_bytes([0, start[0], start[1], start[2]]);
    let seconds = u32::try_from(Utc::now().timestamp()).unwrap_or(u32::MAX);
    Ok((0..count)
        .map(|_| {
            counter = (counter + 1) & 0x00FF_FFFF;
            let mut bytes = Vec::with_capacity(12);
            bytes.extend_from_slice(&seconds.to_be_bytes());
            bytes.extend_from_slice(&process);
            bytes.extend_from_slice(&counter.to_be_bytes()[1..]);
            hex(&bytes, false)
        })
        .collect())
}

fn ksuids(count: u32) -> Result<Vec<String>, NoRandomness> {
    let seconds = u32::try_from(Utc::now().timestamp() - KSUID_EPOCH).unwrap_or(0);
    (0..count)
        .map(|_| {
            let payload: [u8; 16] = random_bytes()?;
            let mut bytes = seconds.to_be_bytes().to_vec();
            bytes.extend_from_slice(&payload);
            Ok(in_base(&bytes, BASE62, 27))
        })
        .collect()
}

fn uuids(count: u32, version: IdKind, hyphens: bool) -> Result<Vec<String>, NoRandomness> {
    (0..count)
        .map(|_| {
            let uuid = if version == IdKind::UuidV7 {
                Uuid::now_v7()
            } else {
                uuid::Builder::from_random_bytes(random_bytes()?).into_uuid()
            };
            Ok(if hyphens {
                uuid.hyphenated().to_string()
            } else {
                uuid.simple().to_string()
            })
        })
        .collect()
}

pub fn generate(request: &IdentifiersRequest) -> Result<Vec<String>, NoRandomness> {
    let count = request.count.clamp(1, MAX_IDENTIFIERS);
    let drawn = match request.kind {
        IdKind::UuidV4 | IdKind::UuidV7 => uuids(count, request.kind, request.hyphens)?,
        IdKind::Ulid => ulids(count)?,
        IdKind::NanoId => nanoids(count, request.nano_length, request.nano_alphabet)?,
        IdKind::Cuid2 => cuid2s(count)?,
        IdKind::ObjectId => object_ids(count)?,
        IdKind::Ksuid => ksuids(count)?,
    };
    let caseless = matches!(
        request.kind,
        IdKind::UuidV4 | IdKind::UuidV7 | IdKind::Ulid | IdKind::ObjectId
    );
    Ok(drawn
        .into_iter()
        .map(|id| match (caseless, request.uppercase) {
            (true, true) => id.to_uppercase(),
            (true, false) => id.to_lowercase(),
            (false, _) => id,
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UuidVariant {
    Ncs,
    /// RFC 9562, which every UUID made today follows.
    Rfc,
    Microsoft,
    Future,
}

/// What was drawn rather than stamped: the characters as pasted, and the bits they hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RandomPart {
    pub text: String,
    pub bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IdInspection {
    Uuid {
        version: u32,
        variant: UuidVariant,
        /// v1, v6 and v7 carry the time they were made, in UTC.
        created: Option<String>,
        nil: bool,
        /// v4 and v7 alone: the other versions draw nothing, or not where it can be told.
        random: Option<RandomPart>,
    },
    Ulid {
        created: String,
        random: RandomPart,
    },
    ObjectId {
        created: String,
        counter: u32,
        /// Drawn once per process, not per identifier.
        random: RandomPart,
    },
    Ksuid {
        created: String,
        random: RandomPart,
    },
    /// The first CUID, deprecated by its author: its time is in plain sight.
    CuidV1 {
        created: String,
    },
    /// The shape of an identifier that carries nothing to read.
    Possible {
        kinds: Vec<IdKind>,
    },
    /// The shape fits, the value does not: a ULID past `7ZZZZZZZZZ…`.
    OutOfRange {
        id: IdKind,
    },
    Unrecognised,
}

fn instant(millis: i64) -> Option<String> {
    DateTime::from_timestamp_millis(millis)
        .map(|at| at.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn drawn_part(text: &str, bits: u32) -> RandomPart {
    RandomPart {
        text: text.to_owned(),
        bits,
    }
}

/// `text` is the UUID as pasted, hyphens or not: its random part is quoted in its own form.
fn inspect_uuid(uuid: Uuid, text: &str) -> IdInspection {
    let variant = match uuid.get_variant() {
        Variant::NCS => UuidVariant::Ncs,
        Variant::RFC4122 => UuidVariant::Rfc,
        Variant::Microsoft => UuidVariant::Microsoft,
        _ => UuidVariant::Future,
    };
    let created = uuid.get_timestamp().and_then(|timestamp| {
        let (seconds, nanos) = timestamp.to_unix();
        DateTime::<Utc>::from_timestamp(i64::try_from(seconds).ok()?, nanos)
            .map(|at| at.to_rfc3339_opts(SecondsFormat::Millis, true))
    });
    let version = uuid.get_version_num();
    // After v7's 48 bits of milliseconds: 12 + 62 drawn bits, the version and variant between.
    let after_time = if text.contains('-') { 14 } else { 12 };
    let random = match version {
        4 => Some(drawn_part(text, 122)),
        7 => text.get(after_time..).map(|rest| drawn_part(rest, 74)),
        _ => None,
    }
    .filter(|_| !uuid.is_nil());
    IdInspection::Uuid {
        version: u32::try_from(version).unwrap_or(0),
        variant,
        created,
        nil: uuid.is_nil(),
        random,
    }
}

pub fn inspect(text: &str) -> IdInspection {
    let text = text.trim();
    if let Ok(uuid) = Uuid::parse_str(text) {
        return inspect_uuid(uuid, text);
    }
    let length = text.len();
    let only = |alphabet: &[u8]| text.bytes().all(|byte| alphabet.contains(&byte));

    let upper = text.to_ascii_uppercase();
    if length == 26 && upper.bytes().all(|byte| CROCKFORD.contains(&byte)) {
        return match from_base(&upper[..10], CROCKFORD, 8)
            .map(|bytes| i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8])))
            .filter(|_| upper.as_bytes()[0] <= b'7')
            .and_then(instant)
        {
            Some(created) => IdInspection::Ulid {
                created,
                random: drawn_part(&upper[10..], 80),
            },
            None => IdInspection::OutOfRange { id: IdKind::Ulid },
        };
    }
    if length == 24 && text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        let seconds = i64::from_str_radix(&text[..8], 16).unwrap_or(0);
        return IdInspection::ObjectId {
            created: instant(seconds * 1000).unwrap_or_default(),
            counter: u32::from_str_radix(&text[18..], 16).unwrap_or(0),
            random: drawn_part(&text[8..18], 40),
        };
    }
    if length == 27 && only(BASE62) {
        return match from_base(text, BASE62, 20) {
            Some(bytes) => {
                let seconds =
                    i64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                IdInspection::Ksuid {
                    created: instant((seconds + KSUID_EPOCH) * 1000).unwrap_or_default(),
                    random: drawn_part(&hex(&bytes[4..], true), 128),
                }
            }
            None => IdInspection::OutOfRange { id: IdKind::Ksuid },
        };
    }
    if length == 25
        && text.starts_with('c')
        && only(BASE36)
        && let Some(created) = i64::from_str_radix(&text[1..9], 36).ok().and_then(instant)
    {
        return IdInspection::CuidV1 { created };
    }

    let mut kinds = Vec::new();
    if length == CUID2_LENGTH && text.starts_with(|c: char| c.is_ascii_lowercase()) && only(BASE36)
    {
        kinds.push(IdKind::Cuid2);
    }
    if (NANOID_LENGTHS.0 as usize..=NANOID_LENGTHS.1 as usize).contains(&length)
        && only(NanoAlphabet::UrlSafe.characters())
        && length >= 8
    {
        kinds.push(IdKind::NanoId);
    }
    if kinds.is_empty() {
        IdInspection::Unrecognised
    } else {
        IdInspection::Possible { kinds }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: IdKind, count: u32) -> IdentifiersRequest {
        IdentifiersRequest {
            kind,
            count,
            uppercase: false,
            hyphens: true,
            nano_length: 21,
            nano_alphabet: NanoAlphabet::UrlSafe,
        }
    }

    fn drawn(kind: IdKind, count: u32) -> Vec<String> {
        generate(&request(kind, count)).unwrap()
    }

    #[test]
    fn uuids_come_in_the_version_and_case_asked_for() {
        let four = generate(&IdentifiersRequest {
            uppercase: true,
            ..request(IdKind::UuidV4, 2)
        })
        .unwrap();
        let seven = drawn(IdKind::UuidV7, 2);

        assert_ne!(four[0], four[1]);
        assert_eq!(four[0], four[0].to_uppercase());
        assert_eq!(Uuid::parse_str(&four[0]).unwrap().get_version_num(), 4);
        assert_eq!(Uuid::parse_str(&seven[1]).unwrap().get_version_num(), 7);
        assert!(seven[0] <= seven[1], "a v7 sorts by creation");
    }

    #[test]
    fn a_uuid_without_hyphens_is_its_32_digits_in_either_case() {
        for (version, uppercase) in [(IdKind::UuidV4, false), (IdKind::UuidV7, true)] {
            let ids = generate(&IdentifiersRequest {
                uppercase,
                hyphens: false,
                ..request(version, 3)
            })
            .unwrap();

            assert!(ids.iter().all(|id| id.len() == 32
                && id.bytes().all(|b| b.is_ascii_hexdigit())
                && (id == &id.to_uppercase()) == uppercase));
            assert!(matches!(inspect(&ids[0]), IdInspection::Uuid { .. }));
        }
    }

    #[test]
    fn a_uuid_says_its_version_its_variant_and_when_it_was_made() {
        assert_eq!(
            inspect(" 01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77 "),
            IdInspection::Uuid {
                version: 7,
                variant: UuidVariant::Rfc,
                created: Some("2024-09-25T23:05:01.488Z".to_owned()),
                nil: false,
                random: Some(RandomPart {
                    text: "7cc4-9a5c-6f2d8e1b3a77".into(),
                    bits: 74
                }),
            }
        );
        assert!(matches!(
            inspect("9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90"),
            IdInspection::Uuid {
                version: 4,
                created: None,
                ..
            }
        ));
        assert!(matches!(
            inspect("00000000-0000-0000-0000-000000000000"),
            IdInspection::Uuid {
                nil: true,
                random: None,
                ..
            }
        ));
    }

    #[test]
    fn the_random_part_of_each_kind_is_quoted_with_its_bits() {
        let random = |text: &str| match inspect(text) {
            IdInspection::Uuid { random, .. } => random,
            IdInspection::Ulid { random, .. }
            | IdInspection::ObjectId { random, .. }
            | IdInspection::Ksuid { random, .. } => Some(random),
            other => panic!("{other:?}"),
        };
        let part = |text: &str, bits| Some(drawn_part(text, bits));

        assert_eq!(
            random("9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90"),
            part("9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90", 122)
        );
        assert_eq!(
            random("01922b6e4b307cc49a5c6f2d8e1b3a77"),
            part("7cc49a5c6f2d8e1b3a77", 74),
            "without hyphens"
        );
        assert_eq!(
            random("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            part("TSV4RRFFQ69G5FAV", 80)
        );
        assert_eq!(random("507f1f77bcf86cd799439011"), part("bcf86cd799", 40));
        assert_eq!(
            random("0ujtsYcgvSTl8PAuAdqWYSMnLOv"),
            part("B5A1CD34B5F99D1154FB6853345C9735", 128)
        );
        assert_eq!(random("6ba7b810-9dad-11d1-80b4-00c04fd430c8"), None, "a v1");
    }

    #[test]
    fn ulids_are_crockford_and_count_up_within_a_millisecond() {
        let ulids = drawn(IdKind::Ulid, 500);

        assert!(ulids.iter().all(|ulid| {
            ulid.len() == 26
                && ulid
                    .bytes()
                    .all(|byte| CROCKFORD.contains(&byte.to_ascii_uppercase()))
        }));
        let mut sorted = ulids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, ulids, "monotonic, and none drawn twice");
        let IdInspection::Ulid { created, .. } = inspect(&ulids[0]) else {
            panic!("a ULID");
        };
        assert!(created.starts_with(&Utc::now().format("%Y-").to_string()));
    }

    #[test]
    fn the_ulid_spec_s_example_reads_back_its_time() {
        assert_eq!(
            inspect("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            IdInspection::Ulid {
                created: "2016-07-30T23:54:10.259Z".into(),
                random: drawn_part("TSV4RRFFQ69G5FAV", 80),
            }
        );
        assert_eq!(
            inspect("01arz3ndektsv4rrffq69g5fav"),
            inspect("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            "Crockford reads in any case"
        );
        assert_eq!(
            inspect("8ZZZZZZZZZZZZZZZZZZZZZZZZZ"),
            IdInspection::OutOfRange { id: IdKind::Ulid }
        );
    }

    #[test]
    fn nanoids_take_their_length_and_alphabet() {
        let default = drawn(IdKind::NanoId, 20);
        assert!(default.iter().all(|id| {
            id.len() == 21
                && id
                    .bytes()
                    .all(|byte| NanoAlphabet::UrlSafe.characters().contains(&byte))
        }));
        for (alphabet, allowed) in [
            (NanoAlphabet::Alphanumeric, BASE62.as_slice()),
            (NanoAlphabet::HexLower, b"0123456789abcdef".as_slice()),
            (NanoAlphabet::Digits, b"0123456789".as_slice()),
        ] {
            let ids = generate(&IdentifiersRequest {
                nano_length: 10,
                nano_alphabet: alphabet,
                ..request(IdKind::NanoId, 20)
            })
            .unwrap();
            assert!(
                ids.iter()
                    .all(|id| id.len() == 10 && id.bytes().all(|b| allowed.contains(&b)))
            );
        }
        let clamped = generate(&IdentifiersRequest {
            nano_length: 1,
            ..request(IdKind::NanoId, 1)
        })
        .unwrap();
        assert_eq!(clamped[0].len(), 2);
    }

    #[test]
    fn cuid2s_are_a_letter_then_base_36() {
        let ids = drawn(IdKind::Cuid2, 50);

        assert!(ids.iter().all(|id| id.len() == CUID2_LENGTH
            && id.starts_with(|c: char| c.is_ascii_lowercase())
            && id.bytes().all(|byte| BASE36.contains(&byte))));
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
        assert_eq!(
            inspect(&ids[0]),
            IdInspection::Possible {
                kinds: vec![IdKind::Cuid2, IdKind::NanoId]
            }
        );
    }

    #[test]
    fn object_ids_share_their_second_and_count_up() {
        let ids = generate(&IdentifiersRequest {
            uppercase: true,
            ..request(IdKind::ObjectId, 3)
        })
        .unwrap();

        assert!(
            ids.iter()
                .all(|id| id.len() == 24 && id == &id.to_uppercase())
        );
        assert_eq!(ids[0][..18], ids[1][..18]);
        let counter = |id: &str| u32::from_str_radix(&id[18..], 16).unwrap();
        assert_eq!(
            (counter(&ids[1]) + 0x0100_0000 - counter(&ids[0])) & 0x00FF_FFFF,
            1
        );
    }

    #[test]
    fn an_object_id_reads_back_its_second_and_its_counter() {
        assert_eq!(
            inspect("507f1f77bcf86cd799439011"),
            IdInspection::ObjectId {
                created: "2012-10-17T21:13:27.000Z".into(),
                counter: 0x0043_9011,
                random: drawn_part("bcf86cd799", 40),
            }
        );
    }

    #[test]
    fn a_ksuid_is_27_base_62_characters_and_reads_back() {
        let ids = drawn(IdKind::Ksuid, 5);
        assert!(
            ids.iter()
                .all(|id| id.len() == 27 && id.bytes().all(|b| BASE62.contains(&b)))
        );

        assert_eq!(
            inspect("0ujtsYcgvSTl8PAuAdqWYSMnLOv"),
            IdInspection::Ksuid {
                created: "2017-10-10T04:00:47.000Z".into(),
                random: drawn_part("B5A1CD34B5F99D1154FB6853345C9735", 128),
            }
        );
        assert_eq!(
            inspect("zzzzzzzzzzzzzzzzzzzzzzzzzzz"),
            IdInspection::OutOfRange { id: IdKind::Ksuid }
        );
    }

    #[test]
    fn a_first_cuid_gives_its_time_away() {
        assert_eq!(
            inspect("ch72gsb320000udocl363eofy"),
            IdInspection::CuidV1 {
                created: "2012-09-13T23:03:12.926Z".into()
            }
        );
    }

    #[test]
    fn what_has_no_known_shape_is_said() {
        assert_eq!(
            inspect("V1StGXR8_Z5jdHi6B-myT"),
            IdInspection::Possible {
                kinds: vec![IdKind::NanoId]
            }
        );
        assert_eq!(inspect("not an identifier"), IdInspection::Unrecognised);
        assert_eq!(inspect("abc"), IdInspection::Unrecognised);
    }

    #[test]
    fn a_count_is_kept_within_its_bounds() {
        assert_eq!(drawn(IdKind::UuidV4, 0).len(), 1);
        assert_eq!(
            drawn(IdKind::UuidV4, 10_000).len(),
            MAX_IDENTIFIERS as usize
        );
    }
}
