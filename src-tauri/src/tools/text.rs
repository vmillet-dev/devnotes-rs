//! The Texte panel: the case converter, the slug generator and the line breaks.

use serde::{Deserialize, Serialize};
use specta::Type;
use unicode_normalization::UnicodeNormalization;

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

/// What a case is for. The code cases strip accents when asked; the text cases keep them, and
/// keep the apostrophe that binds an elided word to the next (`l'été`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CaseGroup {
    Code,
    Text,
}

impl TextCase {
    fn group(self) -> CaseGroup {
        match self {
            Self::Title | Self::Sentence | Self::Upper | Self::Lower => CaseGroup::Text,
            _ => CaseGroup::Code,
        }
    }
}

/** In the order the tool lists them: the code cases, then the text ones. */
const CASES: [TextCase; 13] = [
    TextCase::Camel,
    TextCase::Pascal,
    TextCase::Snake,
    TextCase::Constant,
    TextCase::Kebab,
    TextCase::Train,
    TextCase::Dot,
    TextCase::Path,
    TextCase::Flat,
    TextCase::Title,
    TextCase::Sentence,
    TextCase::Upper,
    TextCase::Lower,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TitleLanguage {
    /// Every word capitalised.
    English,
    /// The small words stay in lower case, but at the start.
    French,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaseRequest {
    pub text: String,
    /// The code cases only: `été` is `ete` in `snake_case` and stays `été` in a title.
    pub strip_accents: bool,
    /// Each line on its own; off, the whole text is one phrase.
    pub per_line: bool,
    pub title_case_language: TitleLanguage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaseConversion {
    pub case: TextCase,
    pub group: CaseGroup,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaseAnswer {
    /// Empty for a text without a word.
    pub conversions: Vec<CaseConversion>,
    /// What the converters work from: the first line holding a word, or the whole phrase.
    pub words: Vec<String>,
}

const APOSTROPHES: [char; 2] = ['\'', '’'];

/// French words a title leaves in lower case, the elided `l'` and `d'` included.
const FRENCH_SMALL_WORDS: [&str; 26] = [
    "de", "du", "des", "la", "le", "les", "l", "d", "un", "une", "et", "ou", "à", "au", "aux",
    "en", "sur", "sous", "par", "pour", "dans", "avec", "sans", "ni", "chez", "vers",
];

/// What elides into the word after it in French: `l'été`, `qu'il`. The word after one of these
/// takes a capital in a French title; the second half of `aujourd'hui` or `don't` never does.
const FRENCH_ELISIONS: [&str; 12] = [
    "l", "d", "j", "m", "n", "s", "t", "c", "qu", "jusqu", "lorsqu", "puisqu",
];

/// A word, and the apostrophe that binds it to the one before it, if one does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Word<'a> {
    text: &'a str,
    bound_by: Option<char>,
}

/// Words as a reader sees them: apart at anything but a letter or a digit, at a lower case or a
/// digit followed by a capital (`fooBar`, `v2Beta`), and at the end of an acronym (`HTTPServer`
/// is `HTTP` and `Server`). A digit stays with what it follows. An apostrophe between two letters
/// parts them too, and is remembered on the second.
fn words(text: &str) -> Vec<Word<'_>> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    let mut previous: Option<char> = None;
    // Of the word being read, and of the one about to start.
    let mut bond: Option<char> = None;
    let mut pending: Option<char> = None;
    let mut chars = text.char_indices().peekable();

    while let Some((index, current)) = chars.next() {
        if !current.is_alphanumeric() {
            let next_is_word = chars
                .peek()
                .is_some_and(|&(_, next)| next.is_alphanumeric());
            pending = None;
            if let Some(begun) = start.take() {
                words.push(Word {
                    text: &text[begun..index],
                    bound_by: bond.take(),
                });
                pending = (APOSTROPHES.contains(&current) && next_is_word).then_some(current);
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
        match start {
            None => {
                start = Some(index);
                bond = pending.take();
            }
            Some(begun) if boundary => {
                words.push(Word {
                    text: &text[begun..index],
                    bound_by: bond.take(),
                });
                start = Some(index);
            }
            Some(_) => {}
        }
        previous = Some(current);
    }

    if let Some(begun) = start {
        words.push(Word {
            text: &text[begun..],
            bound_by: bond,
        });
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

/// Accents only: `é` is `e`, but `ß` and `ø` are letters of their own and stay.
fn without_accents(word: &str) -> String {
    word.nfd()
        .filter(|character| !unicode_normalization::char::is_combining_mark(*character))
        .nfc()
        .collect()
}

fn code_case(words: &[String], case: TextCase) -> String {
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
        TextCase::Dot => joined(lower().collect(), "."),
        TextCase::Path => joined(lower().collect(), "/"),
        TextCase::Train => joined(capital().collect(), "-"),
        TextCase::Flat => lower().collect(),
        TextCase::Title | TextCase::Sentence | TextCase::Lower | TextCase::Upper => {
            unreachable!("a text case is written by text_case")
        }
    }
}

fn text_case(words: &[Word<'_>], case: TextCase, language: TitleLanguage) -> String {
    let french = language == TitleLanguage::French;
    let mut written = String::new();
    let mut previous: Option<String> = None;
    for (at, word) in words.iter().enumerate() {
        let lower = word.text.to_lowercase();
        let small = french && at > 0 && FRENCH_SMALL_WORDS.contains(&lower.as_str());
        let after_elision = french
            && previous
                .as_deref()
                .is_some_and(|before| FRENCH_ELISIONS.contains(&before));
        let part = match case {
            TextCase::Title if word.bound_by.is_some() && at > 0 && !after_elision => lower.clone(),
            TextCase::Title if small => lower.clone(),
            TextCase::Title => capitalised(word.text),
            TextCase::Sentence if at == 0 => capitalised(word.text),
            TextCase::Upper => word.text.to_uppercase(),
            _ => lower.clone(),
        };
        match word.bound_by {
            Some(apostrophe) if at > 0 => written.push(apostrophe),
            _ if at > 0 => written.push(' '),
            _ => {}
        }
        written.push_str(&part);
        previous = Some(lower);
    }
    written
}

fn in_case(words: &[Word<'_>], case: TextCase, request: &CaseRequest) -> String {
    match case.group() {
        CaseGroup::Text => text_case(words, case, request.title_case_language),
        CaseGroup::Code => {
            let parts: Vec<String> = words
                .iter()
                .map(|word| {
                    if request.strip_accents {
                        without_accents(word.text)
                    } else {
                        word.text.to_owned()
                    }
                })
                .collect();
            code_case(&parts, case)
        }
    }
}

/// Every case at once, line by line — a list of identifiers comes back as a list; blank lines at
/// either end are dropped, those inside kept so each line keeps its place. Nothing for no word.
pub fn convert_case(request: &CaseRequest) -> CaseAnswer {
    let lines: Vec<Vec<Word<'_>>> = if request.per_line {
        request.text.lines().map(words).collect()
    } else {
        vec![words(&request.text)]
    };
    let (Some(first), Some(last)) = (
        lines.iter().position(|words| !words.is_empty()),
        lines.iter().rposition(|words| !words.is_empty()),
    ) else {
        return CaseAnswer {
            conversions: Vec::new(),
            words: Vec::new(),
        };
    };
    let conversions = CASES
        .into_iter()
        .map(|case| CaseConversion {
            case,
            group: case.group(),
            value: lines[first..=last]
                .iter()
                .map(|words| in_case(words, case, request))
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect();
    CaseAnswer {
        conversions,
        words: lines[first]
            .iter()
            .map(|word| word.text.to_owned())
            .collect(),
    }
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

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineBreaksRequest {
    pub text: String,
    /// `None` keeps each line's own ending.
    pub ending: Option<LineEnding>,
    pub trim_trailing: bool,
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
    /// Lines that lost spaces or tabs at their end.
    pub trimmed: u32,
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

/// Which endings the text has, converted to one if asked, trailing spaces stripped if asked, and
/// the final newline added or removed — each change counted.
pub fn fix_line_breaks(request: &LineBreaksRequest) -> LineBreaksAnswer {
    let lines = lines(&request.text);
    let mut found = EndingCounts::default();
    let (mut converted, mut trimmed) = (0usize, 0usize);
    let mut rewritten: Vec<(String, Option<LineEnding>)> = Vec::with_capacity(lines.len());

    for (content, ending) in lines {
        match ending {
            Some(LineEnding::Lf) => found.lf += 1,
            Some(LineEnding::Crlf) => found.crlf += 1,
            Some(LineEnding::Cr) => found.cr += 1,
            None => {}
        }
        let kept = if request.trim_trailing {
            content.trim_end_matches([' ', '\t'])
        } else {
            content
        };
        if kept.len() != content.len() {
            trimmed += 1;
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
        trimmed: saturating_u32(trimmed),
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

    fn request(text: &str) -> CaseRequest {
        CaseRequest {
            text: text.to_owned(),
            strip_accents: false,
            per_line: true,
            title_case_language: TitleLanguage::English,
        }
    }

    fn case_of(request: &CaseRequest, wanted: TextCase) -> String {
        convert_case(request)
            .conversions
            .into_iter()
            .find(|conversion| conversion.case == wanted)
            .map(|conversion| conversion.value)
            .unwrap_or_default()
    }

    fn case(text: &str, wanted: TextCase) -> String {
        case_of(&request(text), wanted)
    }

    fn texts<'a>(words: &[Word<'a>]) -> Vec<&'a str> {
        words.iter().map(|word| word.text).collect()
    }

    #[test]
    fn words_split_at_separators_steps_and_acronyms() {
        assert_eq!(
            texts(&words("fooBar baz_qux-quux")),
            ["foo", "Bar", "baz", "qux", "quux"]
        );
        assert_eq!(
            texts(&words("HTTPServerError")),
            ["HTTP", "Server", "Error"]
        );
        assert_eq!(texts(&words("version2Beta")), ["version2", "Beta"]);
        assert_eq!(texts(&words("  été 2026 ! ")), ["été", "2026"]);
        assert!(words("--- !").is_empty());
    }

    #[test]
    fn every_case_is_given_in_order() {
        let all = convert_case(&request("parse HTTP response")).conversions;
        let cases: Vec<TextCase> = all.iter().map(|conversion| conversion.case).collect();
        assert_eq!(
            cases,
            [
                TextCase::Camel,
                TextCase::Pascal,
                TextCase::Snake,
                TextCase::Constant,
                TextCase::Kebab,
                TextCase::Train,
                TextCase::Dot,
                TextCase::Path,
                TextCase::Flat,
                TextCase::Title,
                TextCase::Sentence,
                TextCase::Upper,
                TextCase::Lower,
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
                "PARSE_HTTP_RESPONSE",
                "parse-http-response",
                "Parse-Http-Response",
                "parse.http.response",
                "parse/http/response",
                "parsehttpresponse",
                "Parse Http Response",
                "Parse http response",
                "PARSE HTTP RESPONSE",
                "parse http response",
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
            convert_case(&request(
                "
 -
"
            ))
            .conversions
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
        let answer = convert_case(&request("  -_- "));
        assert!(answer.conversions.is_empty());
        assert!(answer.words.is_empty());
    }

    mod with_options {
        use super::*;

        fn with(text: &str, edit: impl FnOnce(&mut CaseRequest)) -> CaseRequest {
            let mut asked = request(text);
            asked.strip_accents = true;
            edit(&mut asked);
            asked
        }

        #[test]
        fn the_code_cases_strip_accents_and_the_text_cases_keep_them() {
            let asked = with("Été à Noël", |_| {});
            assert_eq!(case_of(&asked, TextCase::Snake), "ete_a_noel");
            assert_eq!(case_of(&asked, TextCase::Camel), "eteANoel");
            assert_eq!(case_of(&asked, TextCase::Title), "Été À Noël");
            assert_eq!(case_of(&asked, TextCase::Upper), "ÉTÉ À NOËL");
        }

        /// Accents only: a letter of its own is no accent, and the slug is what transliterates.
        #[test]
        fn stripping_accents_leaves_other_letters_alone() {
            let asked = with("Straße Øre", |_| {});
            assert_eq!(case_of(&asked, TextCase::Snake), "straße_øre");
            assert_eq!(case_of(&asked, TextCase::Constant), "STRASSE_ØRE");
        }

        #[test]
        fn accents_stay_in_the_code_cases_when_asked_to() {
            let asked = with("déjà vu", |asked| asked.strip_accents = false);
            assert_eq!(case_of(&asked, TextCase::Pascal), "DéjàVu");
        }

        #[test]
        fn every_conversion_says_which_group_it_belongs_to() {
            let answer = convert_case(&with("a b", |_| {}));
            let groups: Vec<CaseGroup> = answer.conversions.iter().map(|c| c.group).collect();
            assert_eq!(groups[..9], [CaseGroup::Code; 9]);
            assert_eq!(groups[9..], [CaseGroup::Text; 4]);
        }

        #[test]
        fn a_french_title_keeps_its_small_words_in_lower_case_but_the_first() {
            let asked = with("le guide de la mer et des îles", |asked| {
                asked.title_case_language = TitleLanguage::French;
            });
            assert_eq!(
                case_of(&asked, TextCase::Title),
                "Le Guide de la Mer et des Îles"
            );

            let english = with("le guide de la mer", |_| {});
            assert_eq!(case_of(&english, TextCase::Title), "Le Guide De La Mer");
        }

        #[test]
        fn an_apostrophe_is_kept_in_the_text_cases_and_parts_the_code_ones() {
            let french = with("l'été de l’année", |asked| {
                asked.title_case_language = TitleLanguage::French;
            });
            assert_eq!(case_of(&french, TextCase::Title), "L'Été de l’Année");
            assert_eq!(case_of(&french, TextCase::Sentence), "L'été de l’année");
            assert_eq!(case_of(&french, TextCase::Snake), "l_ete_de_l_annee");

            let english = with("don't stop aujourd'hui", |_| {});
            assert_eq!(case_of(&english, TextCase::Title), "Don't Stop Aujourd'hui");
            assert_eq!(case_of(&english, TextCase::Upper), "DON'T STOP AUJOURD'HUI");
        }

        #[test]
        fn one_phrase_when_the_lines_are_not_converted_apart() {
            let asked = with("user id\nand name", |asked| asked.per_line = false);
            assert_eq!(case_of(&asked, TextCase::Camel), "userIdAndName");
            assert_eq!(convert_case(&asked).words, ["user", "id", "and", "name"]);

            let lines = with("user id\nand name", |_| {});
            assert_eq!(case_of(&lines, TextCase::Camel), "userId\nandName");
        }

        #[test]
        fn the_words_are_those_of_the_first_line_holding_one() {
            let answer = convert_case(&with("\n  \nparseURL et HTTPServer\nnext", |_| {}));
            assert_eq!(answer.words, ["parse", "URL", "et", "HTTP", "Server"]);
        }
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
            trim_trailing: trim,
            final_newline: last,
        })
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
