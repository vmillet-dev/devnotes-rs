use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::*;
use crate::backend::Backend;
use crate::gate::IndexFill;
use crate::testing;

pub(super) struct Bench {
    pub(super) backend: Backend,
    pub(super) state: AppState,
    pub(super) actions: Actions,
    pub(super) canvas: CanvasView,
    _profile: tempfile::TempDir,
}

/// The canvas alone over a real core, the view asked as the application asks it.
pub(super) fn canvas(backend: Backend, profile: tempfile::TempDir) -> Harness<'static, Bench> {
    let mut state = AppState::new(crate::services::preferences::Preferences::default(), true);
    state.language = i18n::Language::En;
    let mut harness = testing::harness()
        .with_size(vec2(1_100.0, 700.0))
        .build_ui_state(
            |ui, bench: &mut Bench| {
                crate::theme::motion::set_reduced(ui.ctx(), true);
                let now = chrono::Utc::now();
                let state = &mut bench.state;
                state.notes.update(&mut state.revision, &mut state.errors);
                state.caches.thumbnails.update(ui.ctx(), &bench.backend);
                state.notes_query.update(
                    ui.ctx(),
                    &bench.backend,
                    now,
                    (None, None),
                    (state.revision, IndexFill::Ready),
                );
                show(ui, &bench.state, &mut bench.canvas, &mut bench.actions);
                let ctx = ui.ctx().clone();
                for action in bench.actions.drain().collect::<Vec<_>>() {
                    bench.state.apply(action, &ctx, &bench.backend);
                }
            },
            Bench {
                backend,
                state,
                actions: Actions::default(),
                canvas: CanvasView::default(),
                _profile: profile,
            },
        );
    testing::settle(&mut harness, |bench| &bench.backend);
    harness
}

fn cards(harness: &Harness<'_, Bench>) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .count()
}

#[test]
fn the_notes_of_today_are_one_section_of_cards() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    testing::note(&backend, "curl health");
    let harness = canvas(backend, profile);

    assert_eq!(cards(&harness), 2);
    let titles: Vec<_> = harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .map(|card| card.accesskit_node().label())
        .collect();
    assert!(
        titles.contains(&Some("psql prod".to_string())),
        "{titles:?}"
    );
    let headings: Vec<_> = harness
        .query_all_by(|node| node.author_id() == Some("note-section"))
        .map(|heading| heading.accesskit_node().label().unwrap_or_default())
        .collect();
    // Exhaustive: an empty section is still a heading.
    assert_eq!(headings, ["TODAY · 2", "THIS WEEK · 0"]);
}

#[test]
fn of_thirty_thousand_notes_only_the_rows_in_view_are_drawn() {
    let (backend, profile) = testing::unlocked();
    testing::corpus(&backend, 30_000);
    let mut harness = canvas(backend, profile);
    let first = cards(&harness);

    harness
        .get_by(|node| node.author_id() == Some("canvas"))
        .scroll_down();
    testing::settle(&mut harness, |bench| &bench.backend);

    // Four columns at this width, four rows and a part in 700 px.
    assert!((4..=24).contains(&first), "{first} cards drawn");
    assert!(cards(&harness) <= 24);
    assert_eq!(
        harness.state().state.notes_query.view().unwrap().matched,
        30_000
    );
}

#[test]
fn a_library_without_notes_says_nothing_matches() {
    let (backend, profile) = testing::unlocked();

    let harness = canvas(backend, profile);

    assert_eq!(cards(&harness), 0);
    harness.get_by(|node| node.author_id() == Some("canvas-no-results"));
}

#[test]
fn a_view_that_failed_offers_to_retry() {
    let (backend, profile) = testing::backend();

    let mut harness = canvas(backend, profile);
    harness.get_by(|node| node.author_id() == Some("canvas-error"));
    let before = harness.state().state.revision;
    harness
        .get_by(|node| node.author_id() == Some("canvas-retry"))
        .click();
    harness.run();

    assert_ne!(harness.state().state.revision, before);
}
