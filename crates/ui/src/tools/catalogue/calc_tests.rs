//! The check and unit tools over a real core: what is typed in, what comes out.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::text_tests::{click, has, opened, output_values, type_in, value_of, waits_for};
use crate::notes::bench::{Bench, settle};

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

    click(&mut harness, "tool-sample");

    waits_for(&mut harness, "the time is estimated", |harness| {
        has(harness, "transfer-time")
    });
    assert!(has(&harness, "transfer-connections"));
}

fn place(harness: &Harness<'_, Bench>, id: &str) -> egui::Rect {
    harness.get_by(|node| node.author_id() == Some(id)).rect()
}

/// Side by side in a wide window, one under the other in a narrow one, both answers on the page.
#[test]
fn the_two_parts_of_sizes_and_of_dates_are_on_one_page() {
    for (tool, first, second, answers) in [
        (
            "sizes",
            "sizes-input",
            "transfer-size",
            ["sizes-results", "transfer-time"],
        ),
        (
            "dates",
            "dates-input",
            "dates-from",
            ["dates-forms", "dates-gap"],
        ),
    ] {
        let mut harness = opened(tool);
        click(&mut harness, "tool-sample");
        waits_for(&mut harness, "both have answered", |harness| {
            answers.iter().all(|id| has(harness, id))
        });
        assert!(place(&harness, first).right() < place(&harness, second).left());

        harness.set_size(egui::vec2(800.0, 900.0));
        settle(&mut harness);
        assert!(place(&harness, first).bottom() < place(&harness, second).top());
    }
}

#[test]
fn two_parts_are_kept_as_one_note_and_one_alone_as_itself() {
    let part = |title: &str, content: &str| {
        Some(crate::tools::ToolResult {
            title: title.to_owned(),
            language: engine::notes::language::Language::Txt,
            content: content.to_owned(),
            warning: None,
        })
    };
    let together = crate::tools::ToolResult::together;

    let both = together(part("A size", "1 GiB"), part("A transfer", "6 min")).unwrap();
    assert_eq!(
        (both.title.as_str(), both.content.as_str()),
        (
            "A size",
            "1 GiB

6 min"
        )
    );
    let alone = together(None, part("A transfer", "6 min")).unwrap();
    assert_eq!(alone.title, "A transfer");
    assert!(together(None, None).is_none());
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

/// One frame a sixtieth of a second after the last, `events` in it: the typing's delay is long.
fn frame(harness: &mut Harness<'_, Bench>, events: Vec<egui::Event>) {
    let time = harness.ctx.input(|input| input.time) + 1.0 / 60.0;
    let input = harness.input_mut();
    input.time = Some(time);
    input.events.extend(events);
    harness.step();
    // Kept from a step to the next otherwise, and the clock would stand still.
    harness.input_mut().time = None;
}

fn press(harness: &mut Harness<'_, Bench>, id: &str) {
    let pos = harness
        .get_by(|node| node.author_id() == Some(id))
        .rect()
        .left_center()
        + egui::vec2(7.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(harness, vec![egui::Event::PointerMoved(pos)]);
    frame(harness, vec![button(true)]);
    frame(harness, vec![button(false)]);
}

/// Whether the box is painted ticked: its node says what it was before the click.
fn painted_ticked(harness: &Harness<'_, Bench>, id: &str) -> bool {
    let place = harness.get_by(|node| node.author_id() == Some(id)).rect();
    let amber = crate::theme::palette(&harness.ctx).amber_fill;
    harness.output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Rect(shape)
            if shape.fill == amber && place.contains_rect(shape.rect))
    })
}

/// The box as painted in every frame from the click to the answer.
#[test]
fn a_box_clicked_is_ticked_once_and_stays_so() {
    let mut harness = opened("permissions");
    type_in(&mut harness, "permissions-octal", "644");
    waits_for(&mut harness, "the mode is read", |harness| {
        has(harness, "permissions-words")
    });
    assert!(!painted_ticked(&harness, "permissions-owner-execute"));

    press(&mut harness, "permissions-owner-execute");
    let mut states = vec![painted_ticked(&harness, "permissions-owner-execute")];
    for _ in 0..30 {
        frame(&mut harness, Vec::new());
        states.push(painted_ticked(&harness, "permissions-owner-execute"));
    }
    assert!(states.iter().all(|on| *on), "{states:?}");
    settle(&mut harness);
    assert!(painted_ticked(&harness, "permissions-owner-execute"));
}

/// Two boxes before the core answered about the first: neither is lost.
#[test]
fn two_boxes_clicked_at_once_are_both_ticked() {
    let mut harness = opened("permissions");
    type_in(&mut harness, "permissions-octal", "600");
    waits_for(&mut harness, "the mode is read", |harness| {
        has(harness, "permissions-words")
    });

    press(&mut harness, "permissions-group-read");
    press(&mut harness, "permissions-others-read");
    settle(&mut harness);

    assert_eq!(value_of(&harness, "permissions-octal"), "644");
}

