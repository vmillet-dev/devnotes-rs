//! A folder cuts a space into regions. Its name and colour travel with an export; where its
//! zone sits on the board never enters this file (see `folders/board`).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use crate::error::ValidationError;

closed_enum! {
    /// Assigned on creation, changed from the zone menu after: drawing a folder stays one
    /// gesture. The five are the theme's accents, each with a light-theme twin.
    pub enum FolderColour {
        #[default]
        Blue = "blue",
        Amber = "amber",
        Purple = "purple",
        Green = "green",
        Red = "red",
    }
}

impl FolderColour {
    /// Rotates through the palette so two folders made back to back differ; counted rather
    /// than random, so a test can assert it and a user predict it.
    pub fn nth(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub space_id: String,
    /// Unique among its siblings, case-insensitively, decided by persistence.
    pub name: String,
    #[serde(default)]
    pub colour: FolderColour,
    pub created_at: DateTime<Utc>,
    /// `None` at the top of its space. `default`: a file from before nesting reads flat.
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderDraft {
    pub space_id: String,
    pub name: String,
    #[serde(default)]
    #[specta(optional)]
    pub parent_id: Option<String>,
}

/// What a card shows, resolved by the back end: the front end never joins an id against
/// a list it happens to hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteFolder {
    pub id: String,
    pub name: String,
    pub colour: FolderColour,
    /// The names from the top of the space down to this one, said on hover.
    pub path: Vec<String>,
}

/// Where a folder was before it moved, the only thing that can put it back: its parent, and
/// its zone's frame on that parent's board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderMove {
    pub folder_id: String,
    #[specta(optional)]
    pub parent_id: Option<String>,
    #[specta(optional)]
    pub frame: Option<crate::folders::board::BoardFrame>,
}

/// Which folder each note left, the only thing that can put a filing back; `None` was loose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteFiling {
    pub note_id: String,
    #[specta(optional)]
    pub folder_id: Option<String>,
}

/// How deep folders nest: a guard against a runaway tree, never a level anyone should reach.
pub const MAX_DEPTH: usize = 8;

/// Where a note sits on the board of one folder — or of the space, `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seat {
    /// In the zone of the root's child that holds its folder, at whatever depth.
    Zone(String),
    /// Filed in the root itself: a card on the root's own ground.
    Free,
    /// Outside the root: not on this board at all.
    Elsewhere,
}

/// One space's folders as a tree, read from ids and parents alone: every rule about nesting is
/// asked here, and no name has to be opened to answer it.
#[derive(Debug, Default)]
pub struct Lineage {
    parents: HashMap<String, Option<String>>,
    children: HashMap<Option<String>, Vec<String>>,
}

impl Lineage {
    /// A parent the list does not hold reads as the top: a folder whose parent is gone is not
    /// lost with it.
    pub fn new(links: impl IntoIterator<Item = (String, Option<String>)>) -> Self {
        let links: Vec<(String, Option<String>)> = links.into_iter().collect();
        let known: HashSet<&str> = links.iter().map(|(id, _)| id.as_str()).collect();

        let mut lineage = Self::default();
        for (id, parent) in &links {
            let parent = parent
                .clone()
                .filter(|parent| known.contains(parent.as_str()));
            lineage
                .children
                .entry(parent.clone())
                .or_default()
                .push(id.clone());
            lineage.parents.insert(id.clone(), parent);
        }
        lineage
    }

    pub fn of(folders: &[Folder]) -> Self {
        Self::new(
            folders
                .iter()
                .map(|folder| (folder.id.clone(), folder.parent_id.clone())),
        )
    }

    pub fn contains(&self, id: &str) -> bool {
        self.parents.contains_key(id)
    }

    pub fn parent(&self, id: &str) -> Option<&str> {
        self.parents.get(id).and_then(Option::as_deref)
    }

    /// In the order the folders were handed over: creation, for the board's zones.
    pub fn children(&self, parent: Option<&str>) -> &[String] {
        self.children
            .get(&parent.map(str::to_string))
            .map_or(&[], Vec::as_slice)
    }

