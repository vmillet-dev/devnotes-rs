//! XML through the usual convention: `@name` for an attribute, `#text` for the text beside
//! children, a repeated element as a list. XML has no types, so everything it gives is text.

use std::fmt::Write;

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde_json::{Map, Value};

use super::{Blocked, Crossing, child};

const INDENT: &str = "  ";

/// An element being read: its attributes, its children by name in order of first appearance,
/// and its text.
#[derive(Default)]
struct Element {
    name: String,
    fields: Map<String, Value>,
    text: String,
    empty: bool,
}

impl Element {
    fn open(start: &BytesStart<'_>) -> Result<Self, ()> {
        let mut element = Self {
            name: start.name().as_ref().to_owned(),
            ..Self::default()
        };
        for attribute in start.attributes() {
            let attribute = attribute.map_err(|_| ())?;
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| ())?
                .into_owned();
            let key = format!("@{}", attribute.key.as_ref());
            element.fields.insert(key, Value::String(value));
        }
        Ok(element)
    }

    fn add(&mut self, name: String, value: Value) {
        match self.fields.get_mut(&name) {
            Some(Value::Array(items)) => items.push(value),
            Some(existing) => {
                let first = existing.take();
                *existing = Value::Array(vec![first, value]);
            }
            None => {
                self.fields.insert(name, value);
            }
        }
    }

    /// Nothing at all is `null`; text alone is a string; anything more, an object.
    fn close(mut self) -> (String, Value) {
        let text = self.text.trim().to_owned();
        let value = if self.fields.is_empty() {
            if text.is_empty() && self.empty {
                Value::Null
            } else {
                Value::String(text)
            }
        } else {
            if !text.is_empty() {
                self.fields.insert("#text".to_owned(), Value::String(text));
            }
            Value::Object(self.fields)
        };
        (self.name, value)
    }
}

/// Text between tags, which only an element may hold.
fn push_text(open: &mut [Element], text: &str) -> Result<(), ()> {
    match open.last_mut() {
        Some(element) => {
            if !text.trim().is_empty() {
                element.empty = false;
            }
            element.text.push_str(text);
            Ok(())
        }
        None if text.trim().is_empty() => Ok(()),
        None => Err(()),
    }
}

/// A character reference, or one of XML's five entities: without a DTD, no other is defined.
fn resolve(reference: &BytesRef<'_>) -> Option<String> {
    if reference.is_char_ref() {
        return reference
            .resolve_char_ref()
            .ok()
            .flatten()
            .map(String::from);
    }
    resolve_predefined_entity(reference).map(str::to_owned)
}

