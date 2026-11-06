//! The Tools area over a real core.

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use crate::notes::bench::{Bench, area_of, settle};
use crate::state::Area;
use crate::testing;

pub(crate) fn tools() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Tools, backend, profile);
    // The window finds ≈ and − in the system's fonts, which a bench has not.
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    settle(&mut harness);
    harness
}

#[track_caller]
fn click(harness: &mut Harness<'_, Bench>, id: &str) {
    harness.get_by(|node| node.author_id() == Some(id)).click();
    settle(harness);
}

fn present(harness: &Harness<'_, Bench>, id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(id))
        .is_some()
}

fn type_search(harness: &mut Harness<'_, Bench>, text: &str) {
    harness
        .get_by(|node| node.author_id() == Some("tools-search"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("tools-search"))
        .type_text(text);
    settle(harness);
}

fn entries(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| {
            node.author_id()
                .is_some_and(|id| id.starts_with("tools-entry-") && !id.contains("-star-"))
        })
        .filter_map(|node| {
            let node = node.accesskit_node();
            Some(node.author_id()?["tools-entry-".len()..].to_owned())
        })
        .collect()
}

#[test]
fn the_home_lists_the_catalogue_and_says_how_many() {
    let harness = tools();

    assert_eq!(entries(&harness).len(), 32);
    let count = harness.get_by(|node| node.author_id() == Some("tools-count"));
    assert_eq!(
        count.accesskit_node().value().as_deref(),
        Some("32 utilities, all offline")
    );
}

#[test]
fn a_keyword_finds_a_tool_in_any_case_and_a_category_finds_its_tools() {
    let mut harness = tools();

    type_search(&mut harness, "CrLf");
    assert_eq!(entries(&harness), ["line-breaks"]);
}

#[test]
fn nothing_found_says_so() {
    let mut harness = tools();

    type_search(&mut harness, "zzzz");

    assert!(entries(&harness).is_empty());
    assert!(present(&harness, "tools-no-match"));
}

#[test]
fn enter_opens_the_first_result_and_escape_empties_the_search() {
    let mut harness = tools();
    type_search(&mut harness, "cron");
    assert!(present(&harness, "tools-first-result"));

    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert_eq!(entries(&harness).len(), 32);

    type_search(&mut harness, "cron");
    harness.key_press(Key::Enter);
    settle(&mut harness);
    let title = harness.get_by(|node| node.author_id() == Some("tool-title"));
    assert_eq!(title.accesskit_node().label().as_deref(), Some("Cron"));
}

#[test]
fn the_arrows_walk_the_results_and_come_back_to_the_field() {
    let mut harness = tools();
    type_search(&mut harness, "base");

    harness.key_press(Key::ArrowDown);
    settle(&mut harness);
    let focused = |harness: &Harness<'_, Bench>| {
        harness
            .query_by(|node| node.is_focused() && node.author_id().is_some())
            .and_then(|node| node.accesskit_node().author_id().map(str::to_owned))
    };
    assert_eq!(focused(&harness).as_deref(), Some("tools-entry-base64"));

    harness.key_press(Key::ArrowUp);
    settle(&mut harness);
    assert_eq!(focused(&harness).as_deref(), Some("tools-search"));
}

#[test]
fn a_category_of_the_rail_narrows_the_home_and_all_tools_widens_it() {
    let mut harness = tools();

    click(&mut harness, "tools-rail-category-time");
    assert_eq!(entries(&harness), ["dates", "zones", "cron"]);

    click(&mut harness, "tools-rail-all");
    assert_eq!(entries(&harness).len(), 32);
}

#[test]
fn the_rail_put_away_the_bar_says_the_filter_and_lifts_it() {
    let mut harness = tools();
    click(&mut harness, "tools-rail-category-time");

    click(&mut harness, "tools-rail-toggle");
    assert!(present(&harness, "tools-category-shown"));

    click(&mut harness, "tools-category-clear");
    assert_eq!(entries(&harness).len(), 32);
    assert!(present(&harness, "tools-count"));
}

#[test]
fn a_tool_opens_in_its_frame_with_its_siblings_and_comes_back_home() {
    let mut harness = tools();

    click(&mut harness, "tools-entry-hash");

    assert!(present(&harness, "tool-title"));
    assert!(present(&harness, "tool-clear"));
    assert!(present(&harness, "tools-rail-tool-jwt"));
    assert!(!present(&harness, "tools-rail-category-time"));

    click(&mut harness, "tool-back");
    assert!(present(&harness, "tools-search"));
}

#[test]
fn a_tool_opened_is_recent_and_the_rail_lists_it_beside_another_category() {
    let mut harness = tools();
    click(&mut harness, "tools-entry-case");
    click(&mut harness, "tool-back");
    assert!(present(&harness, "tools-recent-case"));

    click(&mut harness, "tools-entry-hash");

    assert!(present(&harness, "tools-rail-recent-case"));
    assert!(!present(&harness, "tools-rail-recent-hash"));
}

#[test]
fn a_star_files_the_tool_under_the_favourites_and_the_last_one_gone_takes_them_away() {
    let mut harness = tools();
    click(&mut harness, "tools-entry-star-jwt");
    assert!(present(&harness, "tools-favourite-jwt"));

    click(&mut harness, "tools-rail-favourites");
    assert_eq!(entries(&harness), ["jwt"]);

    click(&mut harness, "tools-entry-star-jwt");
    assert!(!present(&harness, "tools-rail-favourites"));
    assert_eq!(entries(&harness).len(), 32);
}

#[test]
fn the_star_of_the_frame_is_the_same_star() {
    let mut harness = tools();
    click(&mut harness, "tools-entry-hash");

    click(&mut harness, "tool-favourite");
    click(&mut harness, "tool-back");

    assert!(present(&harness, "tools-favourite-hash"));
}

#[test]
fn ctrl_k_takes_an_open_tool_back_to_the_search() {
    let mut harness = tools();
    click(&mut harness, "tools-entry-hash");

    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    settle(&mut harness);

    let search = harness.get_by(|node| node.author_id() == Some("tools-search"));
    assert!(search.is_focused());
}

#[test]
fn ctrl_shift_t_shows_the_search_from_another_area() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Http, backend, profile);
    settle(&mut harness);

    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::T);
    settle(&mut harness);

    let search = harness.get_by(|node| node.author_id() == Some("tools-search"));
    assert!(search.is_focused());
}
