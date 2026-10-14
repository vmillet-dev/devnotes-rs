//! XML read the converter's way — `@name` for an attribute, `#text` for the text beside
//! children, a repeated element as a list, the document as an object of its one root — keeping
//! where each value was written. The converter and the diff read it too, through `to_value`:
//! what the visualiser shows is what they read. Namespaces keep their prefix; comments,
//! processing instructions and the DOCTYPE are not values. XML has no types: everything is text.

use std::borrow::Cow;

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::parse::{JsonError, JsonErrorReason, MAX_DEPTH, Node, Span, Value};
use super::position::Positions;
use super::read::Read;

/// An element being read: its attributes and children in order of first appearance, its text,
/// and where each was written, in bytes.
struct Element {
    name: String,
    start: usize,
    members: Vec<(String, Node)>,
    text: String,
    /// The first and last byte of text that is not blank.
    text_at: Option<(usize, usize)>,
    empty: bool,
}

impl Element {
    fn open(
        tag: &BytesStart<'_>,
        start: usize,
        end: usize,
        positions: &Positions<'_>,
    ) -> Result<Self, ()> {
        let mut element = Self {
            name: tag.name().as_ref().to_owned(),
            start,
            members: Vec::new(),
            text: String::new(),
            text_at: None,
            empty: true,
        };
        let source = positions.text();
        for attribute in tag.attributes() {
            let attribute = attribute.map_err(|_| ())?;
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| ())?
                .into_owned();
            // Borrowed from the text being read: where it sits is where its bytes are.
            let (from, to) = match &attribute.value {
                Cow::Borrowed(raw) => {
                    let from = (raw.as_ptr() as usize).wrapping_sub(source.as_ptr() as usize);
                    if from <= source.len() {
                        (from, from + raw.len())
                    } else {
                        (start, end)
                    }
                }
                Cow::Owned(_) => (start, end),
            };
            let key = format!("@{}", attribute.key.as_ref());
            element.members.push((
                key,
                Node {
                    value: Value::String(value),
                    span: positions.span(from, to),
                },
            ));
        }
        Ok(element)
    }

    /// `raw` is the text as written, which the blanks around it are measured on: the content
    /// has its line ends normalised and its references resolved.
    fn push_text(&mut self, text: &str, raw: &str, from: usize, to: usize) {
        if !text.trim().is_empty() {
            self.empty = false;
            let lead = raw.len() - raw.trim_start().len();
            let trail = raw.len() - raw.trim_end().len();
            let (first, last) = (from + lead, to.saturating_sub(trail).max(from + lead));
            self.text_at = Some(match self.text_at {
                Some((start, _)) => (start, last),
                None => (first, last),
            });
        }
        self.text.push_str(text);
    }

    /// A repeated child becomes a list, at the place of its first appearance.
    fn add(&mut self, name: String, node: Node) {
        self.empty = false;
        match self.members.iter_mut().find(|(key, _)| *key == name) {
            Some((_, existing)) => {
                let span = Span {
                    start: existing.span.start,
                    end: node.span.end,
                };
                if let Value::Array(items) = &mut existing.value {
                    items.push(node);
                } else {
                    let first = std::mem::replace(&mut existing.value, Value::Null);
                    existing.value = Value::Array(vec![
                        Node {
                            value: first,
                            span: existing.span,
                        },
                        node,
                    ]);
                }
                existing.span = span;
            }
            None => self.members.push((name, node)),
        }
    }

    /// Nothing at all is null; text alone is a string, placed on that text; anything more is an
    /// object, the text under `#text`.
    fn close(mut self, end: usize, positions: &Positions<'_>) -> (String, Node) {
        let text = self.text.trim().to_owned();
        let whole = positions.span(self.start, end);
        let written = self
            .text_at
            .map_or(whole, |(from, to)| positions.span(from, to));
        let node = if self.members.is_empty() {
            if text.is_empty() && self.empty {
                Node {
                    value: Value::Null,
                    span: whole,
                }
            } else {
                Node {
                    value: Value::String(text),
                    span: written,
                }
            }
        } else {
            if !text.is_empty() {
                self.members.push((
                    "#text".to_owned(),
                    Node {
                        value: Value::String(text),
                        span: written,
                    },
                ));
            }
            Node {
                value: Value::Object(self.members),
                span: whole,
            }
        };
        (self.name, node)
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

/// A refusal: where the text stops being read, in bytes, and why.
pub(crate) struct Refusal {
    pub(crate) at: usize,
    pub(crate) reason: JsonErrorReason,
}

impl Refusal {
    fn malformed(at: usize) -> Self {
        Self {
            at,
            reason: JsonErrorReason::Malformed,
        }
    }
}

/// The document as an object of its one root, every value placed where it was written.
pub(crate) fn document(text: &str) -> Result<Node, Refusal> {
    let positions = Positions::new(text);
    let mut reader = Reader::from_str(text);
    let mut open: Vec<Element> = Vec::new();
    let mut root: Option<(String, Node)> = None;
    let at = |reader: &Reader<&[u8]>| usize::try_from(reader.buffer_position()).unwrap_or(0);

    loop {
        let before = at(&reader);
        let event = reader
            .read_event()
            .map_err(|_| Refusal::malformed(at(&reader)))?;
        let after = at(&reader);
        match event {
            Event::Start(tag) => {
                if open.len() >= MAX_DEPTH {
                    return Err(Refusal {
                        at: before,
                        reason: JsonErrorReason::TooDeep,
                    });
                }
                let element = Element::open(&tag, before, after, &positions)
                    .map_err(|()| Refusal::malformed(before))?;
                open.push(element);
            }
            Event::Empty(tag) => {
                let element = Element::open(&tag, before, after, &positions)
                    .map_err(|()| Refusal::malformed(before))?;
                let (name, node) = element.close(after, &positions);
                attach(&mut open, &mut root, name, node)
                    .map_err(|()| Refusal::malformed(before))?;
            }
            Event::End(_) => {
                let element = open.pop().ok_or_else(|| Refusal::malformed(before))?;
                let (name, node) = element.close(after, &positions);
                attach(&mut open, &mut root, name, node)
                    .map_err(|()| Refusal::malformed(before))?;
            }
            Event::Text(raw) => {
                let content = raw.xml10_content();
                match open.last_mut() {
                    Some(element) => {
                        element.push_text(&content, &text[before..after], before, after);
                    }
                    None if content.trim().is_empty() => {}
                    None => return Err(Refusal::malformed(before)),
                }
            }
            Event::GeneralRef(reference) => {
                let resolved = resolve(&reference).ok_or_else(|| Refusal::malformed(before))?;
                match open.last_mut() {
                    Some(element) => {
                        element.push_text(&resolved, &text[before..after], before, after);
                    }
                    None => return Err(Refusal::malformed(before)),
                }
            }
            Event::CData(data) => {
                if let Some(element) = open.last_mut() {
                    let content = data.as_ref().to_owned();
                    element.empty = false;
                    element.text.push_str(&content);
                    // The text itself, inside `<![CDATA[` and `]]>`.
                    let (from, to) = (before + 9, after.saturating_sub(3));
                    element.text_at = Some(match element.text_at {
                        Some((start, _)) => (start, to),
                        None => (from, to),
                    });
                }
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }

    if !open.is_empty() {
        return Err(Refusal::malformed(text.len()));
    }
    let (name, node) = root.ok_or_else(|| Refusal::malformed(0))?;
    Ok(Node {
        value: Value::Object(vec![(name, node)]),
        span: positions.span(0, text.len()),
    })
}

fn attach(
    open: &mut [Element],
    root: &mut Option<(String, Node)>,
    name: String,
    node: Node,
) -> Result<(), ()> {
    match open.last_mut() {
        Some(parent) => {
            parent.add(name, node);
            Ok(())
        }
        None if root.is_none() => {
            *root = Some((name, node));
            Ok(())
        }
        None => Err(()),
    }
}

pub(crate) fn read(text: &str) -> Result<Read, JsonError> {
    document(text)
        .map(|root| Read {
            root,
            notices: Vec::new(),
        })
        .map_err(|refusal| Positions::new(text).error(refusal.reason, refusal.at))
}

/// The value the converter and the diff meet in.
pub(crate) fn to_value(node: &Node) -> serde_json::Value {
    match &node.value {
        Value::Object(members) => serde_json::Value::Object(
            members
                .iter()
                .map(|(key, child)| (key.clone(), to_value(child)))
                .collect(),
        ),
        Value::Array(items) => serde_json::Value::Array(items.iter().map(to_value).collect()),
        Value::String(text) | Value::Number(text) | Value::Datetime(text) => {
            serde_json::Value::String(text.clone())
        }
        Value::Bool(flag) => serde_json::Value::Bool(*flag),
        Value::Null => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: &str = "<?xml version=\"1.0\"?>\n<!DOCTYPE order>\n<!-- a comment -->\n<shop:order xmlns:shop=\"urn:shop\" id=\"42\">\n  <line sku=\"A\">Pro · mensuel</line>\n  <line sku=\"B\">Stockage</line>\n  <note>payée &amp; <b>vite</b> livrée</note>\n  <raw><![CDATA[a < b]]></raw>\n  <empty/>\n</shop:order>\n";

    fn members(node: &Node) -> &[(String, Node)] {
        let Value::Object(members) = &node.value else {
            panic!("an object, not {:?}", node.value);
        };
        members
    }

    fn slice(text: &str, node: &Node) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[node.span.start as usize..node.span.end as usize]).unwrap()
    }

    fn order() -> Node {
        let root = document(ORDER).ok().expect("a readable document");
        members(&root)[0].1.clone()
    }

    #[test]
    fn follows_the_converters_convention_prefixes_included() {
        let root = document(ORDER).ok().unwrap();
        assert_eq!(members(&root)[0].0, "shop:order");

        let order = order();
        assert_eq!(
            members(&order)
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["@xmlns:shop", "@id", "line", "note", "raw", "empty"]
        );
        let Value::Array(lines) = &members(&order)[2].1.value else {
            panic!("repeated siblings as a list");
        };
        assert_eq!(members(&lines[1])[1].0, "#text");
        assert_eq!(
            members(&lines[1])[1].1.value,
            Value::String("Stockage".into())
        );
        assert_eq!(members(&order)[5].1.value, Value::Null);
    }

    #[test]
    fn keeps_mixed_content_and_cdata_as_text() {
        let order = order();
        let note = members(&members(&order)[3].1);

        assert_eq!(note[0].0, "b");
        assert_eq!(note[1].0, "#text");
        assert_eq!(note[1].1.value, Value::String("payée &  livrée".into()));
        assert_eq!(members(&order)[4].1.value, Value::String("a < b".into()));
    }

    #[test]
    fn places_each_value_where_it_was_written() {
        let order = order();
        let top = members(&order);

        assert_eq!(slice(ORDER, &top[1].1), "42");
        let Value::Array(lines) = &top[2].1.value else {
            panic!("a list");
        };
        assert_eq!(
            slice(ORDER, &lines[0]),
            "<line sku=\"A\">Pro · mensuel</line>"
        );
        assert_eq!(slice(ORDER, &members(&lines[0])[1].1), "Pro · mensuel");
        assert_eq!(
            slice(ORDER, &top[2].1),
            "<line sku=\"A\">Pro · mensuel</line>\n  <line sku=\"B\">Stockage</line>"
        );
        assert_eq!(slice(ORDER, &top[4].1), "a < b");
        assert_eq!(slice(ORDER, &top[5].1), "<empty/>");
    }

    #[test]
    fn counts_offsets_in_utf16_over_wide_characters() {
        let text = "<r><c>🦀</c><d k=\"é\">x</d></r>";
        let root = document(text).ok().unwrap();
        let r = members(&members(&root)[0].1);

        assert_eq!((r[0].1.span.start, r[0].1.span.end), (6, 8));
        assert_eq!(slice(text, &members(&r[1].1)[0].1), "é");
    }

    #[test]
    fn says_where_an_unreadable_document_breaks() {
        let error = read("<a>\n  <b></a>").expect_err("a mismatched end tag");
        assert_eq!(error.reason, JsonErrorReason::Malformed);
        assert_eq!(error.line, 2);

        assert!(read("<a></a><b/>").is_err());
        assert!(read("text").is_err());
        assert!(read("<a>&nbsp;</a>").is_err());
        assert_eq!(
            read("<a>").expect_err("unclosed").reason,
            JsonErrorReason::Malformed
        );
    }

    #[test]
    fn refuses_a_nesting_deeper_than_it_can_walk() {
        let deep = format!("{}{}", "<a>".repeat(300), "</a>".repeat(300));

        assert_eq!(
            read(&deep).expect_err("too deep").reason,
            JsonErrorReason::TooDeep
        );
    }

    #[test]
    fn hands_the_converter_the_same_value() {
        let value = to_value(
            &document("<a x=\"1\"><b>2</b><b>3</b><c/></a>")
                .ok()
                .unwrap(),
        );

        assert_eq!(
            value,
            serde_json::json!({ "a": { "@x": "1", "b": ["2", "3"], "c": null } })
        );
    }
}
