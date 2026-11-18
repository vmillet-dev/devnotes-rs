//! What the pointer says over each kind of control: decided in one place, from what it senses.

use egui::CursorIcon;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

/// The pointer's shape once it rests on the first control addressed `author_id`.
fn over(harness: &mut Harness<'_, Bench>, author_id: &str) -> CursorIcon {
    let centre = harness
        .get_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .unwrap()
        .rect()
        .center();
    harness.event(egui::Event::PointerMoved(centre));
    harness.run();
    harness.output().platform_output.cursor_icon
}

#[test]
fn a_hand_over_what_clicks_an_open_hand_over_what_drags_an_i_beam_over_text() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let home = crate::notes::bench::space(&backend, "Work");
    crate::notes::bench::folder(&backend, &home, None, "Runbooks");
    let mut harness = area(backend, profile);

    assert_eq!(over(&mut harness, "new-note"), CursorIcon::PointingHand);
    assert_eq!(
        over(&mut harness, "note-card-open"),
        CursorIcon::PointingHand
    );
    assert_eq!(over(&mut harness, "folder-option"), CursorIcon::Grab);
    assert_eq!(over(&mut harness, "search-input"), CursorIcon::Text);

    click_id(&mut harness, "note-card-open");
    settle(&mut harness);
    assert_eq!(over(&mut harness, "editor-body"), CursorIcon::Text);
}

#[test]
fn a_disabled_control_keeps_the_arrow() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    click_id(&mut harness, "space-option-all");

    assert_eq!(over(&mut harness, "view-board"), CursorIcon::Default);
}
