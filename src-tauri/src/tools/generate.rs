//! Documents shaped by a JSON Schema or by an example, and Lorem ipsum: drawn from a seed, so a
//! generation can be made again.

mod lorem;
mod schema;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use specta::Type;

use crate::count::saturating_u32;

pub use lorem::{LoremAnswer, LoremRequest, LoremUnit, lorem};

pub const MAX_DOCUMENTS: u32 = 100;

/// `SplitMix64`: small, fast and the same everywhere — which is all a seed asks of it. Never for
/// a secret: the passwords are drawn from the system (`random.rs`).
pub(crate) struct Draw(u64);

impl Draw {
    pub(crate) fn new(seed: u32) -> Self {
        Self(u64::from(seed))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `low..=high`; `low` when the range is empty.
    pub(crate) fn between(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        let span = high.abs_diff(low).saturating_add(1);
        let offset = self.next() % span;
        low.saturating_add_unsigned(offset)
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.between(0, i64::try_from(bound).unwrap_or(i64::MAX) - 1)).unwrap_or(0)
    }

    /// In `[0, 1)`, from the top 53 bits: every value a `f64` can hold there is as likely.
    fn unit(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let fraction = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        fraction
    }

    pub(crate) fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }

    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    fn real(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }
}

pub(crate) fn seed_or_draw(seed: Option<u32>) -> u32 {
    seed.unwrap_or_else(|| {
        let mut bytes = [0u8; 4];
        // Without the system's randomness, the seed is still a seed: 0 is as good as any.
        let _ = getrandom::fill(&mut bytes);
        u32::from_le_bytes(bytes)
    })
}

// --- The kinds of string both the schema and the example are made of.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextKind {
    Words,
    Email,
    Uuid,
    DateTime,
    Date,
    Uri,
    Ipv4,
    Hostname,
}

pub(crate) fn text(draw: &mut Draw, kind: TextKind, words: usize) -> String {
    match kind {
        TextKind::Words => lorem::words(draw, words.max(1), false),
        TextKind::Email => format!("{}.{}@example.com", lorem::word(draw), lorem::word(draw)),
        TextKind::Uuid => {
            let bytes: [u8; 16] = std::array::from_fn(|_| draw.next().to_le_bytes()[0]);
            uuid::Builder::from_random_bytes(bytes)
                .into_uuid()
                .hyphenated()
                .to_string()
        }
        TextKind::DateTime => instant(draw).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        TextKind::Date => instant(draw).date_naive().to_string(),
        TextKind::Uri => format!(
            "https://example.com/{}/{}",
            lorem::word(draw),
            lorem::word(draw)
        ),
        // TEST-NET-1: an address documentation may use, reachable nowhere.
        TextKind::Ipv4 => format!("192.0.2.{}", draw.between(1, 254)),
        TextKind::Hostname => format!("{}.example.com", lorem::word(draw)),
    }
}

/// Somewhere between 2020 and 2030.
fn instant(draw: &mut Draw) -> DateTime<Utc> {
    let seconds = draw.between(1_577_836_800, 1_893_456_000);
    Utc.timestamp_opt(seconds, 0).single().unwrap_or_default()
}

fn kind_of(example: &str) -> TextKind {
    if uuid::Uuid::parse_str(example).is_ok() {
        TextKind::Uuid
    } else if DateTime::parse_from_rfc3339(example).is_ok() {
        TextKind::DateTime
    } else if NaiveDate::parse_from_str(example, "%Y-%m-%d").is_ok() {
        TextKind::Date
    } else if example.starts_with("http://") || example.starts_with("https://") {
        TextKind::Uri
    } else if example.contains('@')
        && example
            .rsplit('@')
            .next()
            .is_some_and(|domain| domain.contains('.'))
    {
        TextKind::Email
    } else if example.parse::<std::net::Ipv4Addr>().is_ok() {
        TextKind::Ipv4
    } else {
        TextKind::Words
    }
}

// --- From an example: the same shape, values of the same kind.

