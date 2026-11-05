//! Who gets a key (D14). egui propagates keys like the DOM does not — no capture, no bubbling —
//! so without one order Escape, `Ctrl+Z`, Tab and `Ctrl+1/2/3` would fight between screens:
//!
//! 1. a composition under way (an IME) keeps everything;
//! 2. while drawing, the focused widget consumes what it handles;
//! 3. after drawing, [`dispatch`] reads what is left by level — the top modal, then the area on
//!    screen, then the application — and the first level that knows the key takes it;
//! 4. while a text field has the focus, only the entries marked `while_typing` are read.

use egui::{Context, Event, ImeEvent, Key, Modifiers};

use crate::action::{Action, AppAction};
use crate::state::Area;

/// How a binding reads its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// By position, as a W3C `code`: on AZERTY, `Ctrl+1` is the key that types « & ».
    Position,
    /// By the character it types, as the canvas's keys are.
    Character,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub key: Key,
    pub modifiers: Modifiers,
    pub read: Read,
    /// Read even while a text field has the focus.
    pub while_typing: bool,
    pub action: Action,
}

/// Every area's key, `Ctrl` and its rank in the switch, read by position.
pub static APPLICATION: [Binding; 4] = [
    area(Key::Num1, Area::Notes),
    area(Key::Num2, Area::Tools),
    area(Key::Num3, Area::Http),
    // The window's own: the global shortcut, from another application, joins in P9.
    Binding {
        key: Key::P,
        modifiers: Modifiers::COMMAND.plus(Modifiers::ALT),
        read: Read::Position,
        while_typing: true,
        action: Action::Global(crate::action::GlobalAction::PaletteHere),
    },
];

/// The notes area's, read while it is on screen.
pub static NOTES: [Binding; 1] = [Binding {
    key: Key::K,
    modifiers: Modifiers::COMMAND,
    read: Read::Character,
    while_typing: true,
    action: Action::Notes(crate::notes::Action::FocusSearch),
}];

/// The HTTP area's, read while it is on screen: the rail is the library's, and so is its key.
pub static HTTP: [Binding; 2] = [
    Binding {
        key: Key::B,
        modifiers: Modifiers::COMMAND,
        read: Read::Character,
        while_typing: true,
        action: Action::App(AppAction::ToggleRail),
    },
    Binding {
        key: Key::S,
        modifiers: Modifiers::COMMAND,
        read: Read::Character,
        while_typing: true,
        action: Action::Http(crate::http::Action::SaveActive),
    },
];

const fn area(key: Key, area: Area) -> Binding {
    Binding {
        key,
        modifiers: Modifiers::COMMAND,
        read: Read::Position,
        while_typing: true,
        action: Action::App(AppAction::ShowArea(area)),
    }
}

/// The first binding of `levels`, top level first, that a key pressed this frame and left by
/// the widgets matches. The key is consumed.
pub fn dispatch(ctx: &Context, levels: &[&[Binding]]) -> Option<Action> {
    if composing(ctx) {
        return None;
    }
    // A text field asks for the IME while it has the focus; a focused button asks for nothing.
    let typing = ctx.output(|output| output.ime.is_some());
    ctx.input_mut(|input| {
        let (at, action) = input.events.iter().enumerate().find_map(|(at, event)| {
            let Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return None;
            };
            levels
                .iter()
                .flat_map(|level| level.iter())
                .filter(|binding| binding.while_typing || !typing)
                .find(|binding| {
                    let pressed = match binding.read {
                        Read::Position => physical_key.unwrap_or(*key),
                        Read::Character => *key,
                    };
                    pressed == binding.key && modifiers.matches_exact(binding.modifiers)
                })
                .map(|binding| (at, binding.action.clone()))
        })?;
        input.events.remove(at);
        Some(action)
    })
}

