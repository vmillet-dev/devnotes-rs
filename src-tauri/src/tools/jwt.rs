//! A JSON Web Token taken apart: its header and payload, its dates read against now, and its
//! signature checked for the HMAC algorithms. No JWT crate: `jsonwebtoken` brings `ring`.

use base64::Engine;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use chrono::{DateTime, SecondsFormat, Utc};
use hmac::{KeyInit, Mac, SimpleHmac};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use specta::Type;
use zeroize::Zeroize;

use crate::count::saturating_u32;

/// Base64url, padded or not: a token's segments are written without, a secret often with.
const BASE64URL: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// The claims RFC 7519 and `OpenID Connect` write as seconds since the epoch.
const DATED: [&str; 4] = ["exp", "nbf", "iat", "auth_time"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Segment {
    Header,
    Payload,
    Signature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AlgorithmFamily {
    Hmac,
    Rsa,
    RsaPss,
    Ecdsa,
    EdDsa,
    /// `alg: none`: nothing signs it.
    Unsigned,
    Unknown,
}

impl AlgorithmFamily {
    fn of(algorithm: Option<&str>) -> Self {
        match algorithm {
            Some("HS256" | "HS384" | "HS512") => Self::Hmac,
            Some("RS256" | "RS384" | "RS512") => Self::Rsa,
            Some("PS256" | "PS384" | "PS512") => Self::RsaPss,
            Some("ES256" | "ES256K" | "ES384" | "ES512") => Self::Ecdsa,
            Some("EdDSA" | "Ed25519" | "Ed448") => Self::EdDsa,
            Some(none) if none.eq_ignore_ascii_case("none") => Self::Unsigned,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DatedClaim {
    pub name: String,
    pub iso: String,
    /// For how long ago or how far off, which the page writes and ages.
    pub epoch_milliseconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TokenState {
    Valid,
    Expired,
    NotYetValid,
    /// Neither `exp` nor `nbf`: nothing to be valid against.
    Undated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Verification {
    /// No secret typed.
    NotAsked,
    Valid,
    Invalid,
    /// RS*, PS*, ES*, `EdDSA`: named, not verified here.
    NotVerifiedHere,
    Unsigned,
    /// A secret said to be base64 that is not.
    UnreadableSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JwtProblem {
    /// Not three segments; five is an encrypted token (JWE), not decoded here.
    SegmentCount,
    NotBase64,
    NotJson,
    /// JSON, but not an object.
    NotObject,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JwtRequest {
    pub token: String,
    /// Empty asks for no verification. Zeroed once used.
    pub secret: String,
    pub secret_is_base64: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JwtAnswer {
    Decoded {
        header: String,
        payload: String,
        /// Both as one JSON document, which is what a note keeps: never the token.
        document: String,
        algorithm: Option<String>,
        family: AlgorithmFamily,
        signature: String,
        dates: Vec<DatedClaim>,
        state: TokenState,
        verification: Verification,
    },
    Malformed {
        problem: JwtProblem,
        segment: Option<Segment>,
        segments: u32,
        /// One-based, in characters of the token, where it could be said.
        at: Option<u32>,
    },
}

fn malformed(
    problem: JwtProblem,
    segment: Option<Segment>,
    segments: usize,
    at: Option<usize>,
) -> JwtAnswer {
    JwtAnswer::Malformed {
        problem,
        segment,
        segments: saturating_u32(segments),
        at: at.map(|at| saturating_u32(at) + 1),
    }
}

/// A segment decoded and read as a JSON object, or what fails and where.
fn object(text: &str, offset: usize) -> Result<Map<String, Value>, (JwtProblem, Option<usize>)> {
    let bytes = BASE64URL.decode(text).map_err(|problem| {
        let at = match problem {
            base64::DecodeError::InvalidByte(index, _)
            | base64::DecodeError::InvalidLastSymbol { offset: index, .. } => offset + index,
            _ => offset,
        };
        (JwtProblem::NotBase64, Some(at))
    })?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err((JwtProblem::NotObject, None)),
        Err(_) => Err((JwtProblem::NotJson, None)),
    }
}

fn dated(payload: &Map<String, Value>) -> Vec<DatedClaim> {
    DATED
        .iter()
        .filter_map(|name| {
            let seconds = payload.get(*name)?.as_f64()?;
            #[allow(clippy::cast_possible_truncation)]
            let at = DateTime::from_timestamp_millis((seconds * 1000.0).round() as i64)?;
            Some(DatedClaim {
                name: (*name).to_owned(),
                iso: at.to_rfc3339_opts(SecondsFormat::AutoSi, true),
                #[allow(clippy::cast_precision_loss)]
                epoch_milliseconds: at.timestamp_millis() as f64,
            })
        })
        .collect()
}

fn state(dates: &[DatedClaim], now: DateTime<Utc>) -> TokenState {
    #[allow(clippy::cast_precision_loss)]
    let now = now.timestamp_millis() as f64;
    let at = |name: &str| {
        dates
            .iter()
            .find(|claim| claim.name == name)
            .map(|claim| claim.epoch_milliseconds)
    };
    match (at("exp"), at("nbf")) {
        (Some(expiry), _) if expiry <= now => TokenState::Expired,
        (_, Some(start)) if start > now => TokenState::NotYetValid,
        (None, None) => TokenState::Undated,
        _ => TokenState::Valid,
    }
}

fn verify(algorithm: Option<&str>, signed: &str, signature: &[u8], key: &[u8]) -> Verification {
    macro_rules! check {
        ($digest:ty) => {{
            let mut mac = SimpleHmac::<$digest>::new_from_slice(key).expect("any key length");
            mac.update(signed.as_bytes());
            // `verify_slice` compares in constant time.
            if mac.verify_slice(signature).is_ok() {
                Verification::Valid
            } else {
                Verification::Invalid
            }
        }};
    }
    match algorithm {
        Some("HS256") => check!(sha2::Sha256),
        Some("HS384") => check!(sha2::Sha384),
        Some("HS512") => check!(sha2::Sha512),
        _ => Verification::NotVerifiedHere,
    }
}

/// `now` is handed in, so the tests fix it.
pub fn decode(mut request: JwtRequest, now: DateTime<Utc>) -> JwtAnswer {
    let answer = decode_with(&request, now);
    request.secret.zeroize();
    request.token.zeroize();
    answer
}

fn decode_with(request: &JwtRequest, now: DateTime<Utc>) -> JwtAnswer {
    let token = request.token.trim();
    let token = token.strip_prefix("Bearer ").unwrap_or(token).trim();
    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() != 3 {
        return malformed(JwtProblem::SegmentCount, None, segments.len(), None);
    }
    let lead = request.token.len() - request.token.trim_start().len()
        + if request.token.trim_start().starts_with("Bearer ") {
            7
        } else {
            0
        };
    let header = match object(segments[0], lead) {
        Ok(header) => header,
        Err((problem, at)) => return malformed(problem, Some(Segment::Header), 3, at),
    };
    let payload = match object(segments[1], lead + segments[0].len() + 1) {
        Ok(payload) => payload,
        Err((problem, at)) => return malformed(problem, Some(Segment::Payload), 3, at),
    };
    let signature_offset = lead + segments[0].len() + segments[1].len() + 2;
    let Ok(signature) = BASE64URL.decode(segments[2]) else {
        return malformed(
            JwtProblem::NotBase64,
            Some(Segment::Signature),
            3,
            Some(signature_offset),
        );
    };

    let algorithm = header.get("alg").and_then(Value::as_str).map(str::to_owned);
    let family = AlgorithmFamily::of(algorithm.as_deref());
    let verification = match family {
        AlgorithmFamily::Unsigned => Verification::Unsigned,
        AlgorithmFamily::Hmac if request.secret.is_empty() => Verification::NotAsked,
        AlgorithmFamily::Hmac => {
            let signed = &token[..=(segments[0].len() + segments[1].len())];
            let key = if request.secret_is_base64 {
                BASE64URL
                    .decode(request.secret.trim())
                    .or_else(|_| {
                        base64::engine::general_purpose::STANDARD.decode(request.secret.trim())
                    })
                    .ok()
            } else {
                Some(request.secret.as_bytes().to_vec())
            };
            match key {
                Some(mut key) => {
                    let verified = verify(algorithm.as_deref(), signed, &signature, &key);
                    key.zeroize();
                    verified
                }
                None => Verification::UnreadableSecret,
            }
        }
        _ => Verification::NotVerifiedHere,
    };

    let dates = dated(&payload);
    let pretty = |value: &Value| serde_json::to_string_pretty(value).unwrap_or_default();
    let (header, payload) = (Value::Object(header), Value::Object(payload));
    JwtAnswer::Decoded {
        document: pretty(&serde_json::json!({ "header": header, "payload": payload })),
        header: pretty(&header),
        payload: pretty(&payload),
        algorithm,
        family,
        signature: segments[2].to_owned(),
        state: state(&dates, now),
        dates,
        verification,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JWT_IO: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    /// RFC 7519 §3.1, signed with the key of RFC 7515 appendix A.1.
    const RFC_7519: &str = "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const RFC_7515_KEY: &str =
        "AyM1SysPpbyDfgZld3umj1qzKObwVMkoqQ-EstJQLr_T-1qS0gZH75aKtMN3Yj0iPS4hcgUuTwjAzZr1Z9CAow";

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-30T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn ask(token: &str, secret: &str, secret_is_base64: bool) -> JwtAnswer {
        decode(
            JwtRequest {
                token: token.to_owned(),
                secret: secret.to_owned(),
                secret_is_base64,
            },
            now(),
        )
    }

    fn verification(answer: &JwtAnswer) -> Verification {
        match answer {
            JwtAnswer::Decoded { verification, .. } => *verification,
            malformed @ JwtAnswer::Malformed { .. } => panic!("not decoded: {malformed:?}"),
        }
    }

    /// A token signed here, with the HMAC under test.
    fn signed(algorithm: &str, payload: &Value, secret: &[u8]) -> String {
        let header = BASE64URL.encode(format!(r#"{{"alg":"{algorithm}","typ":"JWT"}}"#));
        let body = BASE64URL.encode(payload.to_string());
        let signed = format!("{header}.{body}");
        let signature = match algorithm {
            "HS384" => {
                let mut mac = SimpleHmac::<sha2::Sha384>::new_from_slice(secret).unwrap();
                mac.update(signed.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            "HS512" => {
                let mut mac = SimpleHmac::<sha2::Sha512>::new_from_slice(secret).unwrap();
                mac.update(signed.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            _ => {
                let mut mac = SimpleHmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                mac.update(signed.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
        };
        format!(
            "{signed}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature)
        )
    }

    #[test]
    fn jwt_io_s_example_is_decoded_and_verified() {
        let answer = ask(JWT_IO, "your-256-bit-secret", false);

        let JwtAnswer::Decoded {
            header,
            payload,
            algorithm,
            family,
            dates,
            state,
            verification,
            ..
        } = answer
        else {
            panic!("decoded");
        };
        assert_eq!(header, "{\n  \"alg\": \"HS256\",\n  \"typ\": \"JWT\"\n}");
        assert!(payload.contains("\"name\": \"John Doe\""));
        assert_eq!(
            (algorithm.as_deref(), family),
            (Some("HS256"), AlgorithmFamily::Hmac)
        );
        assert_eq!(dates[0].name, "iat");
        assert_eq!(dates[0].iso, "2018-01-18T01:30:22Z");
        assert_eq!(state, TokenState::Undated);
        assert_eq!(verification, Verification::Valid);
        assert_eq!(
            verification_of(JWT_IO, "wrong secret"),
            Verification::Invalid
        );
        assert_eq!(verification_of(JWT_IO, ""), Verification::NotAsked);
    }

    fn verification_of(token: &str, secret: &str) -> Verification {
        verification(&ask(token, secret, false))
    }

    #[test]
    fn rfc_7519_s_example_with_a_base64_secret() {
        let answer = ask(RFC_7519, RFC_7515_KEY, true);

        assert_eq!(verification(&answer), Verification::Valid);
        let JwtAnswer::Decoded { state, dates, .. } = answer else {
            panic!("decoded");
        };
        assert_eq!(state, TokenState::Expired, "exp is 2011-03-22");
        assert_eq!(dates[0].iso, "2011-03-22T18:43:00Z");
        assert_eq!(
            verification(&ask(RFC_7519, "not base64!", true)),
            Verification::UnreadableSecret
        );
    }

    #[test]
    fn each_hmac_algorithm_with_a_right_and_a_wrong_secret() {
        for algorithm in ["HS256", "HS384", "HS512"] {
            let token = signed(algorithm, &serde_json::json!({"sub": "42"}), b"s3cret");
            assert_eq!(
                verification_of(&token, "s3cret"),
                Verification::Valid,
                "{algorithm}"
            );
            assert_eq!(
                verification_of(&token, "s3cre7"),
                Verification::Invalid,
                "{algorithm}"
            );
        }
    }

    #[test]
    fn an_expired_and_a_not_yet_valid_token() {
        let expired = signed("HS256", &serde_json::json!({"exp": 1_790_000_000}), b"k");
        let later = signed(
            "HS256",
            &serde_json::json!({"nbf": 1_900_000_000, "exp": 1_950_000_000}),
            b"k",
        );
        let valid = signed("HS256", &serde_json::json!({"exp": 1_900_000_000.5}), b"k");
        let state = |token: &str| match ask(token, "", false) {
            JwtAnswer::Decoded { state, .. } => state,
            other @ JwtAnswer::Malformed { .. } => panic!("{other:?}"),
        };

        assert_eq!(state(&expired), TokenState::Expired);
        assert_eq!(state(&later), TokenState::NotYetValid);
        assert_eq!(state(&valid), TokenState::Valid);
    }

    #[test]
    fn alg_none_is_flagged_and_other_families_named() {
        let none = format!(
            "{}.{}.",
            BASE64URL.encode(r#"{"alg":"none"}"#),
            BASE64URL.encode(r#"{"sub":"42"}"#)
        );
        assert_eq!(verification_of(&none, "anything"), Verification::Unsigned);

        for (algorithm, family) in [
            ("RS256", AlgorithmFamily::Rsa),
            ("PS384", AlgorithmFamily::RsaPss),
            ("ES256", AlgorithmFamily::Ecdsa),
            ("EdDSA", AlgorithmFamily::EdDsa),
            ("XX1", AlgorithmFamily::Unknown),
        ] {
            let token = format!(
                "{}.{}.c2ln",
                BASE64URL.encode(format!(r#"{{"alg":"{algorithm}"}}"#)),
                BASE64URL.encode("{}")
            );
            let answer = ask(&token, "secret", false);
            let JwtAnswer::Decoded {
                family: found,
                verification,
                ..
            } = answer
            else {
                panic!("decoded");
            };
            assert_eq!(
                (found, verification),
                (family, Verification::NotVerifiedHere)
            );
        }
    }

    #[test]
    fn each_malformed_shape_says_what_and_where() {
        assert_eq!(
            ask("abc.def", "", false),
            JwtAnswer::Malformed {
                problem: JwtProblem::SegmentCount,
                segment: None,
                segments: 2,
                at: None
            }
        );
        assert!(matches!(
            ask("a.b.c.d.e", "", false),
            JwtAnswer::Malformed { segments: 5, .. }
        ));
        assert_eq!(
            ask("eyJ!bGciOiJIUzI1NiJ9.e30.c2ln", "", false),
            JwtAnswer::Malformed {
                problem: JwtProblem::NotBase64,
                segment: Some(Segment::Header),
                segments: 3,
                at: Some(4)
            }
        );
        let not_json = format!(
            "{}.{}.c2ln",
            BASE64URL.encode("{}"),
            BASE64URL.encode("{oops")
        );
        assert!(matches!(
            ask(&not_json, "", false),
            JwtAnswer::Malformed {
                problem: JwtProblem::NotJson,
                segment: Some(Segment::Payload),
                ..
            }
        ));
        let not_object = format!(
            "{}.{}.c2ln",
            BASE64URL.encode("[1]"),
            BASE64URL.encode("{}")
        );
        assert!(matches!(
            ask(&not_object, "", false),
            JwtAnswer::Malformed {
                problem: JwtProblem::NotObject,
                segment: Some(Segment::Header),
                ..
            }
        ));
        let bad_signature = format!("{}.{}.@@", BASE64URL.encode("{}"), BASE64URL.encode("{}"));
        assert!(matches!(
            ask(&bad_signature, "", false),
            JwtAnswer::Malformed {
                problem: JwtProblem::NotBase64,
                segment: Some(Segment::Signature),
                ..
            }
        ));
    }

    #[test]
    fn a_bearer_prefix_and_spaces_are_left_out() {
        assert_eq!(
            verification(&ask(
                &format!("  Bearer {JWT_IO} "),
                "your-256-bit-secret",
                false
            )),
            Verification::Valid
        );
    }
}
