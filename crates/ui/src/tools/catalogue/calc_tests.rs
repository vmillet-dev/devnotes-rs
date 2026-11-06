//! The check and unit tools over a real core: what is typed in, what comes out.

use egui_kittest::kittest::{NodeT, Queryable};

use super::text_tests::{click, has, opened, output_values, type_in, value_of, waits_for};
use crate::notes::bench::settle;

#[test]
fn a_size_is_written_in_every_unit() {
    let mut harness = opened("sizes");

    type_in(&mut harness, "sizes-input", "1 GiB");
    waits_for(&mut harness, "the size is converted", |harness| {
        has(harness, "sizes-results")
    });

    assert!(has(&harness, "sizes-unit-in-text"));
    let values = output_values(&harness);
    assert!(
        values.iter().any(|it| it.contains("1,073,741,824")),
        "{values:?}"
    );
}

#[test]
fn what_is_no_size_says_where_it_stops() {
    let mut harness = opened("sizes");

    type_in(&mut harness, "sizes-input", "abc");

    waits_for(&mut harness, "the text is refused", |harness| {
        has(harness, "sizes-problem")
    });
}

#[test]
fn a_transfer_is_estimated_and_compared_with_common_links() {
    let mut harness = opened("sizes");
    click(&mut harness, "sizes-tab-1");

    click(&mut harness, "tool-sample");

    waits_for(&mut harness, "the time is estimated", |harness| {
        has(harness, "transfer-time")
    });
    assert!(has(&harness, "transfer-connections"));
}

#[test]
fn an_octal_mode_is_read_and_written_as_commands() {
    let mut harness = opened("permissions");

    type_in(&mut harness, "permissions-octal", "754");
    waits_for(&mut harness, "the mode is read", |harness| {
        has(harness, "permissions-words")
    });

    assert_eq!(value_of(&harness, "permissions-symbolic"), "rwxr-xr--");
    assert!(
        output_values(&harness)
            .iter()
            .any(|it| it.contains("chmod 754"))
    );
}

#[test]
fn a_box_rewrites_the_mode() {
    let mut harness = opened("permissions");
    type_in(&mut harness, "permissions-octal", "644");
    waits_for(&mut harness, "the mode is read", |harness| {
        has(harness, "permissions-words")
    });

    click(&mut harness, "permissions-owner-execute");

    assert_eq!(value_of(&harness, "permissions-octal"), "744");
}

#[test]
fn a_mode_with_a_wrong_digit_is_refused() {
    let mut harness = opened("permissions");

    type_in(&mut harness, "permissions-octal", "789");

    waits_for(&mut harness, "the mode is refused", |harness| {
        has(harness, "permissions-problem")
    });
}

#[test]
fn a_valid_iban_names_its_country_and_its_parts() {
    let mut harness = opened("checks");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the number is checked", |harness| {
        has(harness, "checks-result")
    });

    assert_eq!(value_of(&harness, "checks-verdict"), "Valid");
    assert!(
        output_values(&harness)
            .iter()
            .any(|it| it.contains("France (FR)"))
    );
}

#[test]
fn a_wrong_card_number_gives_the_digit_it_asks_for_and_leaves_no_trace() {
    let mut harness = opened("checks");

    type_in(&mut harness, "checks-input", "4111111111111112");
    waits_for(&mut harness, "the number is checked", |harness| {
        has(harness, "checks-result")
    });
    assert_eq!(value_of(&harness, "checks-verdict"), "Invalid");

    assert!(!has(&harness, "tool-save-as-note"));
    click(&mut harness, "tool-back");
    let bench = harness.state_mut();
    let open = crate::action::Action::Tools(crate::tools::Action::Open("checks"));
    bench
        .state
        .apply(open, &egui::Context::default(), &bench.backend);
    settle(&mut harness);
    assert_eq!(value_of(&harness, "checks-input"), "");
}

#[test]
fn a_block_is_described_with_what_it_holds() {
    let mut harness = opened("cidr");

    type_in(&mut harness, "cidr-network", "192.168.1.0/24");
    waits_for(&mut harness, "the block is read", |harness| {
        has(harness, "cidr-facts")
    });

    let values = output_values(&harness);
    assert!(values.contains(&"255.255.255.0".to_owned()), "{values:?}");
    assert!(values.contains(&"192.168.1.255".to_owned()));
    assert!(has(&harness, "cidr-binary"));
}

#[test]
fn an_address_is_found_inside_or_outside_the_block() {
    let mut harness = opened("cidr");
    type_in(&mut harness, "cidr-network", "192.168.1.0/24");
    type_in(&mut harness, "cidr-member", "192.168.1.40");

    waits_for(&mut harness, "the address is placed", |harness| {
        has(harness, "cidr-membership")
    });
}

#[test]
fn a_mask_chosen_becomes_the_block() {
    let mut harness = opened("cidr");
    type_in(&mut harness, "cidr-network", "10.0.0.0/8");
    waits_for(&mut harness, "the masks are listed", |harness| {
        has(harness, "cidr-masks")
    });

    click(&mut harness, "cidr-masks-row-1");

    assert!(value_of(&harness, "cidr-network").starts_with("10.0.0.0/"));
    assert_ne!(value_of(&harness, "cidr-network"), "10.0.0.0/8");
}

#[test]
fn what_is_no_block_is_refused() {
    let mut harness = opened("cidr");

    type_in(&mut harness, "cidr-network", "not a block");

    waits_for(&mut harness, "the block is refused", |harness| {
        has(harness, "cidr-problem")
    });
}

#[test]
fn the_ports_are_found_by_the_start_of_their_number() {
    let mut harness = opened("ports");
    settle(&mut harness);

    type_in(&mut harness, "reference-search", "5432");

    let rows: Vec<_> = harness
        .query_all_by(|node| node.author_id() == Some("reference-row"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    assert_eq!(rows, ["5432"]);
}
