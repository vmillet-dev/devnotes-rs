use crate::closed_enum::closed_enum;

use super::model::{Note, NoteDraft, NotePatch};

closed_enum! {
    /// Closed: the front receives a generated union, so an unknown value stops compiling
    /// there instead of being refused at runtime.
    pub enum Language {
        Json = "json",
        Js = "js",
        Ts = "ts",
        Py = "py",
        Rs = "rs",
        Go = "go",
        Java = "java",
        Cs = "cs",
        Php = "php",
        C = "c",
        Sql = "sql",
        Yml = "yml",
        Toml = "toml",
        Xml = "xml",
        Html = "html",
        Css = "css",
        Sh = "sh",
        Md = "md",
        /// Default, and the signal that the front end chose nothing.
        #[default]
        Txt = "txt",
    }
}

/// The front end's choice if it made one, `txt` meaning it made none.
pub fn for_draft(draft: &NoteDraft) -> Language {
    if draft.language == Language::default() {
        from_content(&draft.content)
    } else {
        draft.language
    }
}

/// Creation sees no content, the paste coming after: without this the note stays `txt`. The
/// three refusals keep detection from becoming a permanent correction.
pub fn after_patch(before: &Note, patch: &NotePatch) -> Option<Language> {
    if patch.language.is_some()
        || before.language != Language::default()
        || !before.content.trim().is_empty()
    {
        return None;
    }

    let content = patch.content.as_ref()?;
    if content.trim().is_empty() {
        return None;
    }

    Some(from_content(content))
}

/// What one marker is worth. Three tiers and no more: a marker declares how much it proves,
/// and ten steps would bring back an ordering nobody can read.
const SIGNATURE: u32 = 6;
const STRONG: u32 = 3;
const WEAK: u32 = 1;

/// Below this, the highest score is not an answer: one weak marker is not enough, two are.
/// A wrong language colours the body, badges the card and files the note under a facet;
/// `txt` says nothing, which is honest. A tie is no answer either.
const MIN_CONFIDENCE: u32 = 2;

/// The content in the shapes the markers read it in, computed once.
struct Sample<'a> {
    trimmed: &'a str,
    lower: String,
    lines: Vec<&'a str>,
}

impl<'a> Sample<'a> {
    fn of(content: &'a str) -> Self {
        let trimmed = content.trim();
        Self {
            trimmed,
            lower: trimmed.to_lowercase(),
            lines: trimmed.lines().map(str::trim).collect(),
        }
    }

    fn has(&self, needle: &str) -> bool {
        self.trimmed.contains(needle)
    }

    fn any_line(&self, predicate: impl Fn(&str) -> bool) -> bool {
        self.lines.iter().copied().any(predicate)
    }

    fn line_starts_with_any(&self, prefixes: &[&str]) -> bool {
        self.any_line(|line| starts_with_any(line, prefixes))
    }
}

/// Adds a weight when a marker is present, which is the whole of the arithmetic.
fn worth(weight: u32, present: bool) -> u32 {
    if present { weight } else { 0 }
}