fn like(example: &Value, draw: &mut Draw) -> Value {
    match example {
        Value::Null => Value::Null,
        Value::Bool(_) => Value::Bool(draw.chance(0.5)),
        Value::Number(number) => like_number(number, draw),
        Value::String(text) => {
            let kind = kind_of(text);
            let mut drawn = self::text(draw, kind, text.split_whitespace().count());
            if kind == TextKind::Words && text.chars().next().is_some_and(char::is_uppercase) {
                drawn = lorem::capitalised(&drawn);
            }
            Value::String(drawn)
        }
        Value::Array(items) if items.is_empty() => Value::Array(Vec::new()),
        Value::Array(items) => {
            let low = i64::try_from(items.len())
                .unwrap_or(1)
                .saturating_sub(1)
                .max(1);
            let length = draw.between(low, low + 2);
            Value::Array(
                (0..length)
                    .map(|index| {
                        like(
                            &items[usize::try_from(index).unwrap_or(0) % items.len()],
                            draw,
                        )
                    })
                    .collect(),
            )
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), like(value, draw)))
                .collect(),
        ),
    }
}

/// Of the same sign and order of magnitude, with as many decimals.
fn like_number(number: &Number, draw: &mut Draw) -> Value {
    if let Some(integer) = number.as_i64() {
        let magnitude = integer.unsigned_abs().max(10).saturating_mul(2);
        let high = i64::try_from(magnitude).unwrap_or(i64::MAX);
        let drawn = draw.between(0, high);
        return Value::from(if integer < 0 { -drawn } else { drawn });
    }
    let value = number.as_f64().unwrap_or(0.0);
    let decimals = number
        .to_string()
        .split_once('.')
        .map_or(2, |(_, fraction)| fraction.len().min(6));
    let magnitude = value.abs().max(1.0) * 2.0;
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(2));
    let drawn = (draw.real(0.0, magnitude) * scale).round() / scale;
    Number::from_f64(if value < 0.0 { -drawn } else { drawn }).map_or(Value::Null, Value::Number)
}

// --- The command's shape.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonSource {
    Schema,
    Example,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub source: JsonSource,
    pub text: String,
    pub count: u32,
    /// The same seed and the same source draw the same documents; none draws one, and says which.
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Unsupported {
    pub keyword: String,
    /// A `JSONPath` into the schema.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GenerateAnswer {
    Generated {
        text: String,
        seed: u32,
        unsupported: Vec<Unsupported>,
    },
    Unreadable {
        line: u32,
        column: u32,
    },
    /// A schema is an object, or `true`.
    NotASchema,
}

