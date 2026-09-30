//! The Texte panel: the case converter, the slug generator and the line breaks.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::count::saturating_u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TextCase {
    Camel,
    Pascal,
    Snake,
    Kebab,
    Constant,
    Title,
    Sentence,
    Dot,
    Path,
    Train,
    Lower,
    Upper,
    Flat,
}

/** In the order the tool lists them. */
const CASES: [TextCase; 13] = [
    TextCase::Camel,
    TextCase::Pascal,
    TextCase::Snake,
    TextCase::Kebab,
    TextCase::Constant,
    TextCase::Title,
    TextCase::Sentence,
    TextCase::Dot,
    TextCase::Path,
    TextCase::Train,
    TextCase::Lower,
    TextCase::Upper,
    TextCase::Flat,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaseConversion {
    pub case: TextCase,
    pub value: String,
}

/// Words as a reader sees them: apart at anything but a letter or a digit, at a lower case or a
/// digit followed by a capital (`fooBar`, `v2Beta`), and at the end of an acronym (`HTTPServer`
/// is `HTTP` and `Server`). A digit stays with what it follows.
fn words(text: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    let mut previous: Option<char> = None;
    let mut chars = text.char_indices().peekable();

    while let Some((index, current)) = chars.next() {
        if !current.is_alphanumeric() {
            if let Some(begun) = start.take() {
                words.push(&text[begun..index]);
            }
            previous = None;
            continue;
        }

        let next_is_lower = chars.peek().is_some_and(|&(_, next)| next.is_lowercase());
        let boundary = current.is_uppercase()
            && previous.is_some_and(|before| {
                before.is_lowercase()
                    || before.is_numeric()
                    || (before.is_uppercase() && next_is_lower)
            });
        if boundary && let Some(begun) = start.replace(index) {
            words.push(&text[begun..index]);
        }
        start.get_or_insert(index);
        previous = Some(current);
    }

    if let Some(begun) = start {
        words.push(&text[begun..]);
    }
    words
}

/// `ß` becomes `SS` in upper case: a character's case may be longer than the character.
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}

fn in_case(words: &[&str], case: TextCase) -> String {
    let lower = || words.iter().map(|word| word.to_lowercase());
    let upper = || words.iter().map(|word| word.to_uppercase());
    let capital = || words.iter().map(|word| capitalised(word));
    let joined = |parts: Vec<String>, separator: &str| parts.join(separator);
    match case {
        TextCase::Camel => lower().take(1).chain(capital().skip(1)).collect(),
        TextCase::Pascal => capital().collect(),
        TextCase::Snake => joined(lower().collect(), "_"),
        TextCase::Kebab => joined(lower().collect(), "-"),
        TextCase::Constant => joined(upper().collect(), "_"),
        TextCase::Title => joined(capital().collect(), " "),
        TextCase::Sentence => joined(capital().take(1).chain(lower().skip(1)).collect(), " "),
        TextCase::Dot => joined(lower().collect(), "."),
        TextCase::Path => joined(lower().collect(), "/"),
        TextCase::Train => joined(capital().collect(), "-"),
        TextCase::Lower => joined(lower().collect(), " "),
        TextCase::Upper => joined(upper().collect(), " "),
        TextCase::Flat => lower().collect(),
    }
}

