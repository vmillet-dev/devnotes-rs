//! The part of JSON Schema a document is drawn from. A keyword outside it is listed with where it
//! stands, so a document that ignores it is never passed off as one that honours it.

use std::collections::BTreeSet;

use serde_json::{Map, Number, Value};

use super::{Draw, TextKind, Unsupported, text};

/// Read to draw a document, or changing nothing about what a valid one is.
const UNDERSTOOD: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "$defs",
    "definitions",
    "$ref",
    "title",
    "description",
    "examples",
    "default",
    "deprecated",
    "readOnly",
    "writeOnly",
    "type",
    "enum",
    "const",
    "oneOf",
    "anyOf",
    "properties",
    "required",
    // Nothing is ever added beyond `properties`, which honours `false` as it honours `true`.
    "additionalProperties",
    "items",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minLength",
    "maxLength",
    "format",
];

const FORMATS: &[(&str, TextKind)] = &[
    ("email", TextKind::Email),
    ("uuid", TextKind::Uuid),
    ("date-time", TextKind::DateTime),
    ("date", TextKind::Date),
    ("uri", TextKind::Uri),
    ("url", TextKind::Uri),
    ("ipv4", TextKind::Ipv4),
    ("hostname", TextKind::Hostname),
];

/// Deep enough for any real document; a `$ref` that points back at itself stops here.
const MAX_DEPTH: usize = 16;

fn child(path: &str, key: &str) -> String {
    let mut chars = key.chars();
    let plain = chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_');
    if plain {
        format!("{path}.{key}")
    } else {
        format!("{path}[{}]", serde_json::to_string(key).unwrap_or_default())
    }
}

pub(super) struct Generator<'a> {
    root: &'a Value,
    unsupported: BTreeSet<(String, String)>,
}

impl<'a> Generator<'a> {
    pub(super) fn new(root: &'a Value) -> Self {
        Self {
            root,
            unsupported: BTreeSet::new(),
        }
    }

    pub(super) fn unsupported(self) -> Vec<Unsupported> {
        self.unsupported
            .into_iter()
            .map(|(path, keyword)| Unsupported { keyword, path })
            .collect()
    }

    fn note(&mut self, keyword: impl Into<String>, path: &str) {
        self.unsupported.insert((path.to_owned(), keyword.into()));
    }

    /// `path` is where `schema` stands in the schema, which is what an unsupported keyword names.
    pub(super) fn value(
        &mut self,
        schema: &'a Value,
        path: &str,
        draw: &mut Draw,
        depth: usize,
    ) -> Value {
        let Value::Object(map) = schema else {
            return if *schema == Value::Bool(true) {
                Value::String(text(draw, TextKind::Words, 2))
            } else {
                Value::Null
            };
        };
        if depth > MAX_DEPTH {
            return Value::Null;
        }
        for key in map.keys().filter(|key| !UNDERSTOOD.contains(&key.as_str())) {
            self.note(key.clone(), path);
        }

        if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
            let target = reference
                .strip_prefix('#')
                .and_then(|pointer| self.root.pointer(pointer));
            if let Some(target) = target {
                return self.value(target, reference, draw, depth + 1);
            }
            self.note(format!("$ref: {reference}"), path);
            return Value::Null;
        }
        if let Some(constant) = map.get("const") {
            return constant.clone();
        }
        if let Some(Value::Array(choices)) = map
            .get("enum")
            .filter(|choices| choices.as_array().is_some_and(|c| !c.is_empty()))
        {
            return draw.pick(choices).clone();
        }
        for keyword in ["oneOf", "anyOf"] {
            if let Some(Value::Array(branches)) = map
                .get(keyword)
                .filter(|b| b.as_array().is_some_and(|b| !b.is_empty()))
            {
                let index = draw.below(branches.len());
                return self.value(
                    &branches[index],
                    &format!("{path}.{keyword}[{index}]"),
                    draw,
                    depth + 1,
                );
            }
        }

