//! ⚠️ The grammar is thin and the shipped `CHANGELOG.md` is written to match it: `## `
//! opens a release, `### ` a category, `- ` an entry, and an indented line continues the
//! one above. Reshaping the file past that empties "Nouveautés".

use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogRelease {
    /// `0.1.1`, or whatever the heading names. Square brackets are dropped.
    pub version: String,
    /// `None` when the heading carries no date; never invented.
    pub date: Option<String>,
    pub sections: Vec<ChangelogSection>,
}

/// One `### ` heading, or the anonymous one a release with no category gets.       
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogSection {
    /// Empty for that anonymous category: the front then renders no heading.
    pub title: String,
    pub items: Vec<ChangelogEntry>,
}

/// One bullet, cut into the runs it is drawn with.
///
/// Runs and not a line of markdown: the panel renders them with a `switch` and three
/// spans, so nothing has to be trusted, sanitised or handed to `innerHTML`.
pub type ChangelogEntry = Vec<ChangelogSpan>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SpanKind {
    Plain,
    Strong,
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogSpan {
    pub kind: SpanKind,
    pub text: String,
}

impl ChangelogSpan {
    fn of(kind: SpanKind, text: String) -> Self {
        Self { kind, text }
    }

    fn plain(text: String) -> Self {
        Self::of(SpanKind::Plain, text)
    }

    fn strong(text: String) -> Self {
        Self::of(SpanKind::Strong, text)
    }

    fn code(text: String) -> Self {
        Self::of(SpanKind::Code, text)
    }
}

/// The order of the file, never re-sorted: versions are strings here, and sorting them
/// as such would put `0.10` before `0.9`.
pub fn parse(markdown: &str) -> Vec<ChangelogRelease> {
    let mut releases: Vec<ChangelogRelease> = Vec::new();
    // A continuation line belongs to the entry above it, and only while one is open.
    let mut item_open = false;

    for line in markdown.lines() {
        let line = line.trim();

        if let Some(heading) = line.strip_prefix("## ") {
            let (version, date) = split_heading(heading);
            releases.push(ChangelogRelease {
                version,
                date,
                sections: Vec::new(),
            });
            item_open = false;
            continue;
        }

        // Nothing to attach to: the preamble sits above the first release.
        let Some(release) = releases.last_mut() else {
            continue;
        };

        if let Some(title) = line.strip_prefix("### ") {
            release.sections.push(ChangelogSection {
                title: title.trim().to_string(),
                items: Vec::new(),
            });
            item_open = false;
        } else if let Some(entry) = bullet(line) {
            section_of(release).items.push(spans(entry));
            item_open = true;
        } else if line.is_empty() {
            item_open = false;
        } else if item_open
            && let Some(item) = release.sections.last_mut().and_then(|s| s.items.last_mut())
        {
            // Re-cut with what it continues rather than appended to it: a marker opened
            // on the line above closes on this one.
            *item = spans(&format!("{} {line}", written(item)));
        }
    }

    releases
}

fn section_of(release: &mut ChangelogRelease) -> &mut ChangelogSection {
    if release.sections.is_empty() {
        release.sections.push(ChangelogSection {
            title: String::new(),
            items: Vec::new(),
        });
    }

    release
        .sections
        .last_mut()
        .expect("a section was just pushed if there was none")
}

/// The markdown an entry was cut from, so a continuation line can be re-cut with it.
fn written(entry: &ChangelogEntry) -> String {
    entry
        .iter()
        .map(|span| match span.kind {
            SpanKind::Plain => span.text.clone(),
            SpanKind::Strong => format!("**{}**", span.text),
            SpanKind::Code => format!("`{}`", span.text),
        })
        .collect()
}

/// The trailing `(#123)` a squashed pull request leaves behind.
///
/// Dropped when the entry is read and not when the file is written: the number is what
/// makes the release on github.com navigable, and noise on a panel with no links in it.
fn without_pull_request(entry: &str) -> &str {
    let Some(open) = entry.strip_suffix(')').and_then(|rest| rest.rfind("(#")) else {
        return entry;
    };

    let number = &entry[open + 2..entry.len() - 1];
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return entry;
    }

    entry[..open].trim_end()
}

