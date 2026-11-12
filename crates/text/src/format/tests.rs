//! Prettier's answers as the reference got them (`golden/format`): every parser, two indents,
//! `{{fields}}` kept.

use serde_json::Value;

use super::{Answer, Options, Parser, Request, format};

fn parser(name: &str) -> Parser {
    match name {
        "babel" => Parser::Babel,
        "typescript" => Parser::Typescript,
        "json" => Parser::Json,
        "css" => Parser::Css,
        "scss" => Parser::Scss,
        "less" => Parser::Less,
        "html" => Parser::Html,
        "vue" => Parser::Vue,
        "angular" => Parser::Angular,
        "markdown" => Parser::Markdown,
        "yaml" => Parser::Yaml,
        "graphql" => Parser::Graphql,
        other => panic!("no parser {other}"),
    }
}

fn number(value: &Value) -> u32 {
    u32::try_from(value.as_u64().unwrap()).unwrap()
}

fn request(case: &Value) -> Request {
    let (input, options) = (&case["input"], &case["options"]);
    Request {
        text: input["text"].as_str().unwrap().to_owned(),
        cursor: input["cursor"]
            .as_u64()
            .map_or(0, |at| usize::try_from(at).unwrap()),
        parser: parser(input["parser"].as_str().unwrap()),
        options: Options {
            print_width: number(&options["printWidth"]),
            tab_width: number(&options["tabWidth"]),
            use_tabs: options["useTabs"].as_bool().unwrap(),
            single_quote: options["singleQuote"].as_bool().unwrap(),
            semi: options["semi"].as_bool().unwrap(),
            trailing_comma: options["trailingComma"] == "all",
        },
    }
}

fn expected(output: &Value) -> Answer {
    match output["kind"].as_str().unwrap() {
        "formatted" => Answer::Formatted {
            text: output["text"].as_str().unwrap().to_owned(),
            cursor: usize::try_from(output["cursor"].as_u64().unwrap()).unwrap(),
            changed_lines: output["changedLines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| usize::try_from(line.as_u64().unwrap()).unwrap())
                .collect(),
        },
        "unchanged" => Answer::Unchanged,
        "syntax" => Answer::Syntax {
            line: number(&output["line"]),
            column: number(&output["column"]),
        },
        "fields" => Answer::Fields,
        other => panic!("no answer {other}"),
    }
}

#[test]
fn prettier_answers_every_golden_case_as_the_reference_did() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../../../golden/format/cases.json")).unwrap();
    let differing: Vec<String> = cases
        .iter()
        .filter(|case| format(&request(case)) != expected(&case["output"]))
        .map(|case| format!("{} / {}", case["input"]["parser"], case["options"]["name"]))
        .collect();
    assert!(differing.is_empty(), "{differing:#?}");
}

#[test]
fn a_cursor_past_a_wide_character_lands_where_it_was() {
    let text = "const crab = '🦀';const b=1";
    let cursor = text.chars().count();
    let answer = format(&Request {
        text: text.to_owned(),
        cursor,
        parser: Parser::Babel,
        options: Options {
            print_width: 100,
            tab_width: 2,
            use_tabs: false,
            single_quote: true,
            semi: true,
            trailing_comma: true,
        },
    });
    let Answer::Formatted { text, cursor, .. } = answer else {
        panic!("{answer:?}");
    };
    assert_eq!(text, "const crab = '🦀';\nconst b = 1;");
    assert_eq!(
        cursor,
        text.chars().count() - 1,
        "after the 1, before the semicolon"
    );
}
