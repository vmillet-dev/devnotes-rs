//! The canvas's keys, one table (`CANVAS_KEYS` in the reference), read by `keys::dispatch` after
//! the focused widget and never while a field is typed into. They read the character, not the
//! position: « P » pins wherever the layout puts it. Enter is not here: a card's button answers
//! it itself, as every focused button does.

use egui::{Key, Modifiers};
use engine::notes::kind::NoteKind;
use engine::notes::priority::Priority;

use super::grid::Direction;
use crate::action::Action;
use crate::keys::{Binding, Read};

/// What a key asks of the canvas; the card it acts on is the one with the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasKey {
    New(NoteKind),
    Move(Direction),
    Copy,
    Pin,
    Priority(Priority),
    /// Ticks the focused card, or unticks it.
    Check,
    /// Twice, the way the card's menu asks for two clicks: the focused card can be scrolled
    /// out of view.
    Trash,
    /// Down the ladder: what is armed, then the search, then up a folder.
    StepBack,
}

const fn bound(key: Key, modifiers: Modifiers, canvas: CanvasKey) -> Binding {
    Binding {
        key,
        modifiers,
        read: Read::Character,
        while_typing: false,
        action: Action::Notes(crate::notes::Action::Canvas(canvas)),
    }
}

const fn bare(key: Key, canvas: CanvasKey) -> Binding {
    bound(key, Modifiers::NONE, canvas)
}

