//! What the list shows for what is typed: groups of options in a fixed order, a heading and a
//! count only when the list holds more than one kind.

use engine::notes::model::DisplayNote;
use i18n::{Key, keys};

use super::commands::{COMMANDS, Command};
use crate::api;
use crate::state::AppState;
use crate::state::palette::{Chip, MAX_NOTES, Mode, SCOPES, Scope};

/// Several kinds share the screen and each gives way; one alone has all of it.
const SHARED: (usize, usize) = (5, 3);
const ALONE: usize = MAX_NOTES;
/// The empty field's recent notes.
const RECENT: usize = 4;
const MAX_TAGS: usize = 8;

/// The words `@` is read by, in either language whatever the one on screen, accents aside.
fn scope_words(scope: Scope) -> &'static [&'static str] {
    match scope {
        Scope::Notes => &["notes", "snippets"],
        Scope::Spaces => &["espaces", "spaces"],
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Note(Box<DisplayNote>),
    Space {
        id: String,
        name: String,
    },
    Command(Command),
    /// Chosen, it narrows the search.
    Chip {
        chip: Chip,
        count: Option<u32>,
    },
    /// What was typed while looking for a note is its name.
    Create(String),
}

/// What choosing a row asks, once the palette has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chosen {
    Note(String),
    Space(String),
    Command(Command),
    Create(String),
}

impl Item {
    /// `None` for a chip, which narrows the search instead.
    pub fn chosen(&self) -> Option<Chosen> {
        match self {
            Self::Note(note) => Some(Chosen::Note(note.id.clone())),
            Self::Space { id, .. } => Some(Chosen::Space(id.clone())),
            Self::Command(command) => Some(Chosen::Command(*command)),
            Self::Create(title) => Some(Chosen::Create(title.clone())),
            Self::Chip { .. } => None,
        }
    }

    /// What a row is told apart by: the highlight goes back to the first when these change.
    pub fn key(&self) -> String {
        match self {
            Self::Note(note) => format!("note-{}", note.id),
            Self::Space { id, .. } => format!("space-{id}"),
            Self::Command(command) => format!("command-{}", command.id()),
            Self::Chip { chip, .. } => format!("chip-{chip:?}"),
            Self::Create(_) => "create".to_owned(),
        }
    }
}

pub struct Group {
    pub heading: Option<Key>,
    pub count: Option<usize>,
    pub items: Vec<Item>,
}

pub fn groups(state: &AppState) -> Vec<Group> {
    let palette = &state.palette;
    match palette.mode() {
        Mode::Scopes(word) => scope_groups(state, &api::notes::fold(&word)),
        Mode::Tags(word) => tag_groups(state, &api::notes::fold(&word)),
        Mode::Commands(term) => command_groups(state, &api::notes::fold(&term)),
        Mode::Search(term) => {
            let notes = found_notes(state);
            if palette.recent() {
                let items: Vec<Item> = notes.into_iter().take(RECENT).collect();
                return if items.is_empty() {
                    Vec::new()
                } else {
                    vec![Group {
                        heading: Some(keys::palette::groups::RECENT),
                        count: None,
                        items,
                    }]
                };
            }
            search_groups(state, notes, &term)
        }
    }
}

fn found_notes(state: &AppState) -> Vec<Item> {
    let Some(view) = state.palette.notes() else {
        return Vec::new();
    };
    view.cards()
        .take(MAX_NOTES)
        .map(|card| Item::Note(Box::new(view.display(card))))
        .collect()
}