/// Cuts a bullet into plain, strong and code runs.
///
/// The grammar is two markers, paired, on one line, and nothing else — no links, no
/// nesting, no headings. An unclosed marker is text, which is what keeps an entry about
/// `5 * 3` readable.
fn spans(entry: &str) -> ChangelogEntry {
    let entry = without_pull_request(entry);
    let mut cut: ChangelogEntry = Vec::new();
    let mut plain = String::new();
    let mut rest = entry;

    while let Some(at) = rest.find(['*', '`']) {
        let opening = &rest[at..];
        let (marker, wrap): (&str, fn(String) -> ChangelogSpan) = if opening.starts_with("**") {
            ("**", ChangelogSpan::strong)
        } else if opening.starts_with('`') {
            ("`", ChangelogSpan::code)
        } else {
            // A lone `*`: nothing opens here.
            plain.push_str(&rest[..=at]);
            rest = &rest[at + 1..];
            continue;
        };

        let after = &rest[at + marker.len()..];
        let Some(end) = after.find(marker).filter(|end| *end > 0) else {
            // Nothing closes it: the marker is text, and so is what follows.
            plain.push_str(&rest[..at + marker.len()]);
            rest = after;
            continue;
        };

        plain.push_str(&rest[..at]);
        if !plain.is_empty() {
            cut.push(ChangelogSpan::plain(std::mem::take(&mut plain)));
        }
        cut.push(wrap(after[..end].to_string()));
        rest = &after[end + marker.len()..];
    }

    plain.push_str(rest);
    if !plain.is_empty() {
        cut.push(ChangelogSpan::plain(plain));
    }
    cut
}

/// `- entry` or `* entry`, stripped of its marker.
fn bullet(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
}