/// How much one language believes a sample is its own.
type Scorer = fn(&Sample<'_>) -> u32;

/// Every language that can be guessed at, each scoring itself. The order means nothing: a
/// language added here needs markers with honest weights, not a slot.
const SCORERS: [(Language, Scorer); 18] = [
    (Language::Json, score_json),
    (Language::Php, score_php),
    (Language::Xml, score_xml),
    (Language::Html, score_html),
    (Language::Sql, score_sql),
    (Language::Toml, score_toml),
    (Language::Py, score_python),
    (Language::Go, score_go),
    (Language::Rs, score_rust),
    (Language::Java, score_java),
    (Language::Cs, score_csharp),
    (Language::C, score_c),
    (Language::Ts, score_typescript),
    (Language::Js, score_javascript),
    (Language::Css, score_css),
    (Language::Yml, score_yaml),
    (Language::Md, score_markdown),
    (Language::Sh, score_shell),
];

/// The best-scoring language, or `txt` when nothing proved itself. Weights settle what an
/// order would: `<?php` outweighs the `<` of a processing instruction, and Rust's `println!`
/// outweighs JavaScript's `=>`.
pub fn from_content(content: &str) -> Language {
    let sample = Sample::of(content);
    if sample.trimmed.is_empty() {
        return Language::default();
    }
    // A shebang settles the question on its own, whatever follows it.
    if sample.trimmed.starts_with("#!") {
        return Language::Sh;
    }

    let mut best = (Language::default(), 0);
    let mut runner_up = 0;
    for (language, score) in SCORERS {
        let scored = score(&sample);
        if scored > best.1 {
            runner_up = best.1;
            best = (language, scored);
        } else if scored > runner_up {
            runner_up = scored;
        }
    }

    if best.1 < MIN_CONFIDENCE || best.1 == runner_up {
        return Language::default();
    }
    best.0
}

fn starts_with_any(line: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| line.starts_with(prefix))
}

/// Structural, and worth a signature: nothing else here is a quoted object or an array from
/// its first character to its last.
fn score_json(sample: &Sample<'_>) -> u32 {
    let content = sample.trimmed;
    let wrapped = (content.starts_with('{') && content.ends_with('}'))
        || (content.starts_with('[') && content.ends_with(']'));

    worth(
        SIGNATURE,
        wrapped && (content.contains('"') || content.starts_with('[')),
    )
}

/// A signature, which keeps it ahead of the markup score its `<` also earns.
fn score_php(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.lower.starts_with("<?php")) + worth(WEAK, sample.has("->"))
}

/// The generic shape is XML's alone, and HTML scores only what is its own: sharing it made
/// the two tie on a plain `<config>…</config>`, and a tie is `txt`.
fn score_xml(sample: &Sample<'_>) -> u32 {
    let looks_like_markup =
        sample.lower.starts_with('<') && sample.trimmed.ends_with('>') && sample.has("</");

    worth(STRONG, looks_like_markup) + worth(SIGNATURE, sample.lower.starts_with("<?xml"))
}

/// Its tags are a signature: they must outweigh the markup shape XML earns on the same text.
fn score_html(sample: &Sample<'_>) -> u32 {
    const TAGS: [&str; 8] = [
        "<div", "<span", "<p>", "<body", "<head", "<a ", "<ul", "<table",
    ];

    let doctype = sample.lower.starts_with("<!doctype html") || sample.lower.starts_with("<html");

    worth(SIGNATURE, doctype) + worth(SIGNATURE, TAGS.iter().any(|tag| sample.lower.contains(tag)))
}

fn score_sql(sample: &Sample<'_>) -> u32 {
    const STATEMENTS: [&str; 8] = [
        "select ",
        "insert into",
        "update ",
        "delete from",
        "create table",
        "alter table",
        "drop table",
        "with ",
    ];

    worth(STRONG, starts_with_any(&sample.lower, &STATEMENTS))
        + worth(
            WEAK,
            sample.lower.contains(" from ") || sample.lower.contains(" where "),
        )
}

/// A section and an assignment: `[…]` alone could be an array on its own line.
fn score_toml(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.any_line(is_toml_section) && sample.any_line(is_assignment),
    )
}

fn is_toml_section(line: &str) -> bool {
    let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
        return false;
    };
    let inner = inner.trim_matches(|c| c == '[' || c == ']');

    !inner.is_empty() && inner.chars().all(is_identifier_char)
}

fn is_assignment(line: &str) -> bool {
    line.split_once('=').is_some_and(|(key, _)| {
        let key = key.trim();
        !key.is_empty() && !key.contains(char::is_whitespace)
    })
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// `class` needs its trailing colon: without it, this is TypeScript's.
fn score_python(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("__name__"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["def ", "async def ", "elif "]),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("class ") && line.ends_with(':')),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("from ") && line.contains(" import ")),
        )
        + worth(WEAK, sample.has("self."))
}

