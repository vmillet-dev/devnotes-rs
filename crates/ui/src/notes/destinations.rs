//! Where notes can be moved: every space, its folders under it, and the last ones used at the
//! top. One picker for a card's menu and for the selection.

use std::collections::HashSet;

use egui::Ui;
use engine::folders::model::{Folder, Lineage};
use engine::spaces::model::Space;
use i18n::{Language, keys};

use super::sidebar::tree::{self, Row};
use crate::theme::icons::Icon;
use crate::widgets::{self, Lead, Place};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Out of its folder, each in its own space.
    NoFolder,
    Space(String),
    Folder(String),
}

/// What is moved, and where it can go.
pub(super) struct Moving<'a> {
    pub(super) spaces: &'a [Space],
    pub(super) folders: &'a [Folder],
    pub(super) recent: &'a [Destination],
    /// Its space, offered first.
    pub(super) home: Option<&'a str>,
    /// Where it is; several are nowhere in particular.
    pub(super) here: Option<Destination>,
    /// It can be taken out of a folder.
    pub(super) filed: bool,
    pub(super) language: Language,
}

/// In a popup: the place picked, the frame it is.
pub(super) fn picker(ui: &mut Ui, moving: &Moving<'_>, author_id: &str) -> Option<Destination> {
    let label = i18n::translate(moving.language, keys::notes::MOVE_TO, &[]);
    let mut offered = offered(ui, moving);
    let places: Vec<Place> = offered
        .iter_mut()
        .filter_map(|(_, place)| place.take())
        .collect();
    let picked = widgets::place_picker(ui, &places, (&label, author_id), moving.language)?;
    let (to, _) = offered.swap_remove(picked);
    (moving.here.as_ref() != Some(&to)).then_some(to)
}

/// `Option` so that the places can be handed over without their destinations.
type Offered = (Destination, Option<Place>);

fn offered(ui: &Ui, moving: &Moving<'_>) -> Vec<Offered> {
    let tree = tree_of(ui, moving);
    let mut offered = Vec::new();
    if moving.filed {
        let name = i18n::translate(moving.language, keys::notes::NO_FOLDER, &[]);
        let place = place(name, Lead::Icon(Icon::X));
        offered.push((Destination::NoFolder, Some(place)));
    }
    for recent in moving.recent {
        let known = tree.iter().find(|(to, _)| to == recent);
        if let Some((to, Some(known))) = known
            && moving.here.as_ref() != Some(to)
        {
            let again = Place {
                lead: Lead::Icon(Icon::Clock),
                within: known.within.clone(),
                again: true,
                ..place(known.name.clone(), Lead::None)
            };
            offered.push((to.clone(), Some(again)));
        }
    }
    offered.extend(tree);
    offered
}

fn tree_of(ui: &Ui, moving: &Moving<'_>) -> Vec<Offered> {
    let mut spaces = moving.spaces.to_vec();
    spaces.sort_by_key(|space| Some(space.id.as_str()) != moving.home);
    let lineage = Lineage::of(moving.folders);
    let name_of = |id: &String| moving.folders.iter().find(|folder| folder.id == *id);
    let mut within = String::new();
    let mut offered = Vec::new();
    for row in tree::rows(&spaces, moving.folders, None, &HashSet::new()) {
        let (to, place) = match row {
            Row::Space { space, .. } => {
                within.clone_from(&space.name);
                let place = Place {
                    strong: true,
                    ..place(space.name.clone(), Lead::None)
                };
                (Destination::Space(space.id.clone()), place)
            }
            Row::Folder { folder, depth, .. } => {
                let path = lineage.path(&folder.id);
                let parents = path
                    .iter()
                    .filter(|id| **id != folder.id)
                    .filter_map(name_of);
                let parents = parents.map(|parent| parent.name.as_str());
                let within: Vec<&str> = std::iter::once(within.as_str()).chain(parents).collect();
                let place = Place {
                    within: within.join(" › "),
                    depth: depth + 1,
                    ..place(
                        folder.name.clone(),
                        Lead::Dot(super::ui::folder_hue(ui, folder.colour)),
                    )
                };
                (Destination::Folder(folder.id.clone()), place)
            }
            Row::All | Row::NewFolder { .. } | Row::NoFolders => continue,
        };
        let here = moving.here.as_ref() == Some(&to);
        offered.push((to, Some(Place { here, ..place })));
    }
    offered
}

fn place(name: String, lead: Lead) -> Place {
    Place {
        lead,
        name,
        within: String::new(),
        depth: 0,
        strong: false,
        here: false,
        again: false,
    }
}
