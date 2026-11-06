//! The keys a person can move: which they are, what a keystroke is called, and why one is
//! refused. Two readers, and the difference is not cosmetic: a global key is taken from the whole
//! machine and stored by **position** (the W3C `code`), a canvas key answers only while the
//! canvas has the keyboard and is stored by the **character** on the keycap.

use std::collections::BTreeMap;

use egui::{Key, Modifiers};
use i18n::keys;

/// Where a key acts, which is what decides how it is read and what it may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Canvas,
    Areas,
}

/// An action a key can be moved to: its storage name, its label and the key it ships with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rebindable {
    pub id: &'static str,
    pub label: i18n::Key,
    pub fallback: &'static str,
    pub scope: Scope,
}

const fn rebindable(
    id: &'static str,
    label: i18n::Key,
    fallback: &'static str,
    scope: Scope,
) -> Rebindable {
    Rebindable {
        id,
        label,
        fallback,
        scope,
    }
}

pub const PALETTE: &str = "palette";
pub const CAPTURE: &str = "capture";
pub const NEW_NOTE: &str = "newNote";
pub const RAIL: &str = "areas.rail";

/// In the order the page lists them.
pub const ACTIONS: [Rebindable; 14] = [
    rebindable(
        PALETTE,
        keys::shortcuts::global::PALETTE,
        "Ctrl+Alt+P",
        Scope::Global,
    ),
    rebindable(
        CAPTURE,
        keys::shortcuts::global::CAPTURE,
        "Ctrl+Alt+V",
        Scope::Global,
    ),
    rebindable(
        NEW_NOTE,
        keys::shortcuts::global::NEW_NOTE,
        "Ctrl+Alt+N",
        Scope::Global,
    ),
    rebindable(
        "canvas.newSnippet",
        keys::shortcuts::canvas::NEW_SNIPPET,
        "Ctrl+N",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.newNote",
        keys::shortcuts::canvas::NEW_NOTE,
        "Ctrl+Shift+N",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.newChecklist",
        keys::shortcuts::canvas::NEW_CHECKLIST,
        "Ctrl+Shift+L",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.newCalc",
        keys::shortcuts::canvas::NEW_CALC,
        "Ctrl+Shift+M",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.newDrawing",
        keys::shortcuts::canvas::NEW_DRAWING,
        "Ctrl+Shift+D",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.copy",
        keys::shortcuts::canvas::COPY,
        "C",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.pin",
        keys::shortcuts::canvas::PIN,
        "P",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.check",
        keys::shortcuts::canvas::CHECK,
        "X",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.align",
        keys::shortcuts::canvas::ALIGN,
        "A",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.trash",
        keys::shortcuts::canvas::TRASH,
        "Delete",
        Scope::Canvas,
    ),
    rebindable(
        "canvas.undo",
        keys::shortcuts::canvas::UNDO,
        "Ctrl+Z",
        Scope::Canvas,
    ),
];

/// The rail's, apart from the canvas's: it acts in the three areas.
pub const RAIL_ACTION: Rebindable = rebindable(RAIL, keys::areas::RAIL, "Ctrl+B", Scope::Areas);

pub fn every() -> impl Iterator<Item = Rebindable> {
    ACTIONS.into_iter().chain([RAIL_ACTION])
}

/// Each key with the name its keystroke is written under. Its position's W3C `code` reads
/// `Key` + the letter, `Digit` + the figure, or the name itself: `code_of`.
const NAMED: [(Key, &str); 25] = [
    (Key::Space, "Space"),
    (Key::Enter, "Enter"),
    (Key::Backspace, "Backspace"),
    (Key::Delete, "Delete"),
    (Key::Insert, "Insert"),
    (Key::Home, "Home"),
    (Key::End, "End"),
    (Key::PageUp, "PageUp"),
    (Key::PageDown, "PageDown"),
    (Key::ArrowUp, "ArrowUp"),
    (Key::ArrowDown, "ArrowDown"),
    (Key::ArrowLeft, "ArrowLeft"),
    (Key::ArrowRight, "ArrowRight"),
    (Key::Backtick, "Backquote"),
    (Key::Minus, "Minus"),
    (Key::Equals, "Equal"),
    (Key::OpenBracket, "BracketLeft"),
    (Key::CloseBracket, "BracketRight"),
    (Key::Backslash, "Backslash"),
    (Key::Semicolon, "Semicolon"),
    (Key::Quote, "Quote"),
    (Key::Comma, "Comma"),
    (Key::Period, "Period"),
    (Key::Slash, "Slash"),
    (Key::Tab, "Tab"),
];

/// What a keystroke writes for `key`: `P`, `1`, `F5`, `Backquote`. `None` for a key with no name
/// here, which is refused with a reason rather than stored as something it is not.
pub fn name_of(key: Key) -> Option<String> {
    if let Some((_, name)) = NAMED.iter().find(|(named, _)| *named == key) {
        return Some((*name).to_owned());
    }
    let name = key.name();
    let simple = name.len() == 1 && name.chars().all(|it| it.is_ascii_alphanumeric());
    let function = name.starts_with('F') && name[1..].parse::<u8>().is_ok();
    (simple || function).then(|| name.to_owned())
}