/// `fmt.Print` and `package main` are Go's alone; `import (` is its grouped form.
fn score_go(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("fmt.Print"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["func ", "package main", "import ("]),
        )
        + worth(WEAK, sample.has(":=") || sample.has("err != nil"))
}

/// `struct`, `trait` and `enum` count, weakly: a marker Rust shares with TypeScript is worth
/// little, not nothing.
fn score_rust(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("println!") || sample.has("let mut "))
        + worth(
            STRONG,
            sample.any_line(|line| {
                let line = line.strip_prefix("pub ").unwrap_or(line);
                starts_with_any(line, &["fn ", "async fn ", "impl "])
            }),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("use ") && line.contains("::")),
        )
        + worth(
            WEAK,
            sample.any_line(|line| {
                let line = line.strip_prefix("pub ").unwrap_or(line);
                starts_with_any(line, &["struct ", "trait ", "enum "])
            }),
        )
        + worth(
            WEAK,
            sample.has("&str") || sample.has("Vec<") || sample.has("Option<"),
        )
}

/// `public class` is left to neither this nor C#: both write it.
fn score_java(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.has("System.out.print") || sample.has("public static void main"),
    ) + worth(
        STRONG,
        sample.any_line(|line| line.starts_with("import java")),
    )
}

fn score_csharp(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("Console.Write"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["using System", "namespace "]),
        )
}

/// `#include` is the one marker nothing else here writes.
fn score_c(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.any_line(|line| line.starts_with("#include")),
    ) + worth(STRONG, sample.has("int main(") && sample.has("printf("))
}

/// Its own markers only, never JavaScript's: inheriting them would tie the two on every file.
fn score_typescript(sample: &Sample<'_>) -> u32 {
    const ANNOTATIONS: [&str; 4] = [": string", ": number", ": boolean", "implements "];
    const DECLARATIONS: [&str; 4] = ["interface ", "type ", "enum ", "declare "];

    worth(STRONG, ANNOTATIONS.iter().any(|marker| sample.has(marker)))
        + worth(
            STRONG,
            sample.any_line(|line| {
                let line = line.strip_prefix("export ").unwrap_or(line);
                starts_with_any(line, &DECLARATIONS)
            }),
        )
}

/// `=>` is the greediest marker in this file — C++, Kotlin, Swift, Scala, Dart and anything
/// with a lambda write it — so it is worth the least.
fn score_javascript(sample: &Sample<'_>) -> u32 {
    const KEYWORDS: [&str; 7] = [
        "function ",
        "const ",
        "let ",
        "var ",
        "export ",
        "import ",
        "class ",
    ];

    worth(
        SIGNATURE,
        sample.has("console.log") || sample.has("require("),
    ) + worth(WEAK, sample.has("=>"))
        + worth(WEAK, sample.line_starts_with_any(&KEYWORDS))
}

/// A selector and a declaration, plus something only a stylesheet writes: the shape alone is
/// strong evidence, not proof — `export interface Note {` and `id: string;` have it too.
fn score_css(sample: &Sample<'_>) -> u32 {
    if !sample.has("{") || !sample.has("}") {
        return 0;
    }

    let has_declaration = sample.any_line(|line| {
        line.find(':')
            .zip(line.find(';'))
            .is_some_and(|(colon, semicolon)| colon < semicolon)
    });
    let has_selector = sample.any_line(|line| {
        line.ends_with('{')
            && line.chars().next().is_some_and(|c| {
                c.is_ascii_alphabetic() || matches!(c, '.' | '#' | '@' | ':' | '*')
            })
    });
    let stylesheet_only = sample.has("--")
        || ["px;", "rem;", "%;", "px ", "em;", "vh;", "vw;", "fr;"]
            .iter()
            .any(|unit| sample.has(unit));

    worth(STRONG, has_declaration && has_selector) + worth(WEAK, stylesheet_only)
}

