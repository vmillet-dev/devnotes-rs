//! A YAML reader for the visualiser: the first document of a stream, every value where it was
//! written. An alias is read as what it names, placed where its anchor was written; a merge key
//! `<<` stays a key, as read. Comments are not values, and do not appear.

use std::collections::HashMap;

use yaml_rust2::parser::{Event, Parser, Tag};
use yaml_rust2::scanner::{Marker, TScalarStyle};
use yaml_rust2::{ScanError, Yaml};

use super::parse::{JsonError, JsonErrorReason, MAX_DEPTH, Node, Value};
use super::position::Positions;
use super::read::{JsonNotice, Read};

/// Values read in all, aliases' copies included: past it, a document of aliases to aliases
/// stops instead of multiplying.
const MAX_VALUES: usize = 200_000;

enum Frame {
    Mapping {
        start: usize,
        anchor: usize,
        members: Vec<(String, Node)>,
        key: Option<String>,
        end: usize,
    },
    Sequence {
        start: usize,
        anchor: usize,
        items: Vec<Node>,
        end: usize,
    },
}

struct Reader<'a> {
    positions: Positions<'a>,
    stack: Vec<Frame>,
    anchors: HashMap<usize, Node>,
    root: Option<Node>,
    values: usize,
}

pub(crate) fn read(text: &str) -> Result<Read, JsonError> {
    let mut reader = Reader {
        positions: Positions::new(text),
        stack: Vec::new(),
        anchors: HashMap::new(),
        root: None,
        values: 0,
    };
    let mut parser = Parser::new_from_str(text);
    let mut documents = 0;
    let mut notices = Vec::new();

    loop {
        let (event, marker) = parser
            .next_token()
            .map_err(|error| reader.refused(&error))?;
        let at = reader.byte(&marker);
        match event {
            Event::StreamEnd => break,
            Event::DocumentStart => {
                documents += 1;
                if documents > 1 {
                    notices.push(JsonNotice::FirstDocumentOnly);
                    break;
                }
            }
            Event::MappingStart(anchor, _) => reader.open(Frame::Mapping {
                start: at,
                anchor,
                members: Vec::new(),
                key: None,
                end: at,
            })?,
            Event::SequenceStart(anchor, _) => reader.open(Frame::Sequence {
                start: at,
                anchor,
                items: Vec::new(),
                end: at,
            })?,
            Event::MappingEnd | Event::SequenceEnd => reader.close(at)?,
            Event::Scalar(raw, style, anchor, tag) => {
                reader.scalar(raw, style, anchor, tag.as_ref(), at)?;
            }
            Event::Alias(id) => reader.alias(id, at)?,
            Event::StreamStart | Event::DocumentEnd | Event::Nothing => {}
        }
    }

    let root = reader.root.unwrap_or(Node {
        value: Value::Null,
        span: reader.positions.span(0, 0),
    });
    Ok(Read { root, notices })
}

