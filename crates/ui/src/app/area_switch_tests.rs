//! The switch between the areas heads the rail; the bar takes it only while the rail is away.

use egui_kittest::kittest::Queryable;

use super::tests::{click, open_library};

fn switches(harness: &egui_kittest::Harness<'_, super::DevNotesApp>) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some("area-switch"))
        .count()
}

fn top(harness: &egui_kittest::Harness<'_, super::DevNotesApp>) -> f32 {
    harness
        .get_by(|node| node.author_id() == Some("area-option-notes"))
        .rect()
        .top()
}

#[test]
fn the_switch_leaves_the_rail_for_the_bar_when_the_rail_is_put_away() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    assert_eq!(switches(&harness), 1);
    let in_the_rail = top(&harness);

    click(&mut harness, "library-rail-toggle");

    assert_eq!(switches(&harness), 1);
    assert!(top(&harness) < in_the_rail, "the bar is above the rail");
}

#[test]
fn each_area_is_reached_from_the_switch_of_its_rail() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    click(&mut harness, "area-option-tools");
    assert_eq!(switches(&harness), 1);
    click(&mut harness, "area-option-http");
    assert_eq!(switches(&harness), 1);
    click(&mut harness, "area-option-notes");
    assert_eq!(switches(&harness), 1);
}

/// Where the three buttons are drawn, the area `area` on screen.
fn places(
    harness: &mut egui_kittest::Harness<'_, super::DevNotesApp>,
    area: &str,
) -> Vec<egui::Rect> {
    click(harness, area);
    ["area-option-notes", "area-option-tools", "area-option-http"]
        .map(|author_id| {
            harness
                .get_by(|node| node.author_id() == Some(author_id))
                .rect()
        })
        .to_vec()
}

#[test]
fn the_switch_is_at_one_place_and_one_size_on_every_area() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    for rail_away in [false, true] {
        if rail_away {
            click(&mut harness, "area-option-notes");
            click(&mut harness, "library-rail-toggle");
        }
        let notes = places(&mut harness, "area-option-notes");
        assert_eq!(places(&mut harness, "area-option-tools"), notes);
        assert_eq!(places(&mut harness, "area-option-http"), notes);
    }
}

/// The two halves of a split button, and the one rounded outline it is drawn in.
fn halves(
    harness: &mut egui_kittest::Harness<'_, super::DevNotesApp>,
    area: &str,
    ids: [&str; 2],
) -> [egui::Rect; 2] {
    click(harness, area);
    ids.map(|author_id| {
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .rect()
    })
}

#[test]
fn new_request_has_the_shape_of_new_note() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    let note = halves(
        &mut harness,
        "area-option-notes",
        ["new-note", "new-note-kind"],
    );
    let request = halves(
        &mut harness,
        "area-option-http",
        ["http-tab-new", "http-tab-new-kind"],
    );

    for [main, more] in [note, request] {
        assert!(
            (main.right() - more.left()).abs() < 0.1,
            "{main:?} {more:?}"
        );
        assert!((main.height() - more.height()).abs() < 0.1);
    }
    assert!((note[0].height() - request[0].height()).abs() < 0.1);
    assert!((note[1].size() - request[1].size()).length() < 0.1);
}