/// The key a name in a stored keystroke stands for.
pub fn key_named(name: &str) -> Option<Key> {
    if let Some((key, _)) = NAMED.iter().find(|(_, named)| *named == name) {
        return Some(*key);
    }
    Key::from_name(name).filter(|key| name_of(*key).is_some())
}

/// The W3C `code` of the position `key` stands at.
pub fn code_of(key: Key) -> Option<String> {
    let name = name_of(key)?;
    let first = name.chars().next()?;
    Some(if name.len() == 1 && first.is_ascii_alphabetic() {
        format!("Key{name}")
    } else if name.len() == 1 && first.is_ascii_digit() {
        format!("Digit{name}")
    } else {
        name
    })
}

/// `Ctrl+Alt+P`, the modifiers first. `Ctrl` covers ⌘ too.
fn written(modifiers: Modifiers, name: &str, scope: Scope) -> String {
    let mut parts = Vec::with_capacity(4);
    if modifiers.command || modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if scope == Scope::Global && modifiers.mac_cmd {
        parts.push("Super");
    }
    parts.push(name);
    parts.join("+")
}

/// A keystroke read back: the key, and its modifiers.
pub fn parse(accelerator: &str) -> Option<(Key, Modifiers)> {
    let mut tokens: Vec<&str> = accelerator.split('+').map(str::trim).collect();
    let key = key_named(tokens.pop()?)?;
    let mut modifiers = Modifiers::NONE;
    for token in tokens {
        modifiers |= match token {
            "Ctrl" => Modifiers::COMMAND,
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            "Super" => Modifiers::MAC_CMD,
            _ => return None,
        };
    }
    Some((key, modifiers))
}

/// The arrows move through the grid, Tab crosses the window and Escape leaves everything else.
fn is_reserved(key: Key) -> bool {
    matches!(
        key,
        Key::Tab
            | Key::Escape
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::Space
            | Key::Plus
    )
}

/// Why a keystroke was not taken. A reason and not a boolean: the page says which it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A global key without a modifier would swallow that key in every application.
    NeedsModifier,
    /// The navigation's.
    Reserved,
    /// A key this application cannot name, so could never read back.
    Unknown,
    Taken {
        accelerator: String,
        by: Rebindable,
    },
}

/// What a press means for `action`: a keystroke, or a refusal. `key` is the character on the
/// keycap, `physical` its position: on AZERTY, `KeyA` is the key labelled `Q`.
pub fn capture(
    action: Rebindable,
    (key, physical): (Key, Option<Key>),
    modifiers: Modifiers,
    bindings: &Bindings,
) -> Result<String, Refused> {
    let pressed = if action.scope == Scope::Global {
        physical.unwrap_or(key)
    } else {
        key
    };
    if action.scope == Scope::Global && modifiers.is_none() {
        return Err(Refused::NeedsModifier);
    }
    if action.scope != Scope::Global && is_reserved(key) {
        return Err(Refused::Reserved);
    }
    let name = name_of(pressed).ok_or(Refused::Unknown)?;
    let accelerator = written(modifiers, &name, action.scope);
    let holder = every().find(|other| other.id != action.id && bindings.of(*other) == accelerator);
    match holder {
        Some(by) => Err(Refused::Taken { accelerator, by }),
        None => Ok(accelerator),
    }
}

/// The keys moved from where they shipped: stored one per action, the rest being the defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings(BTreeMap<&'static str, String>);

impl Bindings {
    pub fn of(&self, action: Rebindable) -> &str {
        self.0
            .get(action.id)
            .map_or(action.fallback, String::as_str)
    }

    pub fn is_default(&self, action: Rebindable) -> bool {
        self.of(action) == action.fallback
    }

    pub fn set(&mut self, action: Rebindable, accelerator: String) {
        if accelerator == action.fallback {
            self.0.remove(action.id);
        } else {
            self.0.insert(action.id, accelerator);
        }
    }

    pub fn reset(&mut self, action: Rebindable) {
        self.0.remove(action.id);
    }

    /// The key an action answers to, read back: its default when what is stored does not read.
    pub fn keystroke(&self, action: Rebindable) -> Option<(Key, Modifiers)> {
        parse(self.of(action)).or_else(|| parse(action.fallback))
    }

    /// Every keystroke claimed twice: what no field can refuse — a file edited by hand, a default
    /// that lands on a key someone had already moved something onto.
    pub fn conflicts(&self) -> Vec<(String, Vec<Rebindable>)> {
        let mut claims: BTreeMap<String, Vec<Rebindable>> = BTreeMap::new();
        for action in every() {
            claims
                .entry(self.of(action).to_owned())
                .or_default()
                .push(action);
        }
        claims
            .into_iter()
            .filter(|(_, actions)| actions.len() > 1)
            .collect()
    }
}

