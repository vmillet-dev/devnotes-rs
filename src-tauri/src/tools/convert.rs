//! JSON to and from TOML, XML and YAML, through one value: what one format cannot hold is said,
//! with where it stands, rather than guessed — or, for a key TOML has no `null` for, left out
//! and named.

mod xml;

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use specta::Type;
use yaml_rust2::yaml::Hash;
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use crate::count::saturating_u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DataFormat {
    Json,
    Toml,
    Xml,
    Yaml,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub text: String,
    pub from: DataFormat,
    pub to: DataFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Crossing {
    /// TOML has no `null`, and one in a list cannot be left out without moving the others.
    TomlNull,
    /// A TOML document is a table: its root cannot be a list or a value.
    TomlRoot,
    /// An XML document has one root element: an object of one key.
    XmlRoot,
    /// A key XML cannot take as an element or an attribute name.
    XmlName,
    /// An `@attribute` holds a value, not an object or a list.
    XmlAttribute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConvertAnswer {
    Converted {
        text: String,
        /// The keys left out, as `JSONPath`s: a `null` TOML cannot write.
        dropped: Vec<String>,
    },
    /// The text given does not parse: one-based line and column, in characters.
    Unreadable { line: u32, column: u32 },
    /// It parses, but the other format cannot hold it: `path` is a `JSONPath`.
    Impossible { crossing: Crossing, path: String },
}

pub(super) struct Blocked {
    crossing: Crossing,
    path: String,
}

/// Where a byte offset falls, in one-based lines and characters.
fn position(text: &str, offset: usize) -> (u32, u32) {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (saturating_u32(line), saturating_u32(column))
}

fn unreadable((line, column): (u32, u32)) -> ConvertAnswer {
    ConvertAnswer::Unreadable { line, column }
}

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

// --- TOML.

fn from_toml(value: toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::String(text),
        toml::Value::Integer(number) => Value::from(number),
        toml::Value::Float(number) => Number::from_f64(number).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(flag) => Value::Bool(flag),
        toml::Value::Datetime(instant) => Value::String(instant.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(from_toml).collect()),
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, from_toml(value)))
                .collect(),
        ),
    }
}

/// A key holding `null` is left out and named in `dropped`; a `null` in a list is refused.
fn to_toml(value: &Value, path: &str, dropped: &mut Vec<String>) -> Result<toml::Value, Blocked> {
    Ok(match value {
        Value::Null => {
            return Err(Blocked {
                crossing: Crossing::TomlNull,
                path: path.to_owned(),
            });
        }
        Value::Bool(flag) => toml::Value::Boolean(*flag),
        Value::Number(number) => number.as_i64().map_or_else(
            || toml::Value::Float(number.as_f64().unwrap_or(0.0)),
            toml::Value::Integer,
        ),
        Value::String(text) => toml::Value::String(text.clone()),
        Value::Array(items) => toml::Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| to_toml(item, &format!("{path}[{index}]"), dropped))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => {
            let mut table = toml::Table::new();
            for (key, item) in map {
                let at = child(path, key);
                if item.is_null() {
                    dropped.push(at);
                } else {
                    table.insert(key.clone(), to_toml(item, &at, dropped)?);
                }
            }
            toml::Value::Table(table)
        }
    })
}

// --- YAML.

fn from_yaml(value: Yaml) -> Value {
    match value {
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map_or(Value::String(text), Value::Number),
        Yaml::Integer(number) => Value::from(number),
        Yaml::String(text) => Value::String(text),
        Yaml::Boolean(flag) => Value::Bool(flag),
        Yaml::Array(items) => Value::Array(items.into_iter().map(from_yaml).collect()),
        Yaml::Hash(hash) => Value::Object(
            hash.into_iter()
                .map(|(key, value)| (yaml_key(key), from_yaml(value)))
                .collect(),
        ),
        Yaml::Null | Yaml::Alias(_) | Yaml::BadValue => Value::Null,
    }
}

/// A YAML key may be a number or a boolean; JSON's are text.
fn yaml_key(key: Yaml) -> String {
    match key {
        Yaml::String(text) | Yaml::Real(text) => text,
        Yaml::Integer(number) => number.to_string(),
        Yaml::Boolean(flag) => flag.to_string(),
        other => format!("{other:?}"),
    }
}