impl Reader<'_> {
    fn byte(&self, marker: &Marker) -> usize {
        self.positions.byte_at(marker.line(), marker.col())
    }

    /// The parser's own nesting limit — 255 flow levels — is the same refusal as ours.
    fn refused(&self, error: &ScanError) -> JsonError {
        let reason = if error.info().contains("recursion limit") {
            JsonErrorReason::TooDeep
        } else {
            JsonErrorReason::Malformed
        };
        self.positions.error(reason, self.byte(error.marker()))
    }

    fn open(&mut self, frame: Frame) -> Result<(), JsonError> {
        if self.stack.len() >= MAX_DEPTH {
            let at = match &frame {
                Frame::Mapping { start, .. } | Frame::Sequence { start, .. } => *start,
            };
            return Err(self.positions.error(JsonErrorReason::TooDeep, at));
        }
        self.stack.push(frame);
        Ok(())
    }

    /// A block collection ends where its last value does; a flow one on its bracket.
    fn close(&mut self, at: usize) -> Result<(), JsonError> {
        let Some(frame) = self.stack.pop() else {
            return Ok(());
        };
        let text = self.positions.text();
        let bracket = matches!(text.as_bytes().get(at), Some(b'}' | b']'));
        let (value, start, anchor, last) = match frame {
            Frame::Mapping {
                start,
                anchor,
                members,
                end,
                ..
            } => (Value::Object(members), start, anchor, end),
            Frame::Sequence {
                start,
                anchor,
                items,
                end,
            } => (Value::Array(items), start, anchor, end),
        };
        let end = if bracket { at + 1 } else { last };
        let node = Node {
            value,
            span: self.positions.span(start, end),
        };
        self.place(node, anchor, None, start, end)
    }

    fn scalar(
        &mut self,
        raw: String,
        style: TScalarStyle,
        anchor: usize,
        tag: Option<&Tag>,
        at: usize,
    ) -> Result<(), JsonError> {
        let text = self.positions.text();
        // A `|` or `>` block is marked on its first line of content: it starts on its indicator.
        let at = match style {
            TScalarStyle::Literal | TScalarStyle::Folded => {
                text[..at].rfind(['|', '>']).unwrap_or(at)
            }
            _ => at,
        };
        let end = scalar_end(text, at, &raw, style);
        let value = typed(&raw, style, tag);
        let node = Node {
            value,
            span: self.positions.span(at, end),
        };
        self.place(node, anchor, Some(raw), at, end)
    }

    fn alias(&mut self, id: usize, at: usize) -> Result<(), JsonError> {
        let text = self.positions.text();
        let end = at
            + text[at..]
                .find(|character: char| character.is_whitespace() || ",]}".contains(character))
                .unwrap_or(text.len() - at);
        let Some(node) = self.anchors.get(&id).cloned() else {
            return Err(self.positions.error(JsonErrorReason::Malformed, at));
        };
        self.values += weight(&node);
        if self.values > MAX_VALUES {
            return Err(self.positions.error(JsonErrorReason::TooLarge, at));
        }
        self.place(node, 0, None, at, end)
    }

    /// Into the collection being read — as a key, then as its value — or as the document.
    fn place(
        &mut self,
        node: Node,
        anchor: usize,
        raw: Option<String>,
        start: usize,
        end: usize,
    ) -> Result<(), JsonError> {
        self.values += 1;
        if self.values > MAX_VALUES {
            return Err(self
                .positions
                .error(JsonErrorReason::TooLarge, end.saturating_sub(1)));
        }
        if anchor > 0 {
            self.anchors.insert(anchor, node.clone());
        }
        let text = self.positions.text();
        match self.stack.last_mut() {
            None => self.root = Some(node),
            Some(Frame::Mapping {
                members,
                key,
                start: first,
                end: last,
                ..
            }) => {
                *first = (*first).min(start);
                *last = (*last).max(end);
                match key.take() {
                    None => *key = Some(raw.unwrap_or_else(|| one_line(&text[start..end]))),
                    Some(name) => members.push((name, node)),
                }
            }
            Some(Frame::Sequence {
                items,
                start: first,
                end: last,
                ..
            }) => {
                *first = (*first).min(start);
                *last = (*last).max(end);
                items.push(node);
            }
        }
        Ok(())
    }
}

/// The core schema: a plain scalar is a number, a boolean or null when it reads as one; a
/// quoted one, or one tagged `!!str`, stays a string.
fn typed(raw: &str, style: TScalarStyle, tag: Option<&Tag>) -> Value {
    if style != TScalarStyle::Plain || tag.is_some_and(|tag| tag.suffix == "str") {
        return Value::String(raw.to_string());
    }
    match Yaml::from_str(raw) {
        Yaml::Integer(_) | Yaml::Real(_) => Value::Number(raw.to_string()),
        Yaml::Boolean(value) => Value::Bool(value),
        Yaml::Null => Value::Null,
        _ => Value::String(raw.to_string()),
    }
}

/// A key that is a collection, or an alias, is named by its text on one line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn weight(node: &Node) -> usize {
    1 + match &node.value {
        Value::Object(members) => members.iter().map(|(_, child)| weight(child)).sum(),
        Value::Array(items) => items.iter().map(weight).sum(),
        _ => 0,
    }
}

