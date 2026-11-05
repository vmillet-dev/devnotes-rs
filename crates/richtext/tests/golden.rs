//! The reference's editor, read in and written back (`golden/markdown`), and its typed text as it
//! was stored (`golden/markdown-escape`): byte for byte.

use serde_json::Value;

fn cases(json: &str) -> Vec<Value> {
    serde_json::from_str(json).unwrap()
}

#[test]
fn markdown_read_and_written_back_is_the_reference_s() {
    let differing: Vec<String> = cases(include_str!("../../../golden/markdown/cases.json"))
        .iter()
        .filter_map(|case| {
            let input = case["input"].as_str().unwrap();
            let written = richtext::write(&richtext::read(input));
            let expected = case["output"].as_str().unwrap();
            (written != expected)
                .then(|| format!("{input:?}\n  gave {written:?}\n  want {expected:?}"))
        })
        .collect();
    assert!(differing.is_empty(), "{}", differing.join("\n"));
}

#[test]
fn typed_text_is_escaped_as_the_reference_stored_it() {
    let differing: Vec<String> = cases(include_str!("../../../golden/markdown-escape/cases.json"))
        .iter()
        .filter_map(|case| {
            let input = case["input"].as_str().unwrap();
            let starts_line = case["options"]["startsLine"].as_bool().unwrap();
            let written = richtext::escape(input, starts_line);
            let expected = case["output"].as_str().unwrap();
            (written != expected)
                .then(|| format!("{input:?} ({starts_line}) gave {written:?}, want {expected:?}"))
        })
        .collect();
    assert!(differing.is_empty(), "{}", differing.join("\n"));
}
