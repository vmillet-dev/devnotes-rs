//! The time tools over a real core: what is typed in, what comes out.

use super::text_tests::{click, has, opened, output_values, type_in, value_of, waits_for};

#[test]
fn an_instant_is_read_in_every_form() {
    let mut harness = opened("dates");

    type_in(&mut harness, "dates-input", "1700000000");
    waits_for(&mut harness, "the instant is read", |harness| {
        has(harness, "dates-forms")
    });

    let values = output_values(&harness);
    assert!(values.contains(&"1700000000000".to_owned()), "{values:?}");
    assert!(
        values.contains(&"2023-11-14T22:13:20Z".to_owned()),
        "{values:?}"
    );
    assert!(has(&harness, "dates-reading"));
}

#[test]
fn what_is_no_date_says_where_it_stops() {
    let mut harness = opened("dates");

    type_in(&mut harness, "dates-input", "not a date");

    waits_for(&mut harness, "the text is refused", |harness| {
        has(harness, "dates-problem")
    });
}

#[test]
fn now_fills_the_field() {
    let mut harness = opened("dates");

    click(&mut harness, "dates-now");

    waits_for(&mut harness, "the field is filled", |harness| {
        !value_of(harness, "dates-input").is_empty()
    });
}

#[test]
fn a_gap_and_a_duration_are_measured_and_read_aloud() {
    let mut harness = opened("dates");
    click(&mut harness, "dates-tab-1");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the gap is measured", |harness| {
        has(harness, "dates-gap")
    });

    assert!(value_of(&harness, "dates-gap-calendar").contains("6 months"));
    assert_eq!(
        value_of(&harness, "dates-duration-words"),
        "1 hour 30 minutes"
    );
}

#[test]
fn a_time_is_placed_in_the_zones_of_the_list() {
    let mut harness = opened("zones");

    click(&mut harness, "tool-sample");

    waits_for(&mut harness, "the zones are shown", |harness| {
        has(harness, "zones-row")
    });
    assert!(has(&harness, "zones-from"));
    assert!(has(&harness, "zones-remove-Asia/Tokyo"));
}

#[test]
fn a_zone_found_joins_the_list_and_leaves_it() {
    let mut harness = opened("zones");
    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the zones are shown", |harness| {
        has(harness, "zones-row")
    });

    type_in(&mut harness, "zones-search", "Sydney");
    waits_for(&mut harness, "the zone is found", |harness| {
        has(harness, "zones-found-Australia/Sydney")
    });
    click(&mut harness, "zones-found-Australia/Sydney");
    waits_for(&mut harness, "the zone is listed", |harness| {
        has(harness, "zones-remove-Australia/Sydney")
    });

    click(&mut harness, "zones-remove-Australia/Sydney");
    waits_for(&mut harness, "the zone is gone", |harness| {
        !has(harness, "zones-remove-Australia/Sydney")
    });
}

#[test]
fn a_cron_expression_is_read_aloud_with_its_next_runs() {
    let mut harness = opened("cron");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the expression is read", |harness| {
        has(harness, "cron-sentence")
    });

    assert!(has(&harness, "cron-field"));
    assert!(has(&harness, "cron-run"));
}

#[test]
fn a_cron_field_out_of_range_says_which_and_where() {
    let mut harness = opened("cron");

    type_in(&mut harness, "cron-input", "61 * * * *");

    waits_for(&mut harness, "the expression is refused", |harness| {
        has(harness, "cron-problem")
    });
    assert!(value_of(&harness, "cron-problem").contains("61"));
}

#[test]
fn a_moment_is_checked_against_the_expression() {
    let mut harness = opened("cron");
    type_in(&mut harness, "cron-input", "0 12 * * *");

    type_in(&mut harness, "cron-check", "2026-10-14 12:00");

    waits_for(&mut harness, "the moment is checked", |harness| {
        has(harness, "cron-verdict")
    });
}
