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
use crate::services::shortcuts::Bindings;

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
    /// The last write put back, even once the bar is gone: the gesture made without looking.
    Undo,
    /// Down the ladder: what is armed, then the search, then up a folder.
    StepBack,
    /// The board's loose cards flowed back under the zones; nothing on the list.
    Align,
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
pub static CANVAS: [Binding; 22] = [
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
    bound(Key::Z, Modifiers::COMMAND, CanvasKey::Undo),
    bare(Key::Escape, CanvasKey::StepBack),
    bare(Key::A, CanvasKey::Align),
];

/// The rebindable entries, by the `CanvasKey` each answers for: the table is the defaults.
fn rebindable(canvas: CanvasKey) -> Option<&'static str> {
    Some(match canvas {
        CanvasKey::New(NoteKind::Snippet) => "canvas.newSnippet",
        CanvasKey::New(NoteKind::Note) => "canvas.newNote",
        CanvasKey::New(NoteKind::Checklist) => "canvas.newChecklist",
        CanvasKey::New(NoteKind::Calc) => "canvas.newCalc",
        CanvasKey::New(NoteKind::Drawing) => "canvas.newDrawing",
        CanvasKey::Copy => "canvas.copy",
        CanvasKey::Pin => "canvas.pin",
        CanvasKey::Check => "canvas.check",
        CanvasKey::Align => "canvas.align",
        CanvasKey::Trash => "canvas.trash",
        CanvasKey::Undo => "canvas.undo",
        _ => return None,
    })
}

/// The table with the keys the person moved. Backspace stays on the trash whatever Delete
/// became: it has always trashed a note, and making the key movable took nothing away.
pub fn bound_to(bindings: &Bindings) -> Vec<Binding> {
    CANVAS
        .iter()
        .map(|binding| {
            let Action::Notes(crate::notes::Action::Canvas(canvas)) = binding.action else {
                return binding.clone();
            };
            let moved = rebindable(canvas)
                .filter(|_| binding.key != Key::Backspace)
                .and_then(|id| {
                    crate::services::shortcuts::ACTIONS
                        .iter()
                        .find(|it| it.id == id)
                })
                .and_then(|action| bindings.keystroke(*action));
            match moved {
                Some((key, modifiers)) => Binding {
                    key,
                    modifiers,
                    ..binding.clone()
                },
                None => binding.clone(),
            }
        })
        .collect()
}

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
    fn a_moved_key_answers_and_the_old_one_no_longer_does() {
        use crate::services::shortcuts::{ACTIONS, Bindings};
        let mut bindings = Bindings::default();
        bindings.set(ACTIONS[9], "J".to_owned());

        let table = super::bound_to(&bindings);

        let pin = |key| {
            table.iter().any(|binding| {
                binding.key == key
                    && binding.action
                        == crate::action::Action::Notes(crate::notes::Action::Canvas(
                            super::CanvasKey::Pin,
                        ))
            })
        };
        assert!(pin(Key::J));
        assert!(!pin(Key::P));
    }

    #[test]
    fn backspace_stays_on_the_trash_when_delete_moves() {
        use crate::services::shortcuts::{ACTIONS, Bindings};
        let mut bindings = Bindings::default();
        bindings.set(ACTIONS[12], "D".to_owned());

        let table = super::bound_to(&bindings);

        assert!(table.iter().any(|binding| binding.key == Key::Backspace));
        assert!(table.iter().any(|binding| binding.key == Key::D));
        assert!(!table.iter().any(|binding| binding.key == Key::Delete));
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
    fn the_focus_follows_a_card_that_moves_and_a_second_p_gives_it_back() {
        let (backend, profile) = notes(4);
        let mut harness = area(backend, profile);
        press(&mut harness, Key::ArrowDown);
        press(&mut harness, Key::ArrowRight);
        press(&mut harness, Key::ArrowRight);
        let id = focused(&harness).unwrap();

        press(&mut harness, Key::P);
        assert_eq!(focused(&harness).as_deref(), Some(id.as_str()));
        press(&mut harness, Key::P);

        let view = harness.state().state.notes_query.view().unwrap();
        assert!(
            view.cards().all(|card| !card.pinned),
            "the same note, twice"
        );
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
            harness.state().state.active_folder(),
            Some(folder.id.as_str())
        );

        press(&mut harness, Key::Escape);
        assert_eq!(harness.state().state.active_folder(), None);
    }

    #[test]
    fn ctrl_n_opens_a_snippet_draft() {
        let (backend, profile) = notes(1);
        let mut harness = area(backend, profile);

        harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
        settle(&mut harness);

        let opened = harness.state().state.editor.opened().unwrap();
        assert_eq!(
            opened.note().unwrap().kind,
            engine::notes::kind::NoteKind::Snippet
        );
    }
}