    /// The folder and everything under it, the folder first. A cycle a damaged row could make
    /// is walked once.
    pub fn subtree(&self, id: &str) -> Vec<String> {
        if !self.contains(id) {
            return Vec::new();
        }
        let mut found = vec![id.to_string()];
        let mut seen: HashSet<String> = found.iter().cloned().collect();
        let mut at = 0;
        while let Some(current) = found.get(at).cloned() {
            for child in self.children(Some(&current)) {
                if seen.insert(child.clone()) {
                    found.push(child.clone());
                }
            }
            at += 1;
        }
        found
    }

    /// From the top of the space down to the folder itself.
    pub fn path(&self, id: &str) -> Vec<String> {
        if !self.contains(id) {
            return Vec::new();
        }
        let mut path = vec![id.to_string()];
        let mut current = id;
        while let Some(parent) = self.parent(current) {
            if path.iter().any(|seen| seen == parent) {
                break;
            }
            path.push(parent.to_string());
            current = parent;
        }
        path.reverse();
        path
    }

    /// 1 for a folder at the top of its space.
    pub fn depth(&self, id: &str) -> usize {
        self.path(id).len()
    }

    /// How many levels a folder brings with it when it moves: 1 for one with no sub-folder.
    pub fn height(&self, id: &str) -> usize {
        self.subtree(id)
            .iter()
            .map(|below| self.depth(below))
            .max()
            .map_or(0, |deepest| deepest + 1 - self.depth(id))
    }

    /// Where a note of `folder` sits on the board of `root`.
    pub fn seat(&self, root: Option<&str>, folder: Option<&str>) -> Seat {
        if folder == root {
            return Seat::Free;
        }
        let Some(folder) = folder else {
            return Seat::Elsewhere;
        };
        let path = self.path(folder);
        let start = match root {
            None => 0,
            Some(root) => match path.iter().position(|id| id == root) {
                Some(at) => at + 1,
                None => return Seat::Elsewhere,
            },
        };
        path.get(start)
            .map_or(Seat::Elsewhere, |zone| Seat::Zone(zone.clone()))
    }

    /// A folder made under `parent`: in this space, and not past [`MAX_DEPTH`].
    pub fn check_child(&self, parent: Option<&str>) -> Result<(), ValidationError> {
        self.check_room(parent, 1)
    }

    /// A folder moved under `parent`: never into itself or what it holds, never out of its
    /// space, and its whole sub-tree within [`MAX_DEPTH`].
    pub fn check_move(&self, id: &str, parent: Option<&str>) -> Result<(), ValidationError> {
        if let Some(parent) = parent
            && self.subtree(id).iter().any(|below| below == parent)
        {
            return Err(ValidationError::new(
                "parentId",
                "a folder cannot move into itself",
            ));
        }
        self.check_room(parent, self.height(id).max(1))
    }

    fn check_room(&self, parent: Option<&str>, levels: usize) -> Result<(), ValidationError> {
        let above = match parent {
            None => 0,
            Some(parent) if self.contains(parent) => self.depth(parent),
            Some(_) => {
                return Err(ValidationError::new(
                    "parentId",
                    "the parent is not a folder of this space",
                ));
            }
        };
        if above + levels > MAX_DEPTH {
            return Err(ValidationError::new(
                "parentId",
                format!("folders nest at most {MAX_DEPTH} deep"),
            ));
        }
        Ok(())
    }
}

pub fn validated_name(raw: &str) -> Result<String, ValidationError> {
    crate::name::readable(raw, "folder")
}