        match Self::kind(map, draw) {
            "object" => self.object(map, path, draw, depth),
            "array" => self.array(map, path, draw, depth),
            "integer" => integer(map, draw),
            "number" => number(map, draw),
            "boolean" => Value::Bool(draw.chance(0.5)),
            "null" => Value::Null,
            _ => self.string(map, path, draw),
        }
    }

    /// The type asked for, one of several when a list is; otherwise what the keywords imply.
    fn kind(map: &'a Map<String, Value>, draw: &mut Draw) -> &'a str {
        match map.get("type") {
            Some(Value::String(kind)) => kind,
            Some(Value::Array(kinds)) if !kinds.is_empty() => {
                draw.pick(kinds).as_str().unwrap_or("string")
            }
            _ if map.contains_key("properties") => "object",
            _ if map.contains_key("items") => "array",
            _ if map.contains_key("minimum") || map.contains_key("maximum") => "number",
            _ => "string",
        }
    }

    /// Every required property, and each of the others one time in two.
    fn object(
        &mut self,
        map: &'a Map<String, Value>,
        path: &str,
        draw: &mut Draw,
        depth: usize,
    ) -> Value {
        let required: Vec<&str> = map
            .get("required")
            .and_then(Value::as_array)
            .map(|names| names.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mut object = Map::new();
        if let Some(Value::Object(properties)) = map.get("properties") {
            for (name, property) in properties {
                if required.contains(&name.as_str()) || draw.chance(0.5) {
                    let at = child(&format!("{path}.properties"), name);
                    object.insert(name.clone(), self.value(property, &at, draw, depth + 1));
                }
            }
        }
        Value::Object(object)
    }

    fn array(
        &mut self,
        map: &'a Map<String, Value>,
        path: &str,
        draw: &mut Draw,
        depth: usize,
    ) -> Value {
        let low = bound(map, "minItems").unwrap_or(1);
        let high = bound(map, "maxItems").unwrap_or(low + 2).min(low + 10);
        let items = map.get("items");
        let length = draw.between(low, high.max(low));
        Value::Array(
            (0..length)
                .map(|_| match items {
                    Some(items) => self.value(items, &format!("{path}.items"), draw, depth + 1),
                    None => Value::String(text(draw, TextKind::Words, 1)),
                })
                .collect(),
        )
    }

    fn string(&mut self, map: &'a Map<String, Value>, path: &str, draw: &mut Draw) -> Value {
        let kind = match map.get("format").and_then(Value::as_str) {
            Some(format) => FORMATS
                .iter()
                .find(|(name, _)| *name == format)
                .map_or_else(
                    || {
                        self.note(format!("format: {format}"), path);
                        TextKind::Words
                    },
                    |(_, kind)| *kind,
                ),
            None => TextKind::Words,
        };
        let mut drawn = text(draw, kind, 2);
        if kind == TextKind::Words {
            let low = usize::try_from(bound(map, "minLength").unwrap_or(0)).unwrap_or(0);
            while drawn.chars().count() < low {
                drawn.push(' ');
                drawn.push_str(&text(draw, TextKind::Words, 1));
            }
            if let Some(high) = bound(map, "maxLength").and_then(|high| usize::try_from(high).ok())
            {
                if let Some((at, _)) = drawn.char_indices().nth(high) {
                    drawn.truncate(at);
                }
                drawn.truncate(drawn.trim_end().len());
            }
        }
        Value::String(drawn)
    }
}

fn bound(map: &Map<String, Value>, keyword: &str) -> Option<i64> {
    map.get(keyword).and_then(Value::as_f64).map(|value| {
        #[allow(clippy::cast_possible_truncation)]
        let whole = value.ceil().clamp(-1e15, 1e15) as i64;
        whole
    })
}

/// `minimum` and `maximum`, or the exclusive ones one step inside.
fn range(map: &Map<String, Value>, step: f64, default: (f64, f64)) -> (f64, f64) {
    let read = |keyword: &str| map.get(keyword).and_then(Value::as_f64);
    let low = read("minimum")
        .or_else(|| read("exclusiveMinimum").map(|value| value + step))
        .unwrap_or(default.0);
    let high = read("maximum")
        .or_else(|| read("exclusiveMaximum").map(|value| value - step))
        .unwrap_or(default.1.max(low));
    (low, high.max(low))
}

fn integer(map: &Map<String, Value>, draw: &mut Draw) -> Value {
    let (low, high) = range(map, 1.0, (0.0, 1000.0));
    #[allow(clippy::cast_possible_truncation)]
    let (low, high) = (
        low.ceil().clamp(-1e15, 1e15) as i64,
        high.floor().clamp(-1e15, 1e15) as i64,
    );
    Value::from(draw.between(low, high.max(low)))
}

fn number(map: &Map<String, Value>, draw: &mut Draw) -> Value {
    let (low, high) = range(map, 0.01, (0.0, 1000.0));
    let drawn = (draw.real(low, high) * 100.0).round() / 100.0;
    Number::from_f64(drawn.clamp(low, high)).map_or(Value::Null, Value::Number)
}

#[cfg(test)]
mod tests {
    use super::super::{GenerateAnswer, GenerateRequest, JsonSource, generate};
    use super::*;

    const SCHEMA: &str = r##"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["id", "email", "status", "amount", "lines", "customer"],
        "properties": {
            "id": { "type": "string", "format": "uuid" },
            "email": { "type": "string", "format": "email" },
            "created": { "type": "string", "format": "date-time" },
            "status": { "enum": ["draft", "paid", "void"] },
            "amount": { "type": "integer", "minimum": 100, "maximum": 200 },
            "rate": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 },
            "code": { "type": "string", "pattern": "^[A-Z]{3}$", "maxLength": 3 },
            "lines": { "type": "array", "minItems": 2, "maxItems": 2, "items": { "$ref": "#/$defs/line" } },
            "customer": { "type": "object", "required": ["name"], "properties": { "name": { "type": "string", "minLength": 12 } } }
        },
        "$defs": { "line": { "type": "object", "required": ["sku"], "properties": { "sku": { "const": "PRO-M" } } } }
    }"##;

    fn documents(count: u32, seed: u32) -> (Vec<Value>, Vec<Unsupported>) {
        let answer = generate(&GenerateRequest {
            source: JsonSource::Schema,
            text: SCHEMA.to_owned(),
            count,
            seed: Some(seed),
        });
        let GenerateAnswer::Generated {
            text, unsupported, ..
        } = answer
        else {
            panic!("{answer:?}")
        };
        let value: Value = serde_json::from_str(&text).unwrap();
        (
            value.as_array().cloned().unwrap_or_else(|| vec![value]),
            unsupported,
        )
    }

    #[test]
    fn every_document_honours_the_types_bounds_enums_and_formats() {
        for document in documents(20, 3).0 {
            assert!(uuid::Uuid::parse_str(document["id"].as_str().unwrap()).is_ok());
            assert!(document["email"].as_str().unwrap().contains('@'));
            assert!(["draft", "paid", "void"].contains(&document["status"].as_str().unwrap()));
            assert!((100..=200).contains(&document["amount"].as_i64().unwrap()));
            if let Some(rate) = document.get("rate") {
                let rate = rate.as_f64().unwrap();
                assert!(rate > 0.0 && rate <= 1.0, "{rate}");
            }
            if let Some(code) = document.get("code") {
                assert!(code.as_str().unwrap().chars().count() <= 3);
            }
            assert_eq!(
                document["lines"],
                serde_json::json!([{ "sku": "PRO-M" }, { "sku": "PRO-M" }])
            );
            assert!(
                document["customer"]["name"]
                    .as_str()
                    .unwrap()
                    .chars()
                    .count()
                    >= 12
            );
        }
    }

    #[test]
    fn a_keyword_it_does_not_draw_from_is_listed_where_it_stands() {
        assert_eq!(
            documents(1, 1).1,
            [Unsupported {
                keyword: "pattern".to_owned(),
                path: "$.properties.code".to_owned()
            }]
        );
    }

    #[test]
    fn an_unknown_format_and_a_distant_ref_are_listed_too() {
        let schema: Value = serde_json::json!({
            "properties": { "phone": { "type": "string", "format": "phone" }, "x": { "$ref": "other.json#/a" } },
            "required": ["phone", "x"]
        });
        let mut generator = Generator::new(&schema);
        let mut draw = Draw::new(1);
        generator.value(&schema, "$", &mut draw, 0);

        let keywords: Vec<String> = generator
            .unsupported()
            .into_iter()
            .map(|each| each.keyword)
            .collect();
        assert_eq!(keywords, ["format: phone", "$ref: other.json#/a"]);
    }

    #[test]
    fn a_ref_to_itself_stops_rather_than_loops() {
        let schema: Value = serde_json::json!({
            "type": "object", "required": ["next"], "properties": { "next": { "$ref": "#" } }
        });
        let mut generator = Generator::new(&schema);

        assert!(
            generator
                .value(&schema, "$", &mut Draw::new(1), 0)
                .is_object()
        );
    }
}
