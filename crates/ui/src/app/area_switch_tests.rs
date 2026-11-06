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