fn to_yaml(value: &Value) -> Yaml {
    match value {
        Value::Null => Yaml::Null,
        Value::Bool(flag) => Yaml::Boolean(*flag),
        Value::Number(number) => number
            .as_i64()
            .map_or_else(|| Yaml::Real(number.to_string()), Yaml::Integer),
        Value::String(text) => Yaml::String(text.clone()),
        Value::Array(items) => Yaml::Array(items.iter().map(to_yaml).collect()),
        Value::Object(map) => {
            let mut hash = Hash::new();
            for (key, item) in map {
                hash.insert(Yaml::String(key.clone()), to_yaml(item));
            }
            Yaml::Hash(hash)
        }
    }
}

// --- Reading into the pivot, writing out of it.

/// Into the one value every format meets in; a refusal is the line and column it stops at.
pub(crate) fn read_value(text: &str, format: DataFormat) -> Result<Value, (u32, u32)> {
    match format {
        DataFormat::Json => serde_json::from_str(text)
            .map_err(|error| (saturating_u32(error.line()), saturating_u32(error.column()))),
        DataFormat::Toml => text
            .parse::<toml::Table>()
            .map(|table| from_toml(toml::Value::Table(table)))
            .map_err(|error| position(text, error.span().map_or(0, |span| span.start))),
        DataFormat::Yaml => YamlLoader::load_from_str(text)
            .map(|documents| documents.into_iter().next().map_or(Value::Null, from_yaml))
            .map_err(|error| {
                let marker = error.marker();
                (
                    saturating_u32(marker.line()),
                    saturating_u32(marker.col() + 1),
                )
            }),
        DataFormat::Xml => xml::read(text).map_err(|offset| position(text, offset)),
    }
}

fn read(text: &str, format: DataFormat) -> Result<Value, ConvertAnswer> {
    read_value(text, format).map_err(unreadable)
}

fn is_toml_header(line: &str) -> bool {
    let line = line.split(" #").next().unwrap_or_default().trim_end();
    line.len() > 2 && line.starts_with('[') && line.ends_with(']') && !line.contains(',')
}

fn is_toml_pair(line: &str) -> bool {
    line.split_once('=').is_some_and(|(key, _)| {
        let key = key.trim();
        !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_alphanumeric() || "_-.\"' ".contains(c))
    })
}

/// A format by the shape of what was written: `<` is XML, `{` JSON, a `[table]` or a
/// `key = value` line TOML, anything else YAML. A `[` is JSON unless it does not parse as JSON
/// and opens a TOML table.
pub(crate) fn detect(text: &str) -> DataFormat {
    let start = text.trim_start();
    if start.starts_with('<') {
        return DataFormat::Xml;
    }
    if start.starts_with('{') {
        return DataFormat::Json;
    }
    if start.starts_with('[') {
        let header = start.lines().next().unwrap_or_default();
        return if is_toml_header(header) && serde_json::from_str::<Value>(text).is_err() {
            DataFormat::Toml
        } else {
            DataFormat::Json
        };
    }
    let first = start
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'));
    match first {
        Some(line) if is_toml_header(line) || is_toml_pair(line) => DataFormat::Toml,
        _ => DataFormat::Yaml,
    }
}

/// The text, and the keys left out on the way.
fn write(value: &Value, format: DataFormat) -> Result<(String, Vec<String>), Blocked> {
    let whole = |text: String| (text, Vec::new());
    match format {
        DataFormat::Json => Ok(whole(
            serde_json::to_string_pretty(value).unwrap_or_default(),
        )),
        DataFormat::Toml => {
            if !value.is_object() {
                return Err(Blocked {
                    crossing: Crossing::TomlRoot,
                    path: "$".to_owned(),
                });
            }
            let mut dropped = Vec::new();
            let table = to_toml(value, "$", &mut dropped)?;
            Ok((toml::to_string_pretty(&table).unwrap_or_default(), dropped))
        }
        DataFormat::Yaml => {
            let mut out = String::new();
            // The emitter writes to a `String`, which cannot fail.
            let _ = YamlEmitter::new(&mut out).dump(&to_yaml(value));
            Ok(whole(
                out.strip_prefix("---\n").unwrap_or(&out).to_owned() + "\n",
            ))
        }
        DataFormat::Xml => xml::write(value).map(whole),
    }
}