/// `[0.1.1] - 2026-08-27` → `("0.1.1", Some("2026-08-27"))`. The separator is looked for
/// after the version, so `1.0.0-rc.1` never yields a date.
fn split_heading(heading: &str) -> (String, Option<String>) {
    let heading = heading.trim();
    let (version, rest) = match heading.strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some((version, rest)) => (version, rest),
            None => (bracketed, ""),
        },
        None => match heading.split_once(" - ") {
            Some((version, rest)) => (version, rest),
            None => (heading, ""),
        },
    };

    let date = rest
        .trim()
        .trim_start_matches(['-', '–', '—'])
        .trim()
        .to_string();

    (
        version.trim().to_string(),
        (!date.is_empty()).then_some(date),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What an entry reads as once it is drawn, markers and all dropped.
    fn read(entry: &ChangelogEntry) -> String {
        entry.iter().map(|span| span.text.as_str()).collect()
    }

    fn reads(section: &ChangelogSection) -> Vec<String> {
        section.items.iter().map(read).collect()
    }

    const SAMPLE: &str = "\
# Changelog

Everything above the first release is preamble.

## [0.2.0] - 2026-09-11

### Added

- A todo list is a note too,
  and it wraps onto a second line.
- Sample notes on first launch.

### Fixed

- A crash nobody saw.

## 0.1.0

- Shipped at last.
";

    #[test]
    fn reads_a_release_with_its_date_and_categories() {
        let releases = parse(SAMPLE);

        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].version, "0.2.0");
        assert_eq!(releases[0].date.as_deref(), Some("2026-09-11"));
        assert_eq!(
            releases[0]
                .sections
                .iter()
                .map(|section| section.title.as_str())
                .collect::<Vec<_>>(),
            ["Added", "Fixed"]
        );
    }

    #[test]
    fn joins_a_wrapped_entry_rather_than_dropping_its_tail() {
        let releases = parse(SAMPLE);

        assert_eq!(
            read(&releases[0].sections[0].items[0]),
            "A todo list is a note too, and it wraps onto a second line."
        );
    }

    #[test]
    fn gives_an_anonymous_category_to_entries_listed_without_one() {
        let releases = parse(SAMPLE);

        assert_eq!(releases[1].version, "0.1.0");
        assert_eq!(releases[1].date, None);
        assert_eq!(releases[1].sections.len(), 1);
        assert_eq!(releases[1].sections[0].title, "");
        assert_eq!(reads(&releases[1].sections[0]), ["Shipped at last."]);
    }

    #[test]
    fn ignores_what_sits_above_the_first_release() {
        let releases = parse("# Changelog\n\n- Read me first.\n\n## 0.1.0\n\n- Shipped.\n");

        assert_eq!(releases.len(), 1);
        assert_eq!(reads(&releases[0].sections[0]), ["Shipped."]);
    }

    #[test]
    fn keeps_a_dash_inside_a_version_out_of_the_date() {
        assert_eq!(
            split_heading("1.0.0-rc.1"),
            ("1.0.0-rc.1".to_string(), None)
        );
        assert_eq!(
            split_heading("[1.0.0-rc.1] - 2026-01-01"),
            ("1.0.0-rc.1".to_string(), Some("2026-01-01".to_string()))
        );
    }

    #[test]
    fn reads_a_heading_that_names_no_version_number() {
        let releases = parse("## Unreleased\n\n- Not out yet.\n");

        assert_eq!(releases[0].version, "Unreleased");
        assert_eq!(releases[0].date, None);
    }

    #[test]
    fn a_blank_line_closes_an_entry_rather_than_gluing_the_next_paragraph() {
        let releases = parse("## 0.1.0\n\n- Shipped.\n\nA loose paragraph.\n");

        assert_eq!(reads(&releases[0].sections[0]), ["Shipped."]);
    }

    /// The hand-written sections open on `**Todo-list notes.**` and carry backticks, which
    /// must reach the screen as emphasis and code, not punctuation.
    mod how_an_entry_is_drawn {
        use super::*;

        fn only(markdown: &str) -> ChangelogEntry {
            parse(&format!("## 0.1.0\n\n- {markdown}\n"))
                .remove(0)
                .sections[0]
                .items
                .remove(0)
        }

        #[test]
        fn a_bold_run_is_a_run_of_its_own() {
            assert_eq!(
                only("**Todo lists.** A note is now either kind."),
                vec![
                    ChangelogSpan::strong("Todo lists.".to_string()),
                    ChangelogSpan::plain(" A note is now either kind.".to_string()),
                ]
            );
        }

        #[test]
        fn a_code_run_is_one_too() {
            assert_eq!(
                only("Fill a `{{field}}` before copying"),
                vec![
                    ChangelogSpan::plain("Fill a ".to_string()),
                    ChangelogSpan::code("{{field}}".to_string()),
                    ChangelogSpan::plain(" before copying".to_string()),
                ]
            );
        }

        /// A squashed pull request leaves it behind, and the panel has no links in it.
        #[test]
        fn the_pull_request_number_is_dropped() {
            assert_eq!(
                only("Let the library rail go down to 160px (#213)"),
                vec![ChangelogSpan::plain(
                    "Let the library rail go down to 160px".to_string()
                )]
            );
        }

        #[test]
        fn something_that_only_looks_like_a_number_is_left_alone() {
            assert_eq!(
                read(&only("Read the guide (#start here)")),
                "Read the guide (#start here)"
            );
            assert_eq!(
                read(&only("Nothing in brackets (#)")),
                "Nothing in brackets (#)"
            );
        }

        /// Unclosed is text: an entry about `5 * 3` has to stay readable.
        #[test]
        fn a_lone_marker_is_text() {
            assert_eq!(
                only("5 * 3 and a ** that closes nothing"),
                vec![ChangelogSpan::plain(
                    "5 * 3 and a ** that closes nothing".to_string()
                )]
            );
        }

        #[test]
        fn empty_emphasis_is_text_as_well() {
            assert_eq!(read(&only("Nothing to see ****")), "Nothing to see ****");
        }

        /// A continuation is re-cut with what it continues, or a marker opened on the
        /// first line would never find its close.
        #[test]
        fn a_marker_survives_a_wrapped_line() {
            let releases = parse("## 0.1.0\n\n- **A title\n  that wraps.** And the rest.\n");

            assert_eq!(
                releases[0].sections[0].items[0],
                vec![
                    ChangelogSpan::strong("A title that wraps.".to_string()),
                    ChangelogSpan::plain(" And the rest.".to_string()),
                ]
            );
        }
    }
}