/// Every case at once, line by line — a list of identifiers comes back as a list; blank lines at
/// either end are dropped, those inside kept so each line keeps its place. Nothing for no word.
pub fn convert_case(text: &str) -> Vec<CaseConversion> {
    let lines: Vec<Vec<&str>> = text.lines().map(words).collect();
    let (Some(first), Some(last)) = (
        lines.iter().position(|words| !words.is_empty()),
        lines.iter().rposition(|words| !words.is_empty()),
    ) else {
        return Vec::new();
    };
    CASES
        .into_iter()
        .map(|case| CaseConversion {
            case,
            value: lines[first..=last]
                .iter()
                .map(|words| in_case(words, case))
                .collect::<Vec<_>>()
                .join(
                    "
",
                ),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SlugSeparator {
    Dash,
    Underscore,
    Dot,
}

impl SlugSeparator {
    fn as_char(self) -> char {
        match self {
            Self::Dash => '-',
            Self::Underscore => '_',
            Self::Dot => '.',
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SlugRequest {
    pub text: String,
    pub separator: SlugSeparator,
    pub lowercase: bool,
}

/// Transliterated, never dropped: "Été" is `ete`, "Straße" `strasse`, "Привет" `privet`.
pub fn slugify(request: &SlugRequest) -> String {
    let ascii = deunicode::deunicode(&request.text);
    let separator = request.separator.as_char();
    let mut slug = String::with_capacity(ascii.len());
    let mut apart = false;

    for character in ascii.chars() {
        if character.is_ascii_alphanumeric() {
            if apart && !slug.is_empty() {
                slug.push(separator);
            }
            apart = false;
            slug.push(if request.lowercase {
                character.to_ascii_lowercase()
            } else {
                character
            });
        } else {
            apart = true;
        }
    }
    slug
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LineEnding {
    Lf,
    Crlf,
    Cr,
}

impl LineEnding {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FinalNewline {
    Keep,
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Trim {
    Keep,
    End,
    Both,
}

/// A line holding only whitespace counts as empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EmptyLines {
    Keep,
    /// One at most between two paragraphs.
    Collapse,
    Remove,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineBreaksRequest {
    pub text: String,
    /// `None` keeps each line's own ending.
    pub ending: Option<LineEnding>,
    pub trim: Trim,
    pub empty_lines: EmptyLines,
    /// Runs of spaces inside a line become one, the spaces that pass for one become one.
    pub normalize: bool,
    pub final_newline: FinalNewline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EndingCounts {
    pub lf: u32,
    pub crlf: u32,
    pub cr: u32,
}

impl EndingCounts {
    /// What a newline added to the end should be: the text's own habit, `LF` on a tie or none.
    fn most_used(self) -> LineEnding {
        if self.crlf > self.lf && self.crlf >= self.cr {
            LineEnding::Crlf
        } else if self.cr > self.lf && self.cr > self.crlf {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FinalNewlineChange {
    Unchanged,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineBreaksAnswer {
    /// In the text as it was given.
    pub found: EndingCounts,
    pub text: String,
    /// Endings rewritten to another kind.
    pub converted: u32,
    /// Lines that lost whitespace at an end.
    pub trimmed: u32,
    /// Empty lines taken out.
    pub removed_lines: u32,
    /// Whitespace characters replaced, merged into one, or removed as invisible.
    pub normalized: u32,
    pub final_newline: FinalNewlineChange,
}

/// Each line and the ending that closes it; the last one has none, and is empty when the text
/// ends with a newline.
fn lines(text: &str) -> Vec<(&str, Option<LineEnding>)> {
    let mut lines = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut index = 0;

    while index < bytes.len() {
        let ending = match bytes[index] {
            b'\n' => Some((LineEnding::Lf, 1)),
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => Some((LineEnding::Crlf, 2)),
            b'\r' => Some((LineEnding::Cr, 1)),
            _ => None,
        };
        if let Some((ending, width)) = ending {
            lines.push((&text[start..index], Some(ending)));
            index += width;
            start = index;
        } else {
            index += 1;
        }
    }
    lines.push((&text[start..], None));
    lines
}

/// Spaces that pass for one: no-break, narrow, the typographic widths, ideographic.
fn passes_for_a_space(character: char) -> bool {
    matches!(
        character,
        '\u{a0}' | '\u{202f}' | '\u{2000}'..='\u{200a}' | '\u{205f}' | '\u{3000}'
    )
}

/// Invisible, and nothing to keep: a zero-width space, a word joiner, a byte order mark. The
/// zero-width joiners are left alone — an emoji sequence is made of them.
fn invisible(character: char) -> bool {
    matches!(character, '\u{200b}' | '\u{2060}' | '\u{feff}')
}

/// A line's whitespace made plain: the indentation kept, every other run one space.
fn normalized(line: &str) -> (String, usize) {
    let mut out = String::with_capacity(line.len());
    let mut changed = 0;
    let mut begun = false;
    let mut in_run = false;
    for character in line.chars() {
        if invisible(character) {
            changed += 1;
            continue;
        }
        let space = character == ' ' || character == '\t' || passes_for_a_space(character);
        if !space {
            begun = true;
            in_run = false;
            out.push(character);
        } else if !begun {
            if passes_for_a_space(character) {
                changed += 1;
                out.push(' ');
            } else {
                out.push(character);
            }
        } else if in_run {
            changed += 1;
        } else {
            in_run = true;
            if character != ' ' {
                changed += 1;
            }
            out.push(' ');
        }
    }
    (out, changed)
}

fn trimmed(line: &str, trim: Trim) -> &str {
    match trim {
        Trim::Keep => line,
        Trim::End => line.trim_end(),
        Trim::Both => line.trim(),
    }
}

/// Which endings the text has, converted to one if asked, its lines trimmed, emptied or
/// normalised if asked, and the final newline added or removed — each change counted.
pub fn fix_line_breaks(request: &LineBreaksRequest) -> LineBreaksAnswer {
    let lines = lines(&request.text);
    let last = lines.len() - 1;
    let mut found = EndingCounts::default();
    let (mut converted, mut trimmed_lines, mut removed, mut normalized_count) = (0, 0, 0, 0);
    let mut rewritten: Vec<(String, Option<LineEnding>)> = Vec::with_capacity(lines.len());
    let mut previous_blank = false;
    for (index, (content, ending)) in lines.into_iter().enumerate() {
        match ending {
            Some(LineEnding::Lf) => found.lf += 1,
            Some(LineEnding::Crlf) => found.crlf += 1,
            Some(LineEnding::Cr) => found.cr += 1,
            None => {}
        }
        let (plain, changed) = if request.normalize {
            normalized(content)
        } else {
            (content.to_owned(), 0)
        };
        normalized_count += changed;
        let kept = trimmed(&plain, request.trim);
        if kept.len() != plain.len() {
            trimmed_lines += 1;
        }

        // The last piece has no ending: it is where the text stops, not a line to take out.
        let blank = index != last && kept.trim().is_empty();
        let dropped = blank
            && match request.empty_lines {
                EmptyLines::Keep => false,
                EmptyLines::Collapse => previous_blank,
                EmptyLines::Remove => true,
            };
        previous_blank = blank;
        if dropped {
            removed += 1;
            continue;
        }

        let ending = ending.map(|own| match request.ending {
            Some(wanted) if wanted != own => {
                converted += 1;
                wanted
            }
            _ => own,
        });
        rewritten.push((kept.to_owned(), ending));
    }

    let added = request.ending.unwrap_or_else(|| found.most_used());
    let final_newline = settle_final_newline(&mut rewritten, request.final_newline, added);
    let text = rewritten
        .iter()
        .map(|(content, ending)| content.clone() + ending.map_or("", LineEnding::as_str))
        .collect();

    LineBreaksAnswer {
        found,
        text,
        converted: saturating_u32(converted),
        trimmed: saturating_u32(trimmed_lines),
        removed_lines: saturating_u32(removed),
        normalized: saturating_u32(normalized_count),
        final_newline,
    }
}

/// An empty text is left alone: a newline added to nothing would be the whole of it.
fn settle_final_newline(
    lines: &mut Vec<(String, Option<LineEnding>)>,
    wanted: FinalNewline,
    added: LineEnding,
) -> FinalNewlineChange {
    let ends_with_newline =
        lines.len() > 1 && lines.last().is_some_and(|(content, _)| content.is_empty());

    match wanted {
        FinalNewline::Add
            if !ends_with_newline && lines.iter().any(|(content, _)| !content.is_empty()) =>
        {
            if let Some(last) = lines.last_mut() {
                last.1 = Some(added);
            }
            lines.push((String::new(), None));
            FinalNewlineChange::Added
        }
        FinalNewline::Remove if ends_with_newline => {
            lines.pop();
            if let Some(last) = lines.last_mut() {
                last.1 = None;
            }
            FinalNewlineChange::Removed
        }
        _ => FinalNewlineChange::Unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(text: &str, wanted: TextCase) -> String {
        convert_case(text)
            .into_iter()
            .find(|conversion| conversion.case == wanted)
            .map(|conversion| conversion.value)
            .unwrap_or_default()
    }

    #[test]
    fn words_split_at_separators_steps_and_acronyms() {
        assert_eq!(
            words("fooBar baz_qux-quux"),
            ["foo", "Bar", "baz", "qux", "quux"]
        );
        assert_eq!(words("HTTPServerError"), ["HTTP", "Server", "Error"]);
        assert_eq!(words("version2Beta"), ["version2", "Beta"]);
        assert_eq!(words("  été 2026 ! "), ["été", "2026"]);
        assert!(words("--- !").is_empty());
    }

    #[test]
    fn every_case_is_given_in_order() {
        let all = convert_case("parse HTTP response");
        let cases: Vec<TextCase> = all.iter().map(|conversion| conversion.case).collect();
        assert_eq!(
            cases,
            [
                TextCase::Camel,
                TextCase::Pascal,
                TextCase::Snake,
                TextCase::Kebab,
                TextCase::Constant,
                TextCase::Title,
                TextCase::Sentence,
                TextCase::Dot,
                TextCase::Path,
                TextCase::Train,
                TextCase::Lower,
                TextCase::Upper,
                TextCase::Flat,
            ]
        );
        let values: Vec<&str> = all
            .iter()
            .map(|conversion| conversion.value.as_str())
            .collect();
        assert_eq!(
            values,
            [
                "parseHttpResponse",
                "ParseHttpResponse",
                "parse_http_response",
                "parse-http-response",
                "PARSE_HTTP_RESPONSE",
                "Parse Http Response",
                "Parse http response",
                "parse.http.response",
                "parse/http/response",
                "Parse-Http-Response",
                "parse http response",
                "PARSE HTTP RESPONSE",
                "parsehttpresponse",
            ]
        );
    }

    #[test]
    fn the_new_cases_split_the_hard_words_as_the_others_do() {
        for (text, dot, train, flat) in [
            (
                "HTTPResponse",
                "http.response",
                "Http-Response",
                "httpresponse",
            ),
            (
                "parseHTTPResponse",
                "parse.http.response",
                "Parse-Http-Response",
                "parsehttpresponse",
            ),
            ("utf8Decoder", "utf8.decoder", "Utf8-Decoder", "utf8decoder"),
            ("v2Api", "v2.api", "V2-Api", "v2api"),
            ("élévation", "élévation", "Élévation", "élévation"),
            (
                "some_mixed-Case value",
                "some.mixed.case.value",
                "Some-Mixed-Case-Value",
                "somemixedcasevalue",
            ),
        ] {
            assert_eq!(case(text, TextCase::Dot), dot, "{text}");
            assert_eq!(case(text, TextCase::Train), train, "{text}");
            assert_eq!(case(text, TextCase::Flat), flat, "{text}");
        }
        assert_eq!(case("utf8Decoder", TextCase::Path), "utf8/decoder");
        assert_eq!(
            case("élévation rapide", TextCase::Upper),
            "ÉLÉVATION RAPIDE"
        );
        assert_eq!(case("Straße Nummer", TextCase::Lower), "straße nummer");
    }

    #[test]
    fn several_lines_are_converted_line_by_line() {
        assert_eq!(
            case(
                "userId
HTTPServer

  created_at  
",
                TextCase::Snake
            ),
            "user_id
http_server

created_at"
        );
        assert_eq!(
            case(
                "

fooBar

",
                TextCase::Kebab
            ),
            "foo-bar"
        );
        assert!(
            convert_case(
                "
 - 
"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_case_keeps_its_accents_and_its_long_capitals() {
        assert_eq!(case("déjà vu", TextCase::Pascal), "DéjàVu");
        assert_eq!(case("straße", TextCase::Constant), "STRASSE");
    }

    #[test]
    fn a_text_without_a_word_converts_to_nothing() {
        assert!(convert_case("  -_- ").is_empty());
    }

    fn slug(text: &str, separator: SlugSeparator, lowercase: bool) -> String {
        slugify(&SlugRequest {
            text: text.to_owned(),
            separator,
            lowercase,
        })
    }

    #[test]
    fn a_slug_transliterates_rather_than_drops() {
        assert_eq!(slug("Été 2026 !", SlugSeparator::Dash, true), "ete-2026");
        assert_eq!(
            slug("Straße & Œuvre", SlugSeparator::Dash, true),
            "strasse-oeuvre"
        );
        assert_eq!(slug("Привет мир", SlugSeparator::Dash, true), "privet-mir");
    }

    #[test]
    fn a_slug_takes_its_separator_and_may_keep_its_case() {
        assert_eq!(
            slug("  Hello, World  ", SlugSeparator::Underscore, false),
            "Hello_World"
        );
        assert_eq!(slug("a.b c", SlugSeparator::Dot, true), "a.b.c");
        assert_eq!(slug("!!!", SlugSeparator::Dash, true), "");
    }

    fn fix(
        text: &str,
        ending: Option<LineEnding>,
        trim: bool,
        last: FinalNewline,
    ) -> LineBreaksAnswer {
        fix_line_breaks(&LineBreaksRequest {
            text: text.to_owned(),
            ending,
            trim: if trim { Trim::End } else { Trim::Keep },
            empty_lines: EmptyLines::Keep,
            normalize: false,
            final_newline: last,
        })
    }

    fn tidy(text: &str, trim: Trim, empty_lines: EmptyLines, normalize: bool) -> LineBreaksAnswer {
        fix_line_breaks(&LineBreaksRequest {
            text: text.to_owned(),
            ending: None,
            trim,
            empty_lines,
            normalize,
            final_newline: FinalNewline::Keep,
        })
    }

    #[test]
    fn empty_lines_are_kept_collapsed_or_removed() {
        let text = "a\n\n  \n\t\nb\n\nc\n";

        assert_eq!(tidy(text, Trim::Keep, EmptyLines::Keep, false).text, text);
        let collapsed = tidy(text, Trim::Keep, EmptyLines::Collapse, false);
        assert_eq!(collapsed.text, "a\n\nb\n\nc\n");
        assert_eq!(collapsed.removed_lines, 2);
        let removed = tidy(text, Trim::Keep, EmptyLines::Remove, false);
        assert_eq!(removed.text, "a\nb\nc\n");
        assert_eq!(removed.removed_lines, 4);
    }

    #[test]
    fn a_text_of_blank_lines_empties_and_keeps_its_end() {
        assert_eq!(
            tidy(" \n\n\t\n", Trim::Keep, EmptyLines::Remove, false).text,
            ""
        );
        assert_eq!(
            tidy("\n\n", Trim::Both, EmptyLines::Collapse, false).text,
            "\n"
        );
    }

    #[test]
    fn both_ends_trimmed_or_the_end_alone() {
        let text = "  a  \n\tb\t";

        assert_eq!(
            tidy(text, Trim::End, EmptyLines::Keep, false).text,
            "  a\n\tb"
        );
        let both = tidy(text, Trim::Both, EmptyLines::Keep, false);
        assert_eq!(both.text, "a\nb");
        assert_eq!(both.trimmed, 2);
    }

    #[test]
    fn whitespace_is_normalised_inside_a_line_and_the_indentation_kept() {
        let answer = tidy(
            "  a  b\t\tc\u{a0}d\u{202f}e\u{200b}f\u{3000}g\u{2009}h",
            Trim::Keep,
            EmptyLines::Keep,
            true,
        );

        assert_eq!(answer.text, "  a b c d ef g h");
        assert_eq!(answer.normalized, 8);
        assert_eq!(
            tidy("\u{a0}\u{a0}x", Trim::Keep, EmptyLines::Keep, true).text,
            "  x"
        );
        assert_eq!(
            tidy("👨\u{200d}👩\u{200d}👧", Trim::Keep, EmptyLines::Keep, true).text,
            "👨\u{200d}👩\u{200d}👧",
            "a joiner holds an emoji together"
        );
        assert_eq!(
            tidy("\u{feff}x", Trim::Keep, EmptyLines::Keep, true).text,
            "x"
        );
    }

    #[test]
    fn every_option_at_once_keeps_crlf() {
        let answer = fix_line_breaks(&LineBreaksRequest {
            text: "a\u{a0}\u{a0}b  \r\n\r\n\r\n  c\r\n".into(),
            ending: None,
            trim: Trim::Both,
            empty_lines: EmptyLines::Collapse,
            normalize: true,
            final_newline: FinalNewline::Keep,
        });

        assert_eq!(answer.text, "a b\r\n\r\nc\r\n");
        assert_eq!(answer.found.crlf, 4);
        assert_eq!((answer.removed_lines, answer.trimmed), (1, 2));
    }

    #[test]
    fn line_breaks_are_counted_by_kind() {
        let answer = fix("a\r\nb\nc\rd\r\n", None, false, FinalNewline::Keep);

        assert_eq!(
            answer.found,
            EndingCounts {
                lf: 1,
                crlf: 2,
                cr: 1
            }
        );
        assert_eq!(answer.text, "a\r\nb\nc\rd\r\n");
        assert_eq!((answer.converted, answer.trimmed), (0, 0));
        assert_eq!(answer.final_newline, FinalNewlineChange::Unchanged);
    }

    #[test]
    fn line_breaks_convert_to_one_kind_and_count_what_moved() {
        let answer = fix(
            "a\r\nb\nc\r",
            Some(LineEnding::Lf),
            false,
            FinalNewline::Keep,
        );

        assert_eq!(answer.text, "a\nb\nc\n");
        assert_eq!(answer.converted, 2);
    }

    #[test]
    fn trailing_spaces_and_tabs_go_line_by_line() {
        let answer = fix("a  \nb\t\nc", None, true, FinalNewline::Keep);

        assert_eq!(answer.text, "a\nb\nc");
        assert_eq!(answer.trimmed, 2);
    }

    #[test]
    fn a_final_newline_is_added_in_the_text_own_kind() {
        let answer = fix("a\r\nb", None, false, FinalNewline::Add);

        assert_eq!(answer.text, "a\r\nb\r\n");
        assert_eq!(answer.final_newline, FinalNewlineChange::Added);
        assert_eq!(fix("a", None, false, FinalNewline::Add).text, "a\n");
        assert_eq!(
            fix("a\n", None, false, FinalNewline::Add).final_newline,
            FinalNewlineChange::Unchanged
        );
    }

    #[test]
    fn a_final_newline_is_removed_once() {
        let answer = fix("a\n\n", None, false, FinalNewline::Remove);

        assert_eq!(answer.text, "a\n");
        assert_eq!(answer.final_newline, FinalNewlineChange::Removed);
        assert_eq!(
            fix("a", None, false, FinalNewline::Remove).final_newline,
            FinalNewlineChange::Unchanged
        );
    }

    #[test]
    fn an_empty_text_stays_empty() {
        let answer = fix("", Some(LineEnding::Crlf), true, FinalNewline::Add);

        assert_eq!(answer.text, "");
        assert_eq!(answer.final_newline, FinalNewlineChange::Unchanged);
    }
}