impl FolderDraft {
    pub fn validated_name(&self) -> Result<String, ValidationError> {
        validated_name(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_before_being_stored() {
        assert_eq!(validated_name("  Perf  ").unwrap(), "Perf");
    }

    #[test]
    fn the_palette_rotates_and_wraps() {
        assert_eq!(FolderColour::nth(0), FolderColour::Blue);
        assert_eq!(FolderColour::nth(1), FolderColour::Amber);
        assert_eq!(
            FolderColour::nth(FolderColour::ALL.len()),
            FolderColour::Blue
        );
    }

    fn tree() -> Lineage {
        // sql ─ perf ─ index
        //     └ migrations
        // angular
        Lineage::new([
            ("sql".to_string(), None),
            ("perf".to_string(), Some("sql".to_string())),
            ("migrations".to_string(), Some("sql".to_string())),
            ("index".to_string(), Some("perf".to_string())),
            ("angular".to_string(), None),
            ("orphan".to_string(), Some("gone".to_string())),
        ])
    }

    #[test]
    fn a_sub_tree_holds_the_folder_and_everything_under_it() {
        let lineage = tree();

        assert_eq!(
            lineage.subtree("sql"),
            ["sql", "perf", "migrations", "index"]
        );
        assert_eq!(lineage.subtree("index"), ["index"]);
        assert!(lineage.subtree("nowhere").is_empty());
    }

    #[test]
    fn a_path_runs_from_the_top_down_and_a_lost_parent_is_the_top() {
        let lineage = tree();

        assert_eq!(lineage.path("index"), ["sql", "perf", "index"]);
        assert_eq!(lineage.path("orphan"), ["orphan"]);
        assert_eq!(lineage.children(None), ["sql", "angular", "orphan"]);
        assert_eq!(lineage.depth("index"), 3);
        assert_eq!(lineage.height("sql"), 3);
        assert_eq!(lineage.height("angular"), 1);
    }

    #[test]
    fn a_cycle_a_damaged_row_makes_is_walked_once() {
        let lineage = Lineage::new([
            ("a".to_string(), Some("b".to_string())),
            ("b".to_string(), Some("a".to_string())),
        ]);

        assert_eq!(lineage.subtree("a"), ["a", "b"]);
        assert_eq!(lineage.path("a"), ["b", "a"]);
    }

    #[test]
    fn a_note_sits_in_the_zone_of_the_child_that_holds_its_folder() {
        let lineage = tree();

        assert_eq!(
            lineage.seat(None, Some("index")),
            Seat::Zone("sql".to_string())
        );
        assert_eq!(lineage.seat(None, None), Seat::Free);
        assert_eq!(
            lineage.seat(Some("sql"), Some("index")),
            Seat::Zone("perf".to_string())
        );
        assert_eq!(lineage.seat(Some("sql"), Some("sql")), Seat::Free);
        assert_eq!(lineage.seat(Some("sql"), Some("angular")), Seat::Elsewhere);
        assert_eq!(lineage.seat(Some("sql"), None), Seat::Elsewhere);
    }

    #[test]
    fn a_folder_never_moves_into_itself_nor_out_of_its_space() {
        let lineage = tree();

        assert!(lineage.check_move("sql", Some("index")).is_err());
        assert!(lineage.check_move("sql", Some("sql")).is_err());
        assert!(lineage.check_move("perf", Some("elsewhere")).is_err());
        assert!(lineage.check_move("perf", None).is_ok());
        assert!(lineage.check_move("angular", Some("index")).is_ok());
    }

    #[test]
    fn nesting_stops_at_its_depth_and_counts_what_a_folder_brings() {
        let chain: Vec<(String, Option<String>)> = (0..MAX_DEPTH)
            .map(|level| {
                (
                    format!("f{level}"),
                    level.checked_sub(1).map(|up| format!("f{up}")),
                )
            })
            .chain([
                ("tall".to_string(), None),
                ("below".to_string(), Some("tall".to_string())),
            ])
            .collect();
        let lineage = Lineage::new(chain);
        let deepest = format!("f{}", MAX_DEPTH - 1);

        assert!(lineage.check_child(Some(&deepest)).is_err());
        assert!(
            lineage
                .check_child(Some(&format!("f{}", MAX_DEPTH - 2)))
                .is_ok()
        );
        assert!(
            lineage
                .check_move("tall", Some(&format!("f{}", MAX_DEPTH - 2)))
                .is_err()
        );
        assert!(
            lineage
                .check_move("tall", Some(&format!("f{}", MAX_DEPTH - 3)))
                .is_ok()
        );
    }

    #[test]
    fn every_colour_survives_its_stored_spelling() {
        for colour in FolderColour::ALL {
            assert_eq!(colour.as_str().parse::<FolderColour>(), Ok(colour));
        }
    }
}