/// Where each text of the frame is painted.
fn texts(harness: &Harness<'_, Bench>) -> std::collections::BTreeMap<String, egui::Pos2> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if !text.galley.text().is_empty() => {
                Some((text.galley.text().to_owned(), text.pos.round()))
            }
            _ => None,
        })
        .collect()
}

/// What a kind shows is at its place from the frame of the click, its options with it.
#[test]
fn a_kind_of_identifier_shows_its_options_where_they_stay() {
    let mut harness = opened("identifiers");
    waits_for(&mut harness, "the list is drawn", |harness| {
        has(harness, "identifiers-row")
    });
    for kind in [3, 0, 6, 2, 3] {
        press(&mut harness, &format!("identifiers-kind-{kind}"));
        let first = texts(&harness);
        frame(&mut harness, Vec::new());
        frame(&mut harness, Vec::new());
        for (words, at) in texts(&harness) {
            if let Some(before) = first.get(&words) {
                assert_eq!(*before, at, "{words:?} moved after the kind {kind}");
            }
        }
        settle(&mut harness);
    }
}

/// The picker of a note keeps its height while its list is read.
#[test]
fn the_note_picker_does_not_move_once_its_notes_are_listed() {
    let mut harness = opened("convert");
    press(&mut harness, "note-import-text");
    let place = |harness: &Harness<'_, Bench>| {
        harness
            .query_by(|node| node.author_id() == Some("note-picker-cancel"))
            .map(|node| node.rect())
    };
    let mut places: Vec<egui::Rect> = Vec::new();
    for _ in 0..20 {
        frame(&mut harness, Vec::new());
        places.extend(place(&harness));
    }
    settle(&mut harness);
    let last = place(&harness).expect("the picker is open");
    assert!(places.iter().all(|at| *at == last), "{places:?}");
}

#[test]
fn numbers_are_drawn_between_a_beginning_and_an_end_and_an_iban_for_a_country() {
    let mut harness = opened("checks");
    waits_for(&mut harness, "numbers are drawn at once", |harness| {
        has(harness, "checks-numbers")
    });
    assert!(place(&harness, "checks-input").width() <= 421.0);
    assert!(place(&harness, "checks-input").right() < place(&harness, "checks-prefix").left());

    type_in(&mut harness, "checks-prefix", "41");
    type_in(&mut harness, "checks-suffix", "77");
    waits_for(&mut harness, "the numbers follow", |harness| {
        let values = output_values(harness);
        values.len() == 5
            && values
                .iter()
                .all(|it| it.starts_with("41") && it.ends_with("77") && it.len() == 19)
    });
    let before = output_values(&harness);
    click(&mut harness, "checks-generate");
    waits_for(&mut harness, "others are drawn", |harness| {
        output_values(harness) != before
    });

    type_in(&mut harness, "checks-prefix", "x");
    waits_for(&mut harness, "the letter is refused", |harness| {
        has(harness, "checks-generate-problem")
    });

    click(&mut harness, "checks-generate-kind-1");
    waits_for(&mut harness, "IBANs are drawn", |harness| {
        output_values(harness).iter().all(|it| it.starts_with("FR"))
            && output_values(harness).len() == 5
    });
}

/// What the `at`th of the copies named `id` puts on the clipboard.
fn copied(harness: &mut Harness<'_, Bench>, id: &str, at: usize) -> String {
    harness
        .get_all_by(|node| node.author_id() == Some(id))
        .nth(at)
        .unwrap()
        .click();
    let mut text = None;
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(copied) = command {
                text = Some(copied.clone());
            }
        }
    }
    text.expect("it was copied")
}

#[test]
fn a_printed_number_is_copied_as_it_is_shown_or_raw() {
    let mut harness = opened("checks");
    waits_for(&mut harness, "numbers are drawn", |harness| {
        has(harness, "checks-numbers")
    });
    let shown = output_values(&harness)[0].clone();
    assert!(shown.contains(' '));

    let first = harness
        .get_all_by(|node| node.author_id() == Some("copy-raw"))
        .count();
    assert_eq!(first, 5, "one a number");
    // « Copy all » comes before the rows.
    assert_eq!(copied(&mut harness, "copy-value", 1), shown);
    assert_eq!(copied(&mut harness, "copy-raw", 0), shown.replace(' ', ""));
}

#[test]
fn the_numbers_of_an_identity_have_both_copies_and_its_email_one() {
    let mut harness = opened("identity");
    waits_for(&mut harness, "a person is drawn", |harness| {
        has(harness, "identity-block-contact")
    });
    let raws: Vec<String> = harness
        .query_all_by(|node| node.author_id() == Some("copy-raw"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    for number in ["Phone", "Mobile", "IBAN"] {
        let wanted = format!("Copy {number}, raw");
        assert!(raws.contains(&wanted), "{number}: {raws:?}");
    }
    assert!(
        !raws
            .iter()
            .any(|it| it.contains("mail") || it.contains("Address"))
    );
}