pub fn generate(request: &GenerateRequest) -> GenerateAnswer {
    let source: Value = match serde_json::from_str(&request.text) {
        Ok(value) => value,
        Err(error) => {
            return GenerateAnswer::Unreadable {
                line: saturating_u32(error.line()),
                column: saturating_u32(error.column()),
            };
        }
    };
    let seed = seed_or_draw(request.seed);
    let mut draw = Draw::new(seed);
    let count = request.count.clamp(1, MAX_DOCUMENTS);

    let (documents, unsupported) = match request.source {
        JsonSource::Example => (
            (0..count)
                .map(|_| like(&source, &mut draw))
                .collect::<Vec<_>>(),
            Vec::new(),
        ),
        JsonSource::Schema => {
            if !(source.is_object() || source == Value::Bool(true)) {
                return GenerateAnswer::NotASchema;
            }
            let mut generator = schema::Generator::new(&source);
            let documents = (0..count)
                .map(|_| generator.value(&source, "$", &mut draw, 0))
                .collect();
            (documents, generator.unsupported())
        }
    };

    let value = if count == 1 {
        documents.into_iter().next().unwrap_or(Value::Null)
    } else {
        Value::Array(documents)
    };
    GenerateAnswer::Generated {
        text: serde_json::to_string_pretty(&value).unwrap_or_default(),
        seed,
        unsupported,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map;

    use super::*;

    fn generated(
        source: JsonSource,
        text: &str,
        count: u32,
        seed: u32,
    ) -> (Value, Vec<Unsupported>) {
        match generate(&GenerateRequest {
            source,
            text: text.to_owned(),
            count,
            seed: Some(seed),
        }) {
            GenerateAnswer::Generated {
                text, unsupported, ..
            } => (serde_json::from_str(&text).unwrap(), unsupported),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_seed_draws_the_same_documents_again() {
        let example = r#"{"id":"9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90","n":3}"#;

        assert_eq!(
            generated(JsonSource::Example, example, 3, 7),
            generated(JsonSource::Example, example, 3, 7)
        );
        assert_ne!(
            generated(JsonSource::Example, example, 3, 7),
            generated(JsonSource::Example, example, 3, 8)
        );
    }

    #[test]
    fn no_seed_draws_one_and_says_which() {
        let answer = generate(&GenerateRequest {
            source: JsonSource::Example,
            text: "{\"a\":1}".to_owned(),
            count: 1,
            seed: None,
        });
        let GenerateAnswer::Generated { seed, text, .. } = answer else {
            panic!()
        };

        assert_eq!(
            generated(JsonSource::Example, "{\"a\":1}", 1, seed).0,
            serde_json::from_str::<Value>(&text).unwrap()
        );
    }

    #[test]
    fn an_example_keeps_its_shape_and_the_kinds_of_its_values() {
        let example = r#"{
            "id": "9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90",
            "email": "ana@exemple.fr",
            "created": "2026-09-29T10:00:00Z",
            "day": "2026-09-29",
            "site": "https://exemple.fr",
            "name": "Ana Duval",
            "amount": 49.9,
            "count": 3,
            "paid": true,
            "note": null,
            "lines": [{ "sku": "A", "quantity": 1 }]
        }"#;

        let (value, _) = generated(JsonSource::Example, example, 1, 42);
        let object = value.as_object().unwrap();
        assert_eq!(
            object.keys().collect::<Vec<_>>(),
            serde_json::from_str::<Map<String, Value>>(example)
                .unwrap()
                .keys()
                .collect::<Vec<_>>()
        );
        assert!(uuid::Uuid::parse_str(object["id"].as_str().unwrap()).is_ok());
        assert!(object["email"].as_str().unwrap().contains('@'));
        assert!(DateTime::parse_from_rfc3339(object["created"].as_str().unwrap()).is_ok());
        assert!(NaiveDate::parse_from_str(object["day"].as_str().unwrap(), "%Y-%m-%d").is_ok());
        assert!(object["site"].as_str().unwrap().starts_with("https://"));
        assert!(
            object["name"]
                .as_str()
                .unwrap()
                .chars()
                .next()
                .unwrap()
                .is_uppercase()
        );
        assert!(
            object["amount"].is_f64() && object["count"].is_i64() && object["paid"].is_boolean()
        );
        assert!(object["note"].is_null());
        assert!(
            object["lines"]
                .as_array()
                .unwrap()
                .iter()
                .all(|line| line["quantity"].is_i64())
        );
    }

    #[test]
    fn several_documents_come_as_a_list() {
        let (value, _) = generated(JsonSource::Example, "{\"a\":1}", 4, 1);

        assert_eq!(value.as_array().unwrap().len(), 4);
    }

    #[test]
    fn what_does_not_parse_says_where_and_what_is_no_schema_says_so() {
        assert_eq!(
            generate(&GenerateRequest {
                source: JsonSource::Schema,
                text: "{\n  \"a\": ,\n}".to_owned(),
                count: 1,
                seed: Some(1)
            }),
            GenerateAnswer::Unreadable { line: 2, column: 8 }
        );
        assert_eq!(
            generate(&GenerateRequest {
                source: JsonSource::Schema,
                text: "[1]".to_owned(),
                count: 1,
                seed: Some(1)
            }),
            GenerateAnswer::NotASchema
        );
    }
}
