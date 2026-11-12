//! What the help says, as data: the guide's chapters and their schematics, and the shortcut
//! sheet. A figure is decorative — each chapter says in words what it shows — and is drawn from
//! a handful of shapes in the reference's own 320 × 120 space.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use i18n::{Key, keys};
use serde::Deserialize;

use super::shortcuts::{self, Bindings};

/// In the order they are met. One list: the canvas and the board link straight into a chapter.
pub const CHAPTERS: [&str; 14] = [
    "notes",
    "spaces",
    "folders",
    "organise",
    "fields",
    "checklists",
    "explorer",
    "history",
    "palette",
    "attachments",
    "trash",
    "transfer",
    "tools",
    "http",
];

pub const NOTES: usize = 0;
pub const FOLDERS: usize = 2;

/// The three inks and their stroked twins, as the reference's classes name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ink {
    Panel,
    Line,
    Ink,
    Accent,
    AccentLine,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Shape {
    pub class: Ink,
    pub opacity: Option<f32>,
    /// x, y, width, height, corner.
    pub rect: Option<[f32; 5]>,
    /// x, y, radius.
    pub circle: Option<[f32; 3]>,
    pub lines: Option<Vec<Vec<[f32; 2]>>>,
}

static FIGURES: LazyLock<BTreeMap<String, Vec<Shape>>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("guide_figures.json"))
        .expect("the figures are compiled in, and held by a test")
});

pub fn figure(chapter: &str) -> &'static [Shape] {
    FIGURES.get(chapter).map_or(&[], Vec::as_slice)
}

/// The chapter's title and its body, by the keys the translations give them.
pub fn words(chapter: &str) -> Option<(Key, Key)> {
    let key = |part| Key::find(&format!("gettingStarted.chapters.{chapter}.{part}"));
    Some((key("title")?, key("body")?))
}

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
    fn every_chapter_has_its_words_and_a_figure() {
        for chapter in CHAPTERS {
            assert!(words(chapter).is_some(), "{chapter}");
            assert!(!figure(chapter).is_empty(), "{chapter}");
        }
    }

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
