//! A TOML reader for the visualiser, on the `toml` crate's spanned values: tables, arrays of
//! tables, dotted keys and inline tables each land where they were written, in the order they
//! were written — the crate's map is sorted, so members are put back in the text's order.

use ::toml::Spanned;
use ::toml::de::{DeTable, DeValue};

use super::parse::{JsonError, JsonErrorReason, MAX_DEPTH, Node, Value};
use super::position::Positions;
use super::read::Read;

pub(crate) fn read(text: &str) -> Result<Read, JsonError> {
    let positions = Positions::new(text);
    let root = DeTable::parse(text).map_err(|error| {
        let at = error.span().map_or(0, |span| span.start);
        positions.error(JsonErrorReason::Malformed, at)
    })?;
    // The document's own span is empty: the root is the whole text.
    let root = table(root.get_ref(), &positions, 0, text.len(), 0)?;
    Ok(Read {
        root,
        notices: Vec::new(),
    })
}

fn value(
    spanned: &Spanned<DeValue<'_>>,
    positions: &Positions<'_>,
    depth: usize,
) -> Result<Node, JsonError> {
    let span = spanned.span();
    if depth > MAX_DEPTH {
        return Err(positions.error(JsonErrorReason::TooDeep, span.start));
    }
    let written = || positions.text()[span.clone()].to_string();
    let value = match spanned.get_ref() {
        DeValue::String(text) => Value::String(text.to_string()),
        DeValue::Integer(_) | DeValue::Float(_) => Value::Number(written()),
        DeValue::Boolean(value) => Value::Bool(*value),
        DeValue::Datetime(_) => Value::Datetime(written()),
        DeValue::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| value(item, positions, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        DeValue::Table(members) => return table(members, positions, span.start, span.end, depth),
    };
    Ok(Node {
        value,
        span: positions.span(span.start, span.end),
    })
}

/// A table written as `[name]` is placed on its header, an implicit one on its key.
fn table(
    members: &DeTable<'_>,
    positions: &Positions<'_>,
    start: usize,
    end: usize,
    depth: usize,
) -> Result<Node, JsonError> {
    if depth > MAX_DEPTH {
        return Err(positions.error(JsonErrorReason::TooDeep, start));
    }
    let mut written: Vec<_> = members.iter().collect();
    written.sort_by_key(|(key, _)| key.span().start);
    let members = written
        .into_iter()
        .map(|(key, child)| {
            Ok((
                key.get_ref().to_string(),
                value(child, positions, depth + 1)?,
            ))
        })
        .collect::<Result<_, JsonError>>()?;
    Ok(Node {
        value: Value::Object(members),
        span: positions.span(start, end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "title = \"Réglages\"\nwhen = 1979-05-27T07:32:00Z\nsite.name = \"api\"\n[server]\nport = 8_080 # comment\n[a.b]\nc = 0x1F\n[[products]]\nname = \"hammer\"\n[[products]]\ninline = { x = 1.5, y = [true, false] }\nname = \"nail\"\n";

    fn members(node: &Node) -> &[(String, Node)] {
        let Value::Object(members) = &node.value else {
            panic!("a table, not {:?}", node.value);
        };
        members
    }

    fn slice(text: &str, node: &Node) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[node.span.start as usize..node.span.end as usize]).unwrap()
    }

    fn root() -> Node {
        read(CONFIG).expect("a readable document").root
    }

    #[test]
    fn keeps_the_order_the_tables_and_keys_were_written_in() {
        let root = root();

        assert_eq!(
            members(&root)
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["title", "when", "site", "server", "a", "products"]
        );
        let Value::Array(products) = &members(&root)[5].1.value else {
            panic!("an array of tables");
        };
        assert_eq!(
            members(&products[1])
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["inline", "name"]
        );
    }

    #[test]
    fn places_tables_arrays_of_tables_dotted_keys_and_inline_tables_where_they_were_written() {
        let root = root();
        let top = members(&root);

        assert_eq!(slice(CONFIG, &top[2].1), "site");
        assert_eq!(slice(CONFIG, &members(&top[2].1)[0].1), "\"api\"");
        assert_eq!(slice(CONFIG, &top[3].1), "[server]");
        assert_eq!(slice(CONFIG, &members(&top[4].1)[0].1), "[a.b]");
        let Value::Array(products) = &top[5].1.value else {
            panic!("an array of tables");
        };
        assert_eq!(slice(CONFIG, &products[1]), "[[products]]");
        assert_eq!(
            slice(CONFIG, &members(&products[1])[0].1),
            "{ x = 1.5, y = [true, false] }"
        );
        assert_eq!(root.span.start, 0);
    }

    #[test]
    fn shows_numbers_and_datetimes_as_written() {
        let root = root();
        let top = members(&root);

        assert_eq!(
            top[1].1.value,
            Value::Datetime("1979-05-27T07:32:00Z".into())
        );
        assert_eq!(members(&top[3].1)[0].1.value, Value::Number("8_080".into()));
        assert_eq!(
            members(&members(&top[4].1)[0].1)[0].1.value,
            Value::Number("0x1F".into())
        );
        assert_eq!(top[0].1.value, Value::String("Réglages".into()));
    }

    #[test]
    fn counts_offsets_in_utf16_over_wide_characters() {
        let text = "\"🦀\" = \"crabe\"\nnext = 1\n";
        let root = read(text).unwrap().root;
        let top = members(&root);

        assert_eq!(top[0].0, "🦀");
        assert_eq!((top[0].1.span.start, top[0].1.span.end), (7, 14));
        assert_eq!(slice(text, &top[1].1), "1");
    }

    #[test]
    fn says_where_an_unreadable_document_breaks() {
        let error = read("a = 1\nb = \n").expect_err("a key with no value");

        assert_eq!(error.reason, JsonErrorReason::Malformed);
        assert_eq!(error.line, 2);
        assert!(error.column >= 4);
    }

    #[test]
    fn refuses_a_nesting_deeper_than_it_can_walk() {
        let deep = format!("a = {}1{}", "[".repeat(300), "]".repeat(300));

        let error = read(&deep).expect_err("too deep");
        assert!(matches!(
            error.reason,
            JsonErrorReason::TooDeep | JsonErrorReason::Malformed
        ));
    }

    #[test]
    fn reads_an_empty_document_as_an_empty_table() {
        assert_eq!(read("").unwrap().root.value, Value::Object(Vec::new()));
    }
}