const CREATE: Modifiers = Modifiers::COMMAND;
const CREATE_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// In reading order, which is the order the keyboard sheet lists them in.
pub static CANVAS: [Binding; 20] = [
    bound(Key::N, CREATE, CanvasKey::New(NoteKind::Snippet)),
    bound(Key::N, CREATE_SHIFT, CanvasKey::New(NoteKind::Note)),
    bound(Key::L, CREATE_SHIFT, CanvasKey::New(NoteKind::Checklist)),
    bound(Key::M, CREATE_SHIFT, CanvasKey::New(NoteKind::Calc)),
    bound(Key::D, CREATE_SHIFT, CanvasKey::New(NoteKind::Drawing)),
    bare(Key::ArrowLeft, CanvasKey::Move(Direction::Previous)),
    bare(Key::ArrowRight, CanvasKey::Move(Direction::Next)),
    bare(Key::ArrowUp, CanvasKey::Move(Direction::Up)),
    bare(Key::ArrowDown, CanvasKey::Move(Direction::Down)),
    bare(Key::C, CanvasKey::Copy),
    bare(Key::P, CanvasKey::Pin),
    // Each digit a level, in the order the menus list them.
    bare(Key::Num0, CanvasKey::Priority(Priority::None)),
    bare(Key::Num1, CanvasKey::Priority(Priority::Low)),
    bare(Key::Num2, CanvasKey::Priority(Priority::Medium)),
    bare(Key::Num3, CanvasKey::Priority(Priority::High)),
    bare(Key::Num4, CanvasKey::Priority(Priority::Urgent)),
    bare(Key::X, CanvasKey::Check),
    bare(Key::Delete, CanvasKey::Trash),
    bare(Key::Backspace, CanvasKey::Trash),
    bare(Key::Escape, CanvasKey::StepBack),
];

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use engine::notes::priority::Priority;

    use crate::notes::bench::{Bench, area, cards, click_id, settle};
    use crate::testing;

    fn press(harness: &mut Harness<'_, Bench>, key: Key) {
        harness.key_press(key);
        settle(harness);
    }

    fn focused(harness: &Harness<'_, Bench>) -> Option<String> {
        harness.state().state.notes_view.canvas.focused.clone()
    }

    fn title_of(harness: &Harness<'_, Bench>, id: &str) -> String {
        let view = harness.state().state.notes_query.view().unwrap();
        view.cards()
            .find(|card| card.id == id)
            .map(|card| card.title.clone())
            .unwrap_or_default()
    }

    fn notes(count: usize) -> (crate::backend::Backend, tempfile::TempDir) {
        let (backend, profile) = testing::unlocked();
        for at in 0..count {
            testing::note(&backend, &format!("note {at}"));
        }
        (backend, profile)
    }

    #[test]
    fn the_arrows_walk_the_grid_from_the_first_card() {
        let (backend, profile) = notes(6);
        let mut harness = area(backend, profile);

        press(&mut harness, Key::ArrowDown);
        let first = focused(&harness).unwrap();
        press(&mut harness, Key::ArrowRight);
        let second = focused(&harness).unwrap();
        press(&mut harness, Key::ArrowLeft);

        assert_ne!(first, second);
        assert_eq!(focused(&harness).as_deref(), Some(first.as_str()));
        // Down a row is `columns` cards on, whatever the width laid out.
        let columns = harness.state().state.notes_view.canvas.columns;
        press(&mut harness, Key::ArrowDown);
        let below = focused(&harness).unwrap();
        let view = harness.state().state.notes_query.view().unwrap();
        let order: Vec<&str> = view.cards().map(|card| card.id.as_str()).collect();
        let at = |id: &str| order.iter().position(|each| *each == id).unwrap();
        assert_eq!(at(&below), (at(&first) + columns).min(order.len() - 1));
    }

    #[test]
    fn p_pins_a_digit_sets_a_priority_on_the_focused_card() {
        let (backend, profile) = notes(1);
        let mut harness = area(backend, profile);
        press(&mut harness, Key::ArrowDown);
        let id = focused(&harness).unwrap();

        press(&mut harness, Key::P);
        press(&mut harness, Key::Num4);

        let view = harness.state().state.notes_query.view().unwrap();
        let card = view.cards().find(|card| card.id == id).unwrap();
        assert!(card.pinned);
        assert_eq!(card.priority, Priority::Urgent);
        assert_eq!(title_of(&harness, &id), "note 0");
    }

    #[test]
    fn delete_arms_the_card_escape_disarms_it_twice_trashes_it() {
        let (backend, profile) = notes(1);
        let mut harness = area(backend, profile);
        press(&mut harness, Key::ArrowDown);

        press(&mut harness, Key::Delete);
        harness.get_by(|node| node.author_id() == Some("note-card-arming"));
        press(&mut harness, Key::Escape);
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("note-card-arming"))
                .is_none()
        );
        assert_eq!(cards(&harness), 1);

        press(&mut harness, Key::Delete);
        press(&mut harness, Key::Delete);
        assert_eq!(cards(&harness), 0);
    }

    #[test]
    fn escape_clears_the_search_before_leaving_the_folder() {
        let (backend, profile) = testing::unlocked();
        let filed = testing::note(&backend, "filed").id.clone();
        let spaces = crate::notes::bench::spaces_of_backend(&backend);
        let folder = crate::notes::bench::folder(&backend, &spaces[0], None, "Runbooks");
        let into = folder.id.clone();
        engine::db::run_here(backend.engine(), move |engine| {
            engine::folders::file_notes(engine, vec![filed], Some(into))
        })
        .unwrap();
        let mut harness = area(backend, profile);
        crate::notes::bench::click(&mut harness, "folder-option", "Runbooks");
        click_id(&mut harness, "search-input");
        harness
            .get_by(|node| node.author_id() == Some("search-input"))
            .type_text("nothing");
        settle(&mut harness);
        press(&mut harness, Key::Escape);

        assert_eq!(harness.state().state.notes_query.typed(), "");
        assert_eq!(
            harness.state().state.spaces.active_folder(),
            Some(folder.id.as_str())
        );

        press(&mut harness, Key::Escape);
        assert_eq!(harness.state().state.spaces.active_folder(), None);
    }

    #[test]
    fn ctrl_n_opens_a_snippet_draft() {
        let (backend, profile) = notes(1);
        let mut harness = area(backend, profile);

        harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
        settle(&mut harness);

        let opened = harness.state().state.editor.opened().unwrap();
        assert_eq!(opened.kind, engine::notes::kind::NoteKind::Snippet);
    }
}
