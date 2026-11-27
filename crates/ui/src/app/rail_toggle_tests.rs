//! The ends of the bars: the button that puts the sidebar away stands at one place, whichever area
//! or page is shown, and the HTTP strip ends where the request under it ends.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use engine::http::model::{HttpMethod, RequestKind};

use super::DevNotesApp;
use super::tests::{click, open_library, settle};

/// The toggle's box, found by its address, in the window's points.
fn toggle(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .rect()
}

/// Every place the toggle is drawn, at the density chosen: where it stood, page by page.
fn places(harness: &mut Harness<'static, DevNotesApp>) -> Vec<(&'static str, egui::Rect)> {
    let mut seen = Vec::new();
    click(harness, "area-option-notes");
    seen.push(("notes", toggle(harness, "library-rail-toggle")));
    click(harness, "area-option-tools");
    seen.push(("tools home", toggle(harness, "tools-rail-toggle")));
    click(harness, "tools-entry-convert");
    seen.push(("a tool", toggle(harness, "tools-rail-toggle")));
    click(harness, "tool-back");
    click(harness, "area-option-http");
    seen.push(("request tabs", toggle(harness, "http-rail-toggle")));
    click(harness, "http-environments-open");
    seen.push(("environments", toggle(harness, "http-rail-toggle")));
    click(harness, "http-history-open");
    seen.push(("history", toggle(harness, "http-rail-toggle")));
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-menu")
                && node.label().is_some_and(|label| label.ends_with("Local"))
        })
        .click();
    settle(harness);
    click(harness, "http-node-run");
    seen.push(("runner", toggle(harness, "http-rail-toggle")));
    seen
}

#[test]
fn the_sidebar_toggle_stands_at_one_place_in_every_area_at_both_densities() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    let backend = harness.state().backend();
    let local = crate::http::tests::collection(backend, "Local");
    crate::http::tests::request(
        backend,
        (&local, None),
        "Login",
        (RequestKind::Http, HttpMethod::Get),
    );
    for density in [
        crate::theme::Density::Comfortable,
        crate::theme::Density::Compact,
    ] {
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        app.state.settings.density = density;
        app.state.apply_look(&ctx);
        settle(&mut harness);
        let seen = places(&mut harness);
        let (_, first) = seen[0];
        for (page, rect) in &seen {
            assert!(
                (rect.min - first.min).length() < 0.5
                    && (rect.size() - first.size()).length() < 0.5,
                "{density:?}: {page} at {rect:?}, the notes at {first:?}\n{seen:?}"
            );
        }
    }
}

#[test]
fn the_environment_picker_ends_at_the_margin_of_the_request_line() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    let backend = harness.state().backend();
    let local = crate::http::tests::collection(backend, "Local");
    crate::http::tests::request(
        backend,
        (&local, None),
        "Login",
        (RequestKind::Http, HttpMethod::Post),
    );
    click(&mut harness, "area-option-http");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().as_deref() == Some("POST Login")
        })
        .click();
    let margin = harness.ctx.content_rect().right() - crate::theme::tokens::BAR_INSET;
    for density in [
        crate::theme::Density::Comfortable,
        crate::theme::Density::Compact,
    ] {
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        app.state.settings.density = density;
        app.state.apply_look(&ctx);
        settle(&mut harness);
        let picker = toggle(&harness, "http-environment-picker").right();
        let save = toggle(&harness, "http-save").right();
        // « Save » ends where the measured widths of the line leave it: a point or two short.
        assert!(
            (picker - margin).abs() < 0.5 && save <= margin && margin - save < 3.0,
            "{density:?}: the picker ends at {picker}, Save at {save}, the margin at {margin}"
        );
    }
}