fn score_yaml(sample: &Sample<'_>) -> u32 {
    // These belong to languages that score on them instead.
    if sample.has(";") || sample.has("{") {
        return 0;
    }

    worth(SIGNATURE, sample.trimmed.starts_with("---"))
        + worth(STRONG, sample.any_line(is_mapping))
        + worth(WEAK, sample.any_line(|line| line.starts_with("- ")))
}

/// The space required after the colon rules out a URL, whose `http://…` would otherwise
/// read as a key.
fn is_mapping(line: &str) -> bool {
    let Some((key, value)) = line.split_once(':') else {
        return false;
    };

    !key.is_empty()
        && key.chars().all(is_identifier_char)
        && (value.is_empty() || value.starts_with(' '))
}

fn score_markdown(sample: &Sample<'_>) -> u32 {
    const LINE_MARKERS: [&str; 5] = ["# ", "## ", "### ", "* ", "> "];

    worth(SIGNATURE, sample.has("```"))
        + worth(STRONG, sample.has("]("))
        + worth(STRONG, sample.line_starts_with_any(&LINE_MARKERS))
}

/// Deliberately thin: a wide list would catch prose.
fn score_shell(sample: &Sample<'_>) -> u32 {
    const COMMANDS: [&str; 10] = [
        "echo ", "cd ", "ls ", "cat ", "grep ", "sudo ", "npm ", "git ", "docker ", "curl ",
    ];

    worth(STRONG, sample.line_starts_with_any(&COMMANDS))
        + worth(
            WEAK,
            sample.any_line(|line| line.starts_with('$') || line.starts_with("./")),
        )
        + worth(WEAK, sample.has(" | ") || sample.has(" && "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::checklist::NoteKind;
    use crate::notes::model::NoteLifecycle;

    #[test]
    fn every_variant_round_trips_through_its_stored_form() {
        for language in Language::ALL {
            assert_eq!(language.as_str().parse(), Ok(language));
        }
    }

    #[test]
    fn an_unknown_value_is_refused_rather_than_guessed() {
        assert_eq!("from-the-future".parse::<Language>(), Err(()));
        assert_eq!("JSON".parse::<Language>(), Err(()));
    }

    #[test]
    fn the_default_is_the_one_detection_replaces() {
        assert_eq!(Language::default(), Language::Txt);
    }

    #[test]
    fn the_serialized_form_matches_the_stored_one() {
        for language in Language::ALL {
            let json = serde_json::to_value(language).unwrap();
            assert_eq!(json, serde_json::json!(language.as_str()));
        }
    }

    fn draft(language: Language, content: &str) -> NoteDraft {
        NoteDraft {
            space_id: "s-1".to_string(),
            folder_id: None,
            title: String::new(),
            language,
            content: content.to_string(),
            source: String::new(),
            tags: Vec::new(),
            pinned: false,
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::Snippet,
            items: Vec::new(),
        }
    }

    #[test]
    fn a_draft_with_no_chosen_language_gets_the_detected_one() {
        assert_eq!(for_draft(&draft(Language::Txt, "SELECT 1")), Language::Sql);
    }

    #[test]
    fn a_chosen_language_is_never_overwritten() {
        assert_eq!(for_draft(&draft(Language::Md, "SELECT 1")), Language::Md);
    }

    #[test]
    fn an_empty_draft_stays_plain_text() {
        assert_eq!(for_draft(&draft(Language::Txt, "")), Language::Txt);
    }

    fn blank_note() -> Note {
        Note {
            language: Language::Txt,
            content: String::new(),
            ..crate::notes::fixtures::note()
        }
    }

    fn content_patch(content: &str) -> NotePatch {
        NotePatch {
            content: Some(content.to_string()),
            ..NotePatch::default()
        }
    }

    #[test]
    fn an_empty_note_receiving_its_first_content_gets_a_language() {
        let detected = after_patch(&blank_note(), &content_patch("interface A { id: string }"));

        assert_eq!(detected, Some(Language::Ts));
    }

    #[test]
    fn a_note_that_already_had_content_keeps_its_language() {
        let note = Note {
            content: "some prose".to_string(),
            ..blank_note()
        };

        assert!(after_patch(&note, &content_patch("SELECT 1")).is_none());
    }

    #[test]
    fn a_chosen_language_is_never_corrected_by_a_later_paste() {
        let note = Note {
            language: Language::Md,
            ..blank_note()
        };

        assert!(after_patch(&note, &content_patch("SELECT 1")).is_none());
    }

    #[test]
    fn a_patch_setting_the_language_itself_is_left_alone() {
        let patch = NotePatch {
            language: Some(Language::Md),
            ..content_patch("SELECT 1")
        };

        assert!(after_patch(&blank_note(), &patch).is_none());
    }

    #[test]
    fn a_patch_carrying_no_content_detects_nothing() {
        let patch = NotePatch {
            title: Some("Title".to_string()),
            ..NotePatch::default()
        };

        assert!(after_patch(&blank_note(), &patch).is_none());
    }

    #[test]
    fn clearing_the_content_does_not_detect() {
        assert!(after_patch(&blank_note(), &content_patch("   ")).is_none());
    }

    #[test]
    fn every_detected_language_is_one_the_editor_accepts() {
        let samples = [
            "",
            "#!/bin/bash\necho hi",
            "{\"a\": 1}",
            "<?xml version=\"1.0\"?><a/>",
            "<html><body>hi</body></html>",
            "SELECT 1",
            "[package]\nname = \"x\"",
            "def f():\n    pass",
            "interface A { }",
            "const a = 1",
            ".a { color: red; }",
            "key: value",
            "# Title\n\n- item",
            "git status",
            "just some prose",
        ];

        for sample in samples {
            assert!(
                Language::ALL.contains(&from_content(sample)),
                "sample: {sample}"
            );
        }
    }

    #[test]
    fn an_empty_or_blank_content_stays_plain_text() {
        assert_eq!(from_content(""), Language::Txt);
        assert_eq!(from_content("   \n  "), Language::Txt);
    }

    #[test]
    fn prose_stays_plain_text() {
        assert_eq!(
            from_content("Penser à relancer Marc au sujet du certificat"),
            Language::Txt
        );
    }

    #[test]
    fn a_json_object_or_array_is_recognized() {
        assert_eq!(from_content("{\n  \"id\": 42\n}"), Language::Json);
        assert_eq!(from_content("[1, 2, 3]"), Language::Json);
        assert_eq!(from_content("  {\"a\": [1]}  "), Language::Json);
    }

    #[test]
    fn a_shebang_wins_over_everything_that_follows() {
        assert_eq!(
            from_content("#!/usr/bin/env python\nimport os"),
            Language::Sh
        );
    }

    #[test]
    fn markup_splits_between_html_and_xml() {
        assert_eq!(
            from_content("<!DOCTYPE html>\n<html></html>"),
            Language::Html
        );
        assert_eq!(from_content("<div class=\"x\">hi</div>"), Language::Html);
        assert_eq!(
            from_content("<?xml version=\"1.0\"?>\n<root/>"),
            Language::Xml
        );
        assert_eq!(from_content("<config><item/></config>"), Language::Xml);
    }

    #[test]
    fn sql_is_recognized_whatever_its_case() {
        assert_eq!(from_content("SELECT * FROM notes"), Language::Sql);
        assert_eq!(from_content("select 1"), Language::Sql);
        assert_eq!(
            from_content("CREATE TABLE notes (id TEXT PRIMARY KEY)"),
            Language::Sql
        );
    }

    #[test]
    fn toml_needs_both_a_section_and_an_assignment() {
        assert_eq!(
            from_content("[package]\nname = \"devnotes\""),
            Language::Toml
        );
        assert_ne!(from_content("[1, 2]\nx = 3"), Language::Toml);
    }

    #[test]
    fn python_is_told_apart_from_typescript_by_its_colon() {
        assert_eq!(from_content("def run():\n    return 1"), Language::Py);
        assert_eq!(from_content("class Note:\n    pass"), Language::Py);
        assert_eq!(from_content("from os import path"), Language::Py);
    }

    /// `class Note { }` is written the same way in Java, C#, PHP, Dart, TypeScript and
    /// JavaScript: one weak marker is not an answer.
    #[test]
    fn one_weak_marker_is_not_an_answer() {
        assert_eq!(from_content("class Note { }"), Language::Txt);
        assert_eq!(from_content("x => x + 1"), Language::Txt);
    }

    #[test]
    fn a_compiled_language_is_not_taken_for_javascript() {
        assert_eq!(
            from_content(
                "fn main() {
    println!(\"hi\");
}"
            ),
            Language::Rs
        );
        assert_eq!(
            from_content(
                "match value {
    Some(x) => x,
}
let mut total = 0;"
            ),
            Language::Rs
        );
        assert_eq!(
            from_content(
                "package main

func main() {
    fmt.Println(\"hi\")
}"
            ),
            Language::Go
        );
        assert_eq!(
            from_content(
                "import java.util.List;

class Note { }"
            ),
            Language::Java
        );
        assert_eq!(
            from_content(
                "using System;

var greet = () => Console.WriteLine(\"hi\");"
            ),
            Language::Cs
        );
        assert_eq!(
            from_content(
                "#include <stdio.h>

int main(void) { return 0; }"
            ),
            Language::C
        );
    }

    /// `<?php` opens with a `<`, which the markup check reads as a processing instruction.
    #[test]
    fn php_is_recognised_before_the_markup_check_claims_it() {
        assert_eq!(
            from_content(
                "<?php

echo 'hi';"
            ),
            Language::Php
        );
        assert_eq!(from_content("<?xml version=\"1.0\"?>"), Language::Xml);
    }

    #[test]
    fn the_languages_detected_before_them_are_left_alone() {
        assert_eq!(from_content("export enum Kind { A }"), Language::Ts);
        assert_eq!(from_content("interface Note { id: string }"), Language::Ts);
        assert_eq!(
            from_content(
                "def run():
    return 1"
            ),
            Language::Py
        );
        assert_eq!(from_content("select 1"), Language::Sql);
        assert_eq!(from_content("const add = (a, b) => a + b"), Language::Js);
    }

    #[test]
    fn typescript_wins_over_javascript_on_its_own_markers() {
        assert_eq!(from_content("interface Note { id: string }"), Language::Ts);
        assert_eq!(from_content("export type Id = string"), Language::Ts);
        assert_eq!(from_content("const a: number = 1"), Language::Ts);
        assert_eq!(from_content("const add = (a, b) => a + b"), Language::Js);
        assert_eq!(from_content("console.log('hi')"), Language::Js);
    }

    #[test]
    fn css_needs_a_selector_and_a_declaration() {
        assert_eq!(from_content(".card {\n  color: red;\n}"), Language::Css);
        assert_eq!(
            from_content("@media print {\n  a { color: #000; }\n}"),
            Language::Css
        );
    }

    #[test]
    fn yaml_is_recognized_by_its_mappings_and_lists() {
        assert_eq!(from_content("name: devnotes\nversion: 1"), Language::Yml);
        assert_eq!(from_content("---\nsteps:\n  - build"), Language::Yml);
    }

    #[test]
    fn a_bare_url_is_not_read_as_a_yaml_mapping() {
        assert_ne!(from_content("https://example.com/a/b"), Language::Yml);
    }

    #[test]
    fn markdown_is_recognized_by_its_headings_and_fences() {
        assert_eq!(from_content("# Heading\n\nA paragraph."), Language::Md);
        assert_eq!(from_content("Voir ```code``` ici"), Language::Md);
        assert_eq!(from_content("Un [lien](https://x.dev)"), Language::Md);
    }

    #[test]
    fn shell_commands_are_the_last_resort() {
        assert_eq!(from_content("git status\ngit push"), Language::Sh);
        assert_eq!(from_content("docker run -p 8080:80 nginx"), Language::Sh);
    }
}