/// Where a key is stored: the three global ones as the native side takes them, the rest one each.
pub fn preference_key(action: Rebindable) -> String {
    match action.scope {
        Scope::Global => format!("devnotes.{}Shortcut", action.id),
        _ => format!("devnotes.shortcut.{}", action.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: Rebindable = ACTIONS[9];
    const CHECK: Rebindable = ACTIONS[10];
    const GLOBAL: Rebindable = ACTIONS[0];

    fn modified() -> Modifiers {
        Modifiers::COMMAND | Modifiers::ALT
    }

    #[test]
    fn every_default_reads_back_and_none_is_claimed_twice() {
        for action in every() {
            assert!(parse(action.fallback).is_some(), "{}", action.fallback);
        }
        assert!(Bindings::default().conflicts().is_empty());
    }

    #[test]
    fn every_key_of_the_table_has_its_position_and_its_name_back() {
        let letters = ('A'..='Z').map(String::from);
        let digits = ('0'..='9').map(String::from);
        let functions = (1..=20).map(|at| format!("F{at}"));
        let named = NAMED.iter().map(|(_, name)| (*name).to_owned());
        for name in letters.chain(digits).chain(functions).chain(named) {
            let key = key_named(&name).unwrap_or_else(|| panic!("{name} has no key"));
            assert_eq!(name_of(key).as_deref(), Some(name.as_str()));
            assert!(code_of(key).is_some(), "{name}");
        }
        assert_eq!(code_of(Key::A).as_deref(), Some("KeyA"));
        assert_eq!(code_of(Key::Num1).as_deref(), Some("Digit1"));
        assert_eq!(code_of(Key::Backtick).as_deref(), Some("Backquote"));
    }

    #[test]
    fn a_global_key_is_read_by_position_on_azerty_and_on_qwerty() {
        let bindings = Bindings::default();
        // The key labelled Q on AZERTY is where A stands on QWERTY.
        let azerty = capture(GLOBAL, (Key::Q, Some(Key::A)), modified(), &bindings);
        let qwerty = capture(GLOBAL, (Key::A, Some(Key::A)), modified(), &bindings);

        assert_eq!(azerty.as_deref(), Ok("Ctrl+Alt+A"));
        assert_eq!(azerty, qwerty);
    }

    #[test]
    fn a_canvas_key_is_read_by_the_character_on_the_keycap() {
        let bindings = Bindings::default();

        let azerty = capture(PIN, (Key::Q, Some(Key::A)), Modifiers::NONE, &bindings);

        assert_eq!(azerty.as_deref(), Ok("Q"));
    }

    #[test]
    fn a_global_key_refuses_no_modifier_and_a_canvas_key_takes_a_bare_letter() {
        let bindings = Bindings::default();

        assert_eq!(
            capture(GLOBAL, (Key::J, Some(Key::J)), Modifiers::NONE, &bindings),
            Err(Refused::NeedsModifier)
        );
        assert_eq!(
            capture(PIN, (Key::J, Some(Key::J)), Modifiers::NONE, &bindings).as_deref(),
            Ok("J")
        );
    }

    #[test]
    fn the_navigation_keeps_its_keys() {
        let bindings = Bindings::default();

        for key in [Key::ArrowLeft, Key::Tab, Key::Escape, Key::Space] {
            assert_eq!(
                capture(PIN, (key, Some(key)), Modifiers::NONE, &bindings),
                Err(Refused::Reserved)
            );
        }
    }

    #[test]
    fn a_key_held_by_another_action_is_refused_by_name() {
        let bindings = Bindings::default();

        let refused = capture(CHECK, (Key::C, Some(Key::C)), Modifiers::NONE, &bindings);

        assert_eq!(
            refused,
            Err(Refused::Taken {
                accelerator: "C".to_owned(),
                by: ACTIONS[8],
            })
        );
    }

    #[test]
    fn a_key_with_no_name_here_is_refused_for_that_reason() {
        let bindings = Bindings::default();

        let refused = capture(GLOBAL, (Key::Plus, Some(Key::Plus)), modified(), &bindings);

        assert_eq!(refused, Err(Refused::Unknown));
    }

    #[test]
    fn a_moved_key_is_stored_and_a_key_put_back_is_not() {
        let mut bindings = Bindings::default();

        bindings.set(PIN, "J".to_owned());
        assert_eq!(bindings.of(PIN), "J");
        assert_eq!(bindings.keystroke(PIN), Some((Key::J, Modifiers::NONE)));
        assert!(!bindings.is_default(PIN));

        bindings.set(PIN, "P".to_owned());
        assert_eq!(bindings, Bindings::default());
    }

    #[test]
    fn two_actions_on_one_key_are_found() {
        let mut bindings = Bindings::default();

        bindings.set(PIN, "C".to_owned());

        let conflicts = bindings.conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "C");
        assert_eq!(conflicts[0].1.len(), 2);
    }

    #[test]
    fn global_keys_keep_the_reference_storage_names() {
        assert_eq!(preference_key(GLOBAL), "devnotes.paletteShortcut");
        assert_eq!(preference_key(PIN), "devnotes.shortcut.canvas.pin");
    }
}