/// The parser marks where a scalar starts; where it ends is read from the text, by its style.
fn scalar_end(text: &str, start: usize, raw: &str, style: TScalarStyle) -> usize {
    let rest = &text[start..];
    match style {
        TScalarStyle::SingleQuoted => start + single_quoted(rest),
        TScalarStyle::DoubleQuoted => start + double_quoted(rest),
        TScalarStyle::Literal | TScalarStyle::Folded => block_end(text, start),
        TScalarStyle::Plain if rest.starts_with(raw) => start + raw.len(),
        TScalarStyle::Plain => start + plain_line(rest),
    }
}

/// Past the closing `'`, a doubled one being part of the text.
fn single_quoted(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut at = 1;
    while at < bytes.len() {
        if bytes[at] == b'\'' {
            if bytes.get(at + 1) == Some(&b'\'') {
                at += 2;
                continue;
            }
            return at + 1;
        }
        at += 1;
    }
    rest.len()
}

/// Past the closing `"`, a backslash escaping the byte after it.
fn double_quoted(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut at = 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return at + 1,
            _ => at += 1,
        }
    }
    rest.len()
}

/// A plain scalar over several lines: its first line, up to a comment.
fn plain_line(rest: &str) -> usize {
    let line = rest.split('\n').next().unwrap_or("");
    let line = line.find(" #").map_or(line, |at| &line[..at]);
    line.trim_end().len()
}

