//! The formatting shortcuts, written in the reference's notation (`Mod` is Ctrl, ⌘ on a Mac) as
//! `golden/rich-keys` records them, and read from it by a test rather than copied by hand. A
//! digit is read where the key sits, as the area keys are: on AZERTY, Ctrl+Shift+7 is the key
//! that prints 7 unshifted elsewhere.

use egui::{Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Bold,
    Italic,
    Strike,
    Code,
    Paragraph,
    Heading(u8),
    Bullet,
    Ordered,
    Tasks,
    Quote,
    CodeBlock,
    HardBreak,
    Undo,
    Redo,
    SelectAll,
}

pub(super) const SHORTCUTS: [(&str, Command); 21] = [
    ("Mod-b", Command::Bold),
    ("Mod-i", Command::Italic),
    ("Mod-Shift-s", Command::Strike),
    ("Mod-e", Command::Code),
    ("Mod-Alt-0", Command::Paragraph),
    ("Mod-Alt-1", Command::Heading(1)),
    ("Mod-Alt-2", Command::Heading(2)),
    ("Mod-Alt-3", Command::Heading(3)),
    ("Mod-Alt-4", Command::Heading(4)),
    ("Mod-Alt-5", Command::Heading(5)),
    ("Mod-Alt-6", Command::Heading(6)),
    ("Mod-Shift-7", Command::Ordered),
    ("Mod-Shift-8", Command::Bullet),
    ("Mod-Shift-9", Command::Tasks),
    ("Mod-Shift-b", Command::Quote),
    ("Mod-Alt-c", Command::CodeBlock),
    ("Shift-Enter", Command::HardBreak),
    ("Mod-Enter", Command::HardBreak),
    ("Mod-z", Command::Undo),
    ("Shift-Mod-z", Command::Redo),
    ("Mod-y", Command::Redo),
];

/// `Mod-a` is the editor's own, with the other keys of its keymap.
pub(super) const SELECT_ALL: (&str, Command) = ("Mod-a", Command::SelectAll);

/// The command a key pressed asks, if any.
pub(super) fn find(key: Key, physical: Option<Key>, modifiers: Modifiers) -> Option<Command> {
    SHORTCUTS
        .iter()
        .chain(std::iter::once(&SELECT_ALL))
        .find(|(notation, _)| matches(notation, key, physical, modifiers))
        .map(|(_, command)| *command)
}

fn matches(notation: &str, key: Key, physical: Option<Key>, modifiers: Modifiers) -> bool {
    let parts: Vec<&str> = notation.split('-').collect();
    let Some((name, held)) = parts.split_last() else {
        return false;
    };
    let wants = |modifier| held.contains(&modifier);
    if modifiers.command != wants("Mod")
        || modifiers.shift != wants("Shift")
        || modifiers.alt != wants("Alt")
    {
        return false;
    }
    let pressed = if name.chars().all(|glyph| glyph.is_ascii_digit()) {
        physical.unwrap_or(key)
    } else {
        key
    };
    Key::from_name(&name.to_uppercase()).or_else(|| Key::from_name(name)) == Some(pressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shortcut as the corpus writes it, its modifiers in one order and its key lower-case.
    fn normal(notation: &str) -> String {
        let mut parts: Vec<String> = notation.split('-').map(str::to_lowercase).collect();
        let key = parts.pop().unwrap_or_default();
        parts.sort();
        parts.push(key);
        parts.join("-")
    }

    #[test]
    fn every_formatting_shortcut_of_the_reference_is_ours() {
        let corpus: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../../../golden/rich-keys/cases.json"
        ))
        .unwrap();
        let ours: Vec<String> = SHORTCUTS
            .iter()
            .chain(std::iter::once(&SELECT_ALL))
            .map(|(notation, _)| normal(notation))
            .collect();
        let formatting = [
            "paragraph",
            "bold",
            "italic",
            "strike",
            "code",
            "heading",
            "bulletList",
            "orderedList",
            "taskList",
            "blockquote",
            "codeBlock",
            "hardBreak",
            "undoRedo",
        ];
        for case in &corpus {
            let name = case["input"].as_str().unwrap_or_default();
            if !formatting.contains(&name) {
                continue;
            }
            for shortcut in case["output"]["shortcuts"].as_array().unwrap() {
                let shortcut = shortcut.as_str().unwrap();
                // The block's own keys are the editor's (Enter, Tab, the arrows); `я` is the
                // Cyrillic letter on Z's key, which egui already reads as Z.
                let held = shortcut.contains("Mod") || shortcut.contains("Shift-Enter");
                if !held || shortcut.contains('я') {
                    continue;
                }
                assert!(
                    ours.contains(&normal(shortcut)),
                    "{name}: {shortcut} is not ours"
                );
            }
        }
    }

    #[test]
    fn a_digit_is_read_where_the_key_sits_and_a_letter_as_it_reads() {
        let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        assert_eq!(
            find(Key::Num8, Some(Key::Num8), ctrl_shift),
            Some(Command::Bullet)
        );
        // AZERTY: the key that prints 8 unshifted on QWERTY reads `_` with no digit.
        assert_eq!(
            find(Key::Minus, Some(Key::Num8), ctrl_shift),
            Some(Command::Bullet)
        );
        assert_eq!(find(Key::B, None, Modifiers::COMMAND), Some(Command::Bold));
        assert_eq!(find(Key::B, None, ctrl_shift), Some(Command::Quote));
        assert_eq!(find(Key::B, None, Modifiers::NONE), None);
    }
}
