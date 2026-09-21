pub mod model;

use model::ChangelogRelease;

/// Resolved from the manifest, for the same reason as `BINDINGS_PATH`. ⚠️ The one place
/// user-facing text comes out of Rust: it is data shipped as a file, untranslated like
/// the release notes the updater hands over.
const CHANGELOG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../CHANGELOG.md"));

#[tauri::command(async)]
#[specta::specta]
pub fn app_changelog() -> Vec<ChangelogRelease> {
    model::parse(CHANGELOG)
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

    /// ⚠️ On the shipped file and not on a sample: the hand-written sections are the ones
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