/// A `|` or `>` block: every line after its indicator indented further than the indicator's.
fn block_end(text: &str, start: usize) -> usize {
    let line_start = text[..start].rfind('\n').map_or(0, |at| at + 1);
    let indent = indentation(&text[line_start..]);
    let mut end = text[start..].find('\n').map_or(text.len(), |at| start + at);
    let mut at = end;
    while at < text.len() {
        let next = at + 1;
        let line_end = text[next..]
            .find('\n')
            .map_or(text.len(), |offset| next + offset);
        let line = &text[next..line_end];
        if !line.trim().is_empty() {
            if indentation(line) <= indent {
                break;
            }
            end = next + line.trim_end().len();
        }
        at = line_end;
    }
    end
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_ok(text: &str) -> Read {
        read(text).expect("a readable document")
    }

    fn members(node: &Node) -> &[(String, Node)] {
        let Value::Object(members) = &node.value else {
            panic!("a mapping, not {:?}", node.value);
        };
        members
    }

    fn slice(text: &str, node: &Node) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[node.span.start as usize..node.span.end as usize]).unwrap()
    }

    #[test]
    fn reads_nested_maps_and_sequences_in_their_order() {
        let text = "name: api\nports:\n  - 80\n  - 443\nenv:\n  debug: true\n  level: ~\n";
        let root = read_ok(text).root;
        let top = members(&root);

        assert_eq!(
            top.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(),
            ["name", "ports", "env"]
        );
        assert_eq!(top[0].1.value, Value::String("api".into()));
        let Value::Array(ports) = &top[1].1.value else {
            panic!("a sequence");
        };
        assert_eq!(ports[1].value, Value::Number("443".into()));
        let env = members(&top[2].1);
        assert_eq!(env[0].1.value, Value::Bool(true));
        assert_eq!(env[1].1.value, Value::Null);
    }

    #[test]
    fn keeps_every_value_where_it_was_written() {
        let text = "a: 'it''s'\nb: \"x\\\"y\" # comment\nc: plain text # note\nd: [1, {e: 2}]\nf: |\n  line one\n  line two\ng: 3\n";
        let root = read_ok(text).root;
        let top = members(&root);

        assert_eq!(slice(text, &top[0].1), "'it''s'");
        assert_eq!(slice(text, &top[1].1), "\"x\\\"y\"");
        assert_eq!(slice(text, &top[2].1), "plain text");
        assert_eq!(slice(text, &top[3].1), "[1, {e: 2}]");
        assert_eq!(slice(text, &top[4].1), "|\n  line one\n  line two");
        assert_eq!(slice(text, &top[5].1), "3");
    }

    #[test]
    fn ends_a_block_collection_on_its_last_value_not_on_a_comment_after_it() {
        let text = "outer:\n  inner: 1\n  # trailing\nnext: 2\n";
        let root = read_ok(text).root;

        assert_eq!(slice(text, &members(&root)[0].1), "inner: 1");
    }

    #[test]
    fn counts_offsets_in_utf16_over_wide_characters() {
        let text = "🦀: \"crabe\"\né: [é]\n";
        let root = read_ok(text).root;
        let top = members(&root);

        assert_eq!((top[0].1.span.start, top[0].1.span.end), (4, 11));
        assert_eq!(top[1].0, "é");
        assert_eq!(slice(text, &top[1].1), "[é]");
    }

    #[test]
    fn reads_an_alias_as_what_it_names_and_keeps_a_merge_key_as_read() {
        let text = "base: &base\n  retries: 3\nservice:\n  <<: *base\n  name: api\n";
        let root = read_ok(text).root;
        let service = members(&members(&root)[1].1);

        assert_eq!(service[0].0, "<<");
        assert_eq!(members(&service[0].1)[0].0, "retries");
        // The alias points at what it names: "show in code" goes to the anchor.
        assert_eq!(slice(text, &service[0].1), "retries: 3");
        assert_eq!(slice(text, &members(&root)[1].1), "<<: *base\n  name: api");
    }

    #[test]
    fn explores_the_first_document_of_a_stream_and_says_so() {
        let read = read_ok("a: 1\n---\nb: 2\n");

        assert_eq!(members(&read.root)[0].0, "a");
        assert_eq!(read.notices, [JsonNotice::FirstDocumentOnly]);
        assert!(read_ok("a: 1\n").notices.is_empty());
    }

    #[test]
    fn types_plain_scalars_and_leaves_quoted_ones_strings() {
        let root = read_ok("a: 0x1F\nb: '12'\nc: !!str 12\nd: 1.5e3\ne: yes\nf: FALSE\n").root;
        let values: Vec<&Value> = members(&root).iter().map(|(_, node)| &node.value).collect();

        assert_eq!(
            values,
            [
                &Value::Number("0x1F".into()),
                &Value::String("12".into()),
                &Value::String("12".into()),
                &Value::Number("1.5e3".into()),
                &Value::String("yes".into()),
                &Value::Bool(false),
            ]
        );
    }

    #[test]
    fn names_a_collection_used_as_a_key_by_its_text() {
        let root = read_ok("? [a, b]\n: pair\n").root;

        assert_eq!(members(&root)[0].0, "[a, b]");
    }

    #[test]
    fn reads_an_empty_document_as_null() {
        assert_eq!(read_ok("").root.value, Value::Null);
        assert_eq!(read_ok("# only a comment\n").root.value, Value::Null);
    }

    #[test]
    fn says_where_an_unreadable_document_breaks() {
        let error = read("a: 1\nb: [1, 2\n").expect_err("an unclosed sequence");

        assert_eq!(error.reason, JsonErrorReason::Malformed);
        assert!(error.line >= 2 && error.column >= 1);

        let error = read("ok: 1\n🦀: \"open\n").expect_err("an unclosed string");
        assert_eq!(error.reason, JsonErrorReason::Malformed);
        assert!(error.line >= 2);
    }

    #[test]
    fn refuses_a_nesting_deeper_than_it_can_walk() {
        let flow = format!("{}1{}", "[".repeat(300), "]".repeat(300));
        let block = (0..300)
            .map(|level| format!("{}a:", " ".repeat(level)))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            read(&flow).expect_err("too deep").reason,
            JsonErrorReason::TooDeep
        );
        assert_eq!(
            read(&block).expect_err("too deep").reason,
            JsonErrorReason::TooDeep
        );
    }

    #[test]
    fn stops_a_document_of_aliases_to_aliases_before_it_multiplies() {
        let mut text = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
        for level in 1..8 {
            let previous = level - 1;
            let aliases = vec![format!("*a{previous}"); 10].join(", ");
            text.push_str(
                &[
                    "a",
                    &level.to_string(),
                    ": &a",
                    &level.to_string(),
                    " [",
                    &aliases,
                    "]\n",
                ]
                .concat(),
            );
        }

        assert_eq!(
            read(&text).expect_err("too large").reason,
            JsonErrorReason::TooLarge
        );
    }
}
