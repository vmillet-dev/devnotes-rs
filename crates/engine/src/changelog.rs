pub mod model;

use model::{ChangelogRelease, ChangelogSection};

/// Resolved from the manifest, for the same reason as `BINDINGS_PATH`. The one place
/// user-facing text comes out of Rust: it is data shipped as a file, untranslated like
/// the release notes the updater hands over.
const CHANGELOG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CHANGELOG.md"));

pub fn app_changelog() -> Vec<ChangelogRelease> {
    let mut releases = model::parse(CHANGELOG);
    for release in &mut releases {
        model::drop_what_is_not_in_the_app(&mut release.sections);
    }
    releases
}

/// The notes the updater hands over, read with the same grammar as the file.
pub fn release_notes(markdown: &str) -> Vec<ChangelogSection> {
    let mut sections = model::parse_section(markdown);
    model::drop_what_is_not_in_the_app(&mut sections);
    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::SpanKind;

    #[test]
    fn the_shipped_file_parses_into_something_to_show() {
        let releases = app_changelog();

        assert!(!releases.is_empty(), "the file describes no release");
        for release in &releases {
            assert!(
                !release.sections.is_empty(),
                "release {} carries no entry",
                release.version
            );
            for section in &release.sections {
                assert!(
                    !section.items.is_empty(),
                    "section {} of {} is an empty heading",
                    section.title,
                    release.version
                );
            }
        }
    }

    /// On the shipped file and not on a sample: the hand-written sections are the ones
    /// that carry emphasis, and they were printed with their markers on screen.
    #[test]
    fn the_shipped_file_carries_emphasis_that_is_read_rather_than_printed() {
        let spans: Vec<_> = app_changelog()
            .into_iter()
            .flat_map(|release| release.sections)
            .flat_map(|section| section.items)
            .flatten()
            .collect();

        assert!(
            spans.iter().any(|span| span.kind == SpanKind::Strong),
            "no entry in the file is emphasised any more — check the grammar still matches it"
        );
        assert!(
            spans.iter().any(|span| span.kind == SpanKind::Code),
            "no entry in the file carries code any more"
        );
        assert!(
            !spans.iter().any(|span| span.text.contains("**")),
            "a marker reached the panel as text"
        );
    }

    /// What `release.yml` hands the updater is the newest section of this very file.
    #[test]
    fn the_newest_section_reads_the_same_alone_as_in_the_file() {
        let start = CHANGELOG.find("\n## ").expect("a release heading") + 1;
        let body_start = CHANGELOG[start..]
            .find('\n')
            .map_or(CHANGELOG.len(), |at| start + at + 1);
        let body_end = CHANGELOG[body_start..]
            .find("\n## ")
            .map_or(CHANGELOG.len(), |at| body_start + at);

        assert_eq!(
            release_notes(&CHANGELOG[body_start..body_end]),
            app_changelog().remove(0).sections
        );
    }

    #[test]
    fn the_shipped_file_shows_neither_the_site_nor_its_pages() {
        for heading in model::NOT_IN_THE_APP {
            assert!(
                CHANGELOG.contains(&format!("### {heading}\n")),
                "{heading} is in no release"
            );
        }
        let shown: Vec<_> = app_changelog()
            .into_iter()
            .flat_map(|release| release.sections)
            .map(|section| section.title)
            .collect();

        assert!(
            !shown
                .iter()
                .any(|title| model::NOT_IN_THE_APP.contains(&title.as_str()))
        );
    }

    /// Written by the script, recognised here: a heading renamed on one side alone would reach
    /// the panel.
    #[test]
    fn the_headings_left_out_are_the_ones_the_release_script_writes() {
        let script = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/release-notes.mjs"
        ));

        for heading in model::NOT_IN_THE_APP {
            assert!(
                script.contains(&format!("heading: '{heading}'")),
                "{heading}"
            );
        }
    }

    /// The number is what makes the release on github.com navigable, and noise on a panel
    /// with no links in it.
    #[test]
    fn no_entry_reaches_the_panel_with_a_pull_request_number_on_it() {
        for release in app_changelog() {
            for section in release.sections {
                for entry in section.items {
                    let read: String = entry.iter().map(|span| span.text.as_str()).collect();
                    assert!(
                        !read.trim_end().ends_with(')') || !read.contains("(#"),
                        "{read:?} still names a pull request"
                    );
                }
            }
        }
    }
}
