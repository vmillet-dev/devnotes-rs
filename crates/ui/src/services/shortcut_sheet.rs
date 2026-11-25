//! The shortcut sheet, as data: every key by where it is live, and what it does.

use std::sync::LazyLock;

use i18n::{Key, keys};
use serde::Deserialize;

use super::shortcuts::{self, Bindings};

/// A row of the sheet: its keys, or the action whose key is really bound.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SheetRow {
    keys: Vec<String>,
    label: String,
    action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SheetGroup {
    title: String,
    pub rows: Vec<SheetRow>,
}

impl SheetGroup {
    pub fn title(&self) -> Key {
        Key::find(&self.title).unwrap_or(keys::shortcuts::TITLE)
    }
}

impl SheetRow {
    pub fn label(&self) -> Key {
        Key::find(&self.label).unwrap_or(keys::shortcuts::TITLE)
    }

    /// One key per cap: a single string would put the `+` inside them.
    pub fn caps(&self, bindings: &Bindings) -> Vec<String> {
        let moved = self
            .action
            .as_deref()
            .and_then(|id| shortcuts::every().find(|action| action.id == id));
        match moved {
            Some(action) => bindings.of(action).split('+').map(str::to_owned).collect(),
            None => self.keys.clone(),
        }
    }
}

static SHEET: LazyLock<Vec<SheetGroup>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("shortcut_sheet.json"))
        .expect("the sheet is compiled in, and held by a test")
});

pub fn sheet() -> &'static [SheetGroup] {
    &SHEET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_of_the_sheet_says_something() {
        for group in sheet() {
            assert!(Key::find(&group.title).is_some(), "{}", group.title);
            for row in &group.rows {
                assert!(Key::find(&row.label).is_some(), "{}", row.label);
                assert!(!row.caps(&Bindings::default()).is_empty(), "{}", row.label);
            }
        }
    }

    #[test]
    fn the_sheet_shows_the_key_really_bound() {
        let pin = shortcuts::ACTIONS[9];
        let mut bindings = Bindings::default();
        bindings.set(pin, "Ctrl+J".to_owned());

        let caps = sheet()
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.action.as_deref() == Some(pin.id))
            .unwrap()
            .caps(&bindings);

        assert_eq!(caps, ["Ctrl", "J"]);
    }
}