fn search_groups(state: &AppState, notes: Vec<Item>, term: &str) -> Vec<Group> {
    let palette = &state.palette;
    let wanted = api::notes::fold(term);
    let spaces: Vec<Item> = if palette.searched(Scope::Spaces) {
        state
            .spaces
            .spaces()
            .iter()
            .filter(|space| api::notes::fold(&space.name).contains(&wanted))
            .map(|space| Item::Space {
                id: space.id.clone(),
                name: space.name.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let alone = SCOPES
        .iter()
        .filter(|scope| palette.searched(**scope))
        .count()
        == 1;
    let found = [
        (keys::palette::groups::NOTES, notes, SHARED.0),
        (keys::palette::groups::SPACES, spaces, SHARED.1),
    ];
    let several = found
        .iter()
        .filter(|(_, items, _)| !items.is_empty())
        .count()
        > 1;
    let mut groups: Vec<Group> = found
        .into_iter()
        .filter(|(_, items, _)| !items.is_empty())
        .map(|(heading, items, shared)| {
            let items: Vec<Item> = items
                .into_iter()
                .take(if alone { ALONE } else { shared })
                .collect();
            Group {
                heading: several.then_some(heading),
                count: several.then_some(items.len()),
                items,
            }
        })
        .collect();
    // After the results: finding something keeps first place.
    if !term.is_empty() && palette.searched(Scope::Notes) {
        groups.push(Group {
            heading: None,
            count: None,
            items: vec![Item::Create(term.to_owned())],
        });
    }
    groups
}

fn scope_groups(state: &AppState, word: &str) -> Vec<Group> {
    let chosen = state.palette.chips();
    let scopes: Vec<Item> = SCOPES
        .iter()
        .filter(|scope| !chosen.contains(&Chip::Scope(**scope)))
        .filter(|scope| {
            scope_words(**scope)
                .iter()
                .any(|each| each.starts_with(word))
        })
        .map(|scope| Item::Chip {
            chip: Chip::Scope(*scope),
            count: None,
        })
        .collect();
    let spaces: Vec<Item> = state
        .spaces
        .spaces()
        .iter()
        .filter(|space| api::notes::fold(&space.name).starts_with(word))
        .map(|space| Item::Chip {
            chip: Chip::Space {
                id: space.id.clone(),
                name: space.name.clone(),
            },
            count: None,
        })
        .collect();
    [
        (keys::palette::groups::SCOPES, scopes),
        (keys::palette::groups::SPACE_NOTES, spaces),
    ]
    .into_iter()
    .filter(|(_, items)| !items.is_empty())
    .map(|(heading, items)| Group {
        heading: Some(heading),
        count: None,
        items,
    })
    .collect()
}

/// The most used first.
fn tag_groups(state: &AppState, word: &str) -> Vec<Group> {
    let chosen = state.palette.chips();
    let mut tags: Vec<_> = state
        .palette
        .tags()
        .iter()
        .filter(|usage| !chosen.contains(&Chip::Tag(usage.tag.clone())))
        .filter(|usage| api::notes::fold(&usage.tag).starts_with(word))
        .collect();
    tags.sort_by_key(|usage| std::cmp::Reverse(usage.note_count));
    let items: Vec<Item> = tags
        .into_iter()
        .take(MAX_TAGS)
        .map(|usage| Item::Chip {
            chip: Chip::Tag(usage.tag.clone()),
            count: Some(usage.note_count),
        })
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    vec![Group {
        heading: Some(keys::palette::groups::TAGS),
        count: None,
        items,
    }]
}

fn command_groups(state: &AppState, term: &str) -> Vec<Group> {
    let items: Vec<Item> = COMMANDS
        .iter()
        .filter(|command| **command != Command::Folder || state.spaces.active_space().is_some())
        .filter(|command| {
            let label = i18n::translate(state.language, command.label(), &[]);
            api::notes::fold(&label).contains(term)
        })
        .map(|command| Item::Command(*command))
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    vec![Group {
        heading: Some(keys::palette::groups::COMMANDS),
        count: Some(items.len()),
        items,
    }]
}

/// Said in place of a list, above the offer to create: what was looked for, so a typo shows.
pub fn empty(state: &AppState, groups: &[Group]) -> Option<i18n::Message> {
    let found = groups
        .iter()
        .flat_map(|group| &group.items)
        .any(|item| !matches!(item, Item::Create(_)));
    if found {
        return None;
    }
    let message = match state.palette.mode() {
        Mode::Scopes(word) => {
            i18n::Message::new(keys::palette::empty::SCOPES).with("word", word.as_str())
        }
        Mode::Tags(word) => {
            i18n::Message::new(keys::palette::empty::TAGS).with("word", word.as_str())
        }
        Mode::Commands(term) => {
            i18n::Message::new(keys::palette::empty::COMMANDS).with("term", term.as_str())
        }
        Mode::Search(term) if !term.is_empty() => {
            i18n::Message::new(keys::palette::empty::SEARCH).with("term", term.as_str())
        }
        Mode::Search(_) => match state.palette.chips() {
            [Chip::Scope(Scope::Notes)] => i18n::Message::new(keys::palette::empty::none::NOTES),
            [Chip::Scope(Scope::Spaces)] => i18n::Message::new(keys::palette::empty::none::SPACES),
            _ => i18n::Message::new(keys::palette::empty::NOTHING),
        },
    };
    Some(message)
}