/// The document's one root, as an object of one key; a byte offset where it stops parsing.
pub(super) fn read(text: &str) -> Result<Value, usize> {
    let mut reader = Reader::from_str(text);
    let mut open: Vec<Element> = Vec::new();
    let mut root: Option<(String, Value)> = None;
    let at = |reader: &Reader<&[u8]>| usize::try_from(reader.buffer_position()).unwrap_or(0);

    loop {
        let event = reader.read_event().map_err(|_| at(&reader))?;
        match event {
            Event::Start(start) => {
                let mut element = Element::open(&start).map_err(|()| at(&reader))?;
                element.empty = true;
                open.push(element);
            }
            Event::Empty(start) => {
                let mut element = Element::open(&start).map_err(|()| at(&reader))?;
                element.empty = true;
                let (name, value) = element.close();
                match open.last_mut() {
                    Some(parent) => {
                        parent.empty = false;
                        parent.add(name, value);
                    }
                    None if root.is_none() => root = Some((name, value)),
                    None => return Err(at(&reader)),
                }
            }
            Event::End(_) => {
                let element = open.pop().ok_or_else(|| at(&reader))?;
                let (name, value) = element.close();
                match open.last_mut() {
                    Some(parent) => {
                        parent.empty = false;
                        parent.add(name, value);
                    }
                    None if root.is_none() => root = Some((name, value)),
                    None => return Err(at(&reader)),
                }
            }
            Event::Text(text) => {
                let text = text.xml10_content();
                push_text(&mut open, &text).map_err(|()| at(&reader))?;
            }
            Event::GeneralRef(reference) => {
                let text = resolve(&reference).ok_or_else(|| at(&reader))?;
                push_text(&mut open, &text).map_err(|()| at(&reader))?;
            }
            Event::CData(data) => {
                if let Some(element) = open.last_mut() {
                    element.empty = false;
                    element.text.push_str(data.as_ref());
                }
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }

    if !open.is_empty() {
        return Err(text.len());
    }
    let (name, value) = root.ok_or(0usize)?;
    Ok(Value::Object(Map::from_iter([(name, value)])))
}

/// A name XML takes: a letter or `_` first, then letters, digits, `-`, `_`, `.` or `:`.
fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

fn escape(text: &str, quote: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Null => Some(String::new()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn element(
    name: &str,
    value: &Value,
    path: &str,
    depth: usize,
    out: &mut String,
) -> Result<(), Blocked> {
    if !is_name(name) {
        return Err(Blocked {
            crossing: Crossing::XmlName,
            path: path.to_owned(),
        });
    }
    let indent = INDENT.repeat(depth);

    if let Value::Array(items) = value {
        for (index, item) in items.iter().enumerate() {
            element(name, item, &format!("{path}[{index}]"), depth, out)?;
        }
        return Ok(());
    }

    // Writing to a `String` cannot fail: the `fmt::Result`s below are dropped for that reason.
    let Value::Object(map) = value else {
        if value.is_null() {
            let _ = writeln!(out, "{indent}<{name}/>");
        } else {
            let text = escape(&scalar(value).unwrap_or_default(), false);
            let _ = writeln!(out, "{indent}<{name}>{text}</{name}>");
        }
        return Ok(());
    };

    let mut attributes = String::new();
    let mut text = None;
    let mut children = Vec::new();
    for (key, item) in map {
        let item_path = child(path, key);
        if let Some(attribute) = key.strip_prefix('@') {
            if !is_name(attribute) {
                return Err(Blocked {
                    crossing: Crossing::XmlName,
                    path: item_path,
                });
            }
            let value = scalar(item).ok_or(Blocked {
                crossing: Crossing::XmlAttribute,
                path: item_path,
            })?;
            let _ = write!(attributes, " {attribute}=\"{}\"", escape(&value, true));
        } else if key == "#text" {
            text = scalar(item);
        } else {
            children.push((key, item, item_path));
        }
    }

    match (children.is_empty(), text) {
        (true, None) => {
            let _ = writeln!(out, "{indent}<{name}{attributes}/>");
        }
        (true, Some(text)) => {
            let _ = writeln!(
                out,
                "{indent}<{name}{attributes}>{}</{name}>",
                escape(&text, false)
            );
        }
        (false, text) => {
            let _ = writeln!(out, "{indent}<{name}{attributes}>");
            if let Some(text) = text {
                let _ = writeln!(out, "{indent}{INDENT}{}", escape(&text, false));
            }
            for (key, item, item_path) in children {
                element(key, item, &item_path, depth + 1, out)?;
            }
            let _ = writeln!(out, "{indent}</{name}>");
        }
    }
    Ok(())
}

pub(super) fn write(value: &Value) -> Result<String, Blocked> {
    let root = match value {
        Value::Object(map) if map.len() == 1 => map.iter().next(),
        _ => None,
    };
    let Some((name, value)) = root.filter(|(_, value)| !value.is_array()) else {
        return Err(Blocked {
            crossing: Crossing::XmlRoot,
            path: "$".to_owned(),
        });
    };
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    element(name, value, &child("$", name), 0, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::{ConvertAnswer, ConvertRequest, DataFormat, convert};
    use super::*;

    fn to_json(xml: &str) -> Value {
        read(xml).unwrap()
    }

    #[test]
    fn attributes_text_and_repeated_elements_follow_the_convention() {
        let json = to_json(
            r#"<?xml version="1.0"?>
            <order id="42" paid="true">
              <line sku="A">Pro · mensuel</line>
              <line sku="B">Stockage</line>
              <note>payée &amp; livrée</note>
              <empty/>
            </order>"#,
        );

        assert_eq!(
            json,
            serde_json::json!({
                "order": {
                    "@id": "42",
                    "@paid": "true",
                    "line": [
                        { "@sku": "A", "#text": "Pro · mensuel" },
                        { "@sku": "B", "#text": "Stockage" }
                    ],
                    "note": "payée & livrée",
                    "empty": null
                }
            })
        );
    }

    #[test]
    fn json_is_written_back_under_the_same_convention() {
        let json = serde_json::json!({
            "order": { "@id": 42, "line": [{ "@sku": "A", "#text": "x < y" }, "plain"], "empty": null }
        });

        assert_eq!(
            write(&json).ok().unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<order id=\"42\">\n  <line sku=\"A\">x &lt; y</line>\n  <line>plain</line>\n  <empty/>\n</order>\n"
        );
    }

    #[test]
    fn a_round_trip_keeps_the_value() {
        let xml = "<a x=\"1\"><b>2</b><b>3</b><c/></a>";

        assert_eq!(to_json(&write(&to_json(xml)).ok().unwrap()), to_json(xml));
    }

    #[test]
    fn what_xml_cannot_hold_is_said() {
        let impossible = |json: &str| match convert(&ConvertRequest {
            text: json.to_owned(),
            from: DataFormat::Json,
            to: DataFormat::Xml,
        }) {
            ConvertAnswer::Impossible { crossing, path } => (crossing, path),
            other => panic!("{other:?}"),
        };

        assert_eq!(
            impossible(r#"{"a":1,"b":2}"#),
            (Crossing::XmlRoot, "$".to_owned())
        );
        assert_eq!(
            impossible(r#"{"a":{"1st":true}}"#),
            (Crossing::XmlName, r#"$.a["1st"]"#.to_owned())
        );
        assert_eq!(
            impossible(r#"{"a":{"@b":[1]}}"#),
            (Crossing::XmlAttribute, r#"$.a["@b"]"#.to_owned())
        );
    }

    #[test]
    fn references_are_resolved_and_an_undeclared_entity_refused() {
        assert_eq!(
            to_json(r#"<a q="&quot;x&quot; &amp; y">caf&#233; &#x41;&lt;&gt;&apos;</a>"#),
            serde_json::json!({ "a": { "@q": "\"x\" & y", "#text": "café A<>'" } })
        );
        assert!(read("<a>&nbsp;</a>").is_err());
    }

    #[test]
    fn a_broken_document_says_where() {
        assert!(read("<a><b></a>").is_err());
        assert!(read("<a></a><b/>").is_err());
        assert!(read("text").is_err());
    }
}