pub fn convert(request: &ConvertRequest) -> ConvertAnswer {
    let value = match read(&request.text, request.from) {
        Ok(value) => value,
        Err(answer) => return answer,
    };
    match write(&value, request.to) {
        Ok((text, dropped)) => ConvertAnswer::Converted { text, dropped },
        Err(Blocked { crossing, path }) => ConvertAnswer::Impossible { crossing, path },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn converted(text: &str, from: DataFormat, to: DataFormat) -> String {
        match convert(&ConvertRequest {
            text: text.to_owned(),
            from,
            to,
        }) {
            ConvertAnswer::Converted { text, .. } => text,
            other => panic!("{other:?}"),
        }
    }

    fn answer(text: &str, from: DataFormat, to: DataFormat) -> ConvertAnswer {
        convert(&ConvertRequest {
            text: text.to_owned(),
            from,
            to,
        })
    }

    const SERVICE: &str = r#"{"service":"billing","replicas":2,"ratio":0.5,"tags":["a","b"],"database":{"host":"db","pool":10}}"#;

    #[test]
    fn json_goes_to_toml_and_back_as_the_same_value() {
        let toml = converted(SERVICE, DataFormat::Json, DataFormat::Toml);

        assert!(toml.contains("service = \"billing\""), "{toml}");
        assert!(toml.contains("[database]"), "{toml}");
        let back: Value =
            serde_json::from_str(&converted(&toml, DataFormat::Toml, DataFormat::Json)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(SERVICE).unwrap());
    }

    #[test]
    fn toml_leaves_out_a_key_holding_null_and_names_it() {
        assert_eq!(
            answer(
                r#"{"name":"Dupont","telephone":null}"#,
                DataFormat::Json,
                DataFormat::Toml
            ),
            ConvertAnswer::Converted {
                text: "name = \"Dupont\"\n".to_owned(),
                dropped: vec!["$.telephone".to_owned()]
            }
        );
        let ConvertAnswer::Converted { text, dropped } = answer(
            r#"{"a":{"b key":null,"c":1},"d":null}"#,
            DataFormat::Json,
            DataFormat::Toml,
        ) else {
            panic!()
        };
        assert_eq!(dropped, [r#"$.a["b key"]"#, "$.d"]);
        assert!(text.contains("c = 1"), "{text}");
    }

    #[test]
    fn toml_refuses_a_null_in_a_list_and_says_where() {
        assert_eq!(
            answer(
                r#"{"replicas":[1,null,3]}"#,
                DataFormat::Json,
                DataFormat::Toml
            ),
            ConvertAnswer::Impossible {
                crossing: Crossing::TomlNull,
                path: "$.replicas[1]".to_owned()
            }
        );
        assert_eq!(
            answer("[1,2]", DataFormat::Json, DataFormat::Toml),
            ConvertAnswer::Impossible {
                crossing: Crossing::TomlRoot,
                path: "$".to_owned()
            }
        );
    }

    #[test]
    fn json_goes_to_yaml_and_back_as_the_same_value() {
        let yaml = converted(SERVICE, DataFormat::Json, DataFormat::Yaml);

        assert!(yaml.starts_with("service: billing"), "{yaml}");
        let back: Value =
            serde_json::from_str(&converted(&yaml, DataFormat::Yaml, DataFormat::Json)).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(SERVICE).unwrap());
    }

    #[test]
    fn a_yaml_null_and_numeric_keys_cross_into_json() {
        let json = converted("a: ~\n1: true\n", DataFormat::Yaml, DataFormat::Json);

        assert_eq!(
            serde_json::from_str::<Value>(&json).unwrap(),
            serde_json::json!({ "a": null, "1": true })
        );
    }

    #[test]
    fn a_text_that_does_not_parse_says_where() {
        assert_eq!(
            answer("{\n  \"a\": 1,,\n}", DataFormat::Json, DataFormat::Yaml),
            ConvertAnswer::Unreadable {
                line: 2,
                column: 10
            }
        );
        assert!(matches!(
            answer("a = \n", DataFormat::Toml, DataFormat::Json),
            ConvertAnswer::Unreadable { line: 1, .. }
        ));
        assert!(matches!(
            answer("a: [1, 2\n", DataFormat::Yaml, DataFormat::Json),
            ConvertAnswer::Unreadable { .. }
        ));
    }

    #[test]
    fn a_position_counts_characters_not_bytes() {
        // Byte 8 is the `x`, after two characters of three and two bytes on its line.
        assert_eq!(position("é\né€x", 8), (2, 3));
    }
}