/// Whether an input method is composing: its keys are the composition's.
fn composing(ctx: &Context) -> bool {
    let id = egui::Id::unique("keys.composing");
    let mut composing = ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    ctx.input(|input| {
        for event in &input.events {
            if let Event::Ime(ime) = event {
                composing = matches!(ime, ImeEvent::Preedit { text, .. } if !text.is_empty());
            }
        }
    });
    ctx.data_mut(|data| data.insert_temp(id, composing));
    composing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::GlobalAction;

    fn key(key: Key, physical: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: Some(physical),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn escape(action: GlobalAction) -> Binding {
        Binding {
            key: Key::Escape,
            modifiers: Modifiers::NONE,
            read: Read::Character,
            while_typing: false,
            action: Action::Global(action),
        }
    }

    /// One frame: the events given, an optional focused field, then the levels read.
    struct Frame {
        events: Vec<Event>,
        field: bool,
        modal: bool,
        heard: Option<Action>,
    }

    fn run(events: Vec<Event>, field: bool, modal: bool) -> Option<Action> {
        let mut harness = crate::testing::harness().build_ui_state(
            |ui, frame: &mut Frame| {
                if frame.field {
                    let mut text = String::new();
                    let label = ui.label("Search");
                    ui.text_edit_singleline(&mut text)
                        .labelled_by(label.id)
                        .request_focus();
                }
                for event in frame.events.drain(..) {
                    ui.ctx().input_mut(|input| input.events.push(event));
                }
                let modal = [escape(GlobalAction::Quit)];
                let area = [escape(GlobalAction::NewNote)];
                let levels: Vec<&[Binding]> = if frame.modal {
                    vec![&modal, &area, &APPLICATION]
                } else {
                    vec![&area, &APPLICATION]
                };
                frame.heard = frame.heard.take().or(dispatch(ui.ctx(), &levels));
            },
            Frame {
                events: Vec::new(),
                field,
                modal,
                heard: None,
            },
        );
        harness.run();
        harness.state_mut().events = events;
        harness.step();
        harness.state().heard.clone()
    }

    #[test]
    fn escape_goes_to_the_modal_before_the_area() {
        let pressed = || vec![key(Key::Escape, Key::Escape, Modifiers::NONE)];

        assert_eq!(
            run(pressed(), false, true),
            Some(Action::Global(GlobalAction::Quit))
        );
        assert_eq!(
            run(pressed(), false, false),
            Some(Action::Global(GlobalAction::NewNote))
        );
    }

    #[test]
    fn an_area_key_is_read_by_position_whatever_the_layout_types() {
        // AZERTY: the key at the place of « 1 » types « & », which egui names after its place.
        let ampersand = key(Key::Num1, Key::Num1, Modifiers::COMMAND);

        assert_eq!(
            run(vec![ampersand], false, false),
            Some(Action::App(AppAction::ShowArea(Area::Notes)))
        );
    }

    #[test]
    fn while_typing_only_the_marked_entries_are_read() {
        let escape = || vec![key(Key::Escape, Key::Escape, Modifiers::NONE)];
        let tools = || vec![key(Key::Num2, Key::Num2, Modifiers::COMMAND)];

        assert_eq!(run(escape(), true, false), None);
        assert_eq!(
            run(tools(), true, false),
            Some(Action::App(AppAction::ShowArea(Area::Tools)))
        );
    }

    #[test]
    fn an_extra_modifier_is_another_key() {
        let shifted = key(Key::Num1, Key::Num1, Modifiers::COMMAND | Modifiers::SHIFT);

        assert_eq!(run(vec![shifted], false, false), None);
    }

    #[test]
    fn a_composition_keeps_its_keys() {
        let events = vec![
            Event::Ime(ImeEvent::Preedit {
                text: "か".to_string(),
                active_range_chars: None,
            }),
            key(Key::Escape, Key::Escape, Modifiers::NONE),
        ];

        assert_eq!(run(events, false, false), None);
    }

    #[test]
    fn a_key_taken_is_consumed() {
        let ctx = Context::default();
        let mut input = egui::RawInput::default();
        input
            .events
            .push(key(Key::Escape, Key::Escape, Modifiers::NONE));
        let area = [escape(GlobalAction::NewNote)];
        let mut output = ctx.run_ui(input, |ui| {
            assert!(dispatch(ui.ctx(), &[&area]).is_some());
            assert!(dispatch(ui.ctx(), &[&area]).is_none(), "taken once");
        });
        output.textures_delta.clear();
    }
}
