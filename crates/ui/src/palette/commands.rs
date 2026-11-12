//! What `>` lists. A command names its key from the table that binds it — the canvas's for a
//! new note, the areas' for an area — so the palette never shows a key the application lost.

use engine::notes::kind::NoteKind;
use i18n::{Key, keys};

use crate::state::Area;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Create(NoteKind),
    Request,
    /// Needs a space to put it in: across every space it is not offered.
    Folder,
    Area(Area),
    Preferences,
    Shortcuts,
}

pub const COMMANDS: [Command; 12] = [
    Command::Create(NoteKind::Snippet),
    Command::Create(NoteKind::Note),
    Command::Create(NoteKind::Checklist),
    Command::Create(NoteKind::Calc),
    Command::Create(NoteKind::Drawing),
    Command::Request,
    Command::Folder,
    Command::Area(Area::Notes),
    Command::Area(Area::Tools),
    Command::Area(Area::Http),
    Command::Preferences,
    Command::Shortcuts,
];

impl Command {
    pub fn label(self) -> Key {
        match self {
            Self::Create(NoteKind::Snippet) => keys::shortcuts::canvas::NEW_SNIPPET,
            Self::Create(NoteKind::Note) => keys::shortcuts::canvas::NEW_NOTE,
            Self::Create(NoteKind::Checklist) => keys::shortcuts::canvas::NEW_CHECKLIST,
            Self::Create(NoteKind::Calc) => keys::shortcuts::canvas::NEW_CALC,
            Self::Create(NoteKind::Drawing) => keys::shortcuts::canvas::NEW_DRAWING,
            Self::Request => keys::palette::commands::NEW_REQUEST,
            Self::Folder => keys::palette::commands::NEW_FOLDER,
            Self::Preferences => keys::palette::commands::PREFERENCES,
            Self::Shortcuts => keys::palette::commands::SHORTCUTS,
            Self::Area(Area::Notes) => keys::areas::go::NOTES,
            Self::Area(Area::Tools) => keys::areas::go::TOOLS,
            Self::Area(Area::Http) => keys::areas::go::HTTP,
        }
    }

    /// Its keys, one cap each.
    pub fn keys(self) -> Vec<&'static str> {
        let shortcut = match self {
            Self::Create(kind) => crate::notes::header::shortcut(kind),
            Self::Request | Self::Folder | Self::Preferences | Self::Shortcuts => {
                return Vec::new();
            }
            Self::Area(Area::Notes) => "Ctrl+1",
            Self::Area(Area::Tools) => "Ctrl+2",
            Self::Area(Area::Http) => "Ctrl+3",
        };
        shortcut.split('+').collect()
    }

    /// Its place in the table: what a row is told apart by.
    pub fn id(self) -> String {
        match self {
            Self::Create(kind) => format!("create-{}", kind.as_str()),
            Self::Request => "request".to_owned(),
            Self::Folder => "folder".to_owned(),
            Self::Preferences => "preferences".to_owned(),
            Self::Shortcuts => "shortcuts".to_owned(),
            Self::Area(area) => format!("area-{}", area_name(area)),
        }
    }
}

fn area_name(area: Area) -> &'static str {
    match area {
        Area::Notes => "notes",
        Area::Tools => "tools",
        Area::Http => "http",
    }
}
