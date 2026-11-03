use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use i18n::Language;

use super::*;
use crate::backend::Backend;
use crate::testing;

/// The gate alone, over a real core: drawn, its actions applied, its answers adopted.
struct Screen {
    backend: Backend,
    _profile: tempfile::TempDir,
    gate: Gate,
    view: GateView,
    status: StatusNotifier,
    errors: ErrorNotifier,
}

fn screen(backend: Backend, profile: tempfile::TempDir) -> Harness<'static, Screen> {
    let mut gate = Gate::default();
    gate.ask(&backend);
    backend.settle();
    testing::harness()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(
            |ui, screen: &mut Screen| {
                screen
                    .gate
                    .update(&screen.backend, &mut screen.status, &mut screen.errors);
                let mut asked = Vec::new();
                show(ui, &screen.gate, &mut screen.view, Language::En, &mut asked);
                for action in asked {
                    screen.gate.apply(action, &mut screen.view, &screen.backend);
                }
            },
            Screen {
                backend,
                _profile: profile,
                gate,
                view: GateView::default(),
                status: StatusNotifier::default(),
                errors: ErrorNotifier::default(),
            },
        )
}

fn node<'a>(harness: &'a Harness<'_, Screen>, author_id: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(|node| node.author_id() == Some(author_id))
}

fn present(harness: &Harness<'_, Screen>, author_id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(author_id))
        .is_some()
}

fn type_into(harness: &mut Harness<'_, Screen>, author_id: &str, words: &str) {
    node(harness, author_id).click();
    harness.run();
    node(harness, author_id).type_text(words);
    harness.run();
}

/// Until the core answered and the frame after drew it.
fn settle(harness: &mut Harness<'_, Screen>) {
    harness.state().backend.settle();
    harness.run();
}

/// Until `author_id` is drawn: an answer may ask the core again, and that answer comes later.
fn settle_until(harness: &mut Harness<'_, Screen>, author_id: &str) {
    for _ in 0..10 {
        if present(harness, author_id) {
            return;
        }
        settle(harness);
    }
    panic!(
        "{author_id} never drawn; the vault is {:?}, the work {}",
        harness.state().gate.vault,
        if harness.state().gate.work.is_some() {
            "pending"
        } else {
            "done"
        }
    );
}

#[test]
fn a_first_launch_asks_for_a_phrase_twice_and_holds_it_to_twelve_characters() {
    let (backend, profile) = testing::backend();
    let mut harness = screen(backend, profile);
    harness.run();

    assert!(present(&harness, "vault-confirmation"));
    assert!(
        !present(&harness, "vault-forgotten"),
        "a fresh library has nothing to forget"
    );

    type_into(&mut harness, "vault-passphrase", "too short");
    type_into(&mut harness, "vault-confirmation", "too short");
    assert!(
        node(&harness, "vault-submit")
            .accesskit_node()
            .is_disabled()
    );

    type_into(&mut harness, "vault-passphrase", " no more");
    assert!(
        node(&harness, "vault-submit")
            .accesskit_node()
            .is_disabled(),
        "the two differ"
    );
    type_into(&mut harness, "vault-confirmation", " no more");
    assert!(
        !node(&harness, "vault-submit")
            .accesskit_node()
            .is_disabled()
    );

    node(&harness, "vault-submit").click();
    harness.run();
    settle(&mut harness);

    assert!(harness.state().gate.is_open());
    assert!(
        harness.state().view.passphrase.is_empty(),
        "nothing typed is kept"
    );
}

#[test]
fn an_unlock_takes_any_length_and_says_when_the_phrase_is_wrong() {
    let profile = testing::locked();
    let backend = testing::backend_on(profile.path());
    let mut harness = screen(backend, profile);
    harness.run();

    assert!(!present(&harness, "vault-confirmation"));
    assert!(present(&harness, "vault-forgotten"));
    type_into(&mut harness, "vault-passphrase", "short");
    assert!(
        !node(&harness, "vault-submit")
            .accesskit_node()
            .is_disabled(),
        "no floor to unlock"
    );

    harness.key_press(Key::Enter);
    harness.run();
    settle(&mut harness);
    assert!(harness.state().gate.refused);
    let problem = node(&harness, "vault-problem").accesskit_node();
    let said = problem
        .label()
        .or_else(|| problem.value())
        .unwrap_or_default();
    assert!(said.contains("passphrase"), "{said:?}");

    type_into(&mut harness, "vault-passphrase", testing::PASSPHRASE);
    harness.key_press(Key::Enter);
    harness.run();
    settle(&mut harness);
    assert!(harness.state().gate.is_open());
}

#[test]
fn a_forgotten_phrase_asks_before_archiving_and_can_be_left() {
    let profile = testing::locked();
    let backend = testing::backend_on(profile.path());
    let mut harness = screen(backend, profile);
    harness.run();

    node(&harness, "vault-forgotten").click();
    harness.run();
    assert!(present(&harness, "vault-archive"));
    assert!(present(&harness, "vault-archive-confirm"));

    node(&harness, "vault-keep-trying").click();
    harness.run();
    assert!(present(&harness, "vault-passphrase"));
}

#[test]
fn archiving_a_forgotten_library_says_where_and_starts_a_new_one() {
    let profile = testing::locked();
    let backend = testing::backend_on(profile.path());
    let mut harness = screen(backend, profile);
    harness.run();

    node(&harness, "vault-forgotten").click();
    harness.run();
    node(&harness, "vault-archive-confirm").click();
    settle_until(&mut harness, "vault-confirmation");

    assert!(harness.state().status.latest().is_some());
    assert!(harness.state().errors.notice().is_none());
}

#[test]
fn opening_another_library_asks_for_its_own_phrase() {
    let profile = testing::locked();
    let backend = testing::backend_on(profile.path());
    let other = engine::libraries::create_library(backend.engine(), "Travail".to_string())
        .unwrap()
        .id;
    let mut harness = screen(backend, profile);
    settle(&mut harness);

    let screen = harness.state_mut();
    let rank = screen
        .gate
        .registry
        .as_ref()
        .unwrap()
        .libraries
        .iter()
        .position(|entry| entry.id == other)
        .unwrap();
    screen.view.switch_to = Some(rank);
    screen
        .gate
        .apply(Action::SwitchLibrary, &mut screen.view, &screen.backend);
    settle_until(&mut harness, "vault-confirmation");

    let open = engine::libraries::list_libraries(harness.state().backend.engine()).open;
    assert_eq!(open.as_deref(), Some(other.as_str()));
}
