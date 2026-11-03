//! The library as rows: every space, its folders under it and theirs under them, the way an
//! editor holds a project. What nests in what is `Lineage`'s; this only lays it out.

use std::collections::{HashMap, HashSet};

use engine::folders::model::{Folder, Lineage};
use engine::spaces::model::Space;

use crate::api;

#[derive(Debug)]
pub(super) enum Row<'a> {
    /// « All spaces »: a choice, not a space.
    All,
    Space {
        space: &'a Space,
        open: bool,
    },
    Folder {
        folder: &'a Folder,
        /// 0 for a folder at the top of its space.
        depth: usize,
        nested: bool,
        open: bool,
    },
    /// « + New folder », under the space shown alone: a folder is made in the space one is in.
    NewFolder {
        space: &'a Space,
    },
    NoFolders,
}

/// `collapsed` holds what was closed, not what was opened: a library is worth showing whole,
/// and a fresh install would otherwise look empty.
pub(super) fn rows<'a>(
    spaces: &'a [Space],
    folders: &'a [Folder],
    active_space: Option<&str>,
    collapsed: &HashSet<String>,
) -> Vec<Row<'a>> {
    let mut rows = vec![Row::All];
    for space in spaces {
        let open = !collapsed.contains(&space.id);
        rows.push(Row::Space { space, open });
        if !open {
            continue;
        }
        let inside: HashMap<&str, &Folder> = folders
            .iter()
            .filter(|folder| folder.space_id == space.id)
            .map(|folder| (folder.id.as_str(), folder))
            .collect();
        let lineage = Lineage::new(
            inside
                .values()
                .map(|folder| (folder.id.clone(), folder.parent_id.clone())),
        );
        let level = Level {
            inside: &inside,
            lineage: &lineage,
            collapsed,
        };
        level.push(&mut rows, None, 0);
        if active_space == Some(space.id.as_str()) {
            rows.push(Row::NewFolder { space });
        } else if inside.is_empty() {
            rows.push(Row::NoFolders);
        }
    }
    rows
}

struct Level<'a, 'b> {
    inside: &'b HashMap<&'a str, &'a Folder>,
    lineage: &'b Lineage,
    collapsed: &'b HashSet<String>,
}

impl<'a> Level<'a, '_> {
    /// The folders under `parent` by name, each followed by what it holds when open.
    fn push(&self, rows: &mut Vec<Row<'a>>, parent: Option<&str>, depth: usize) {
        let mut children: Vec<&'a Folder> = self
            .lineage
            .children(parent)
            .iter()
            .filter_map(|id| self.inside.get(id.as_str()).copied())
            .collect();
        children.sort_by_cached_key(|folder| api::folders::sort_key(&folder.name));
        for folder in children {
            let nested = !self.lineage.children(Some(&folder.id)).is_empty();
            let open = !self.collapsed.contains(&folder.id);
            rows.push(Row::Folder {
                folder,
                depth,
                nested,
                open,
            });
            if nested && open {
                self.push(rows, Some(&folder.id), depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use engine::folders::model::FolderColour;

    use super::*;

    fn space(id: &str) -> Space {
        Space {
            id: id.into(),
            name: id.into(),
            pinned: false,
        }
    }

    fn folder(id: &str, space: &str, parent: Option<&str>) -> Folder {
        Folder {
            id: id.into(),
            space_id: space.into(),
            name: id.into(),
            colour: FolderColour::Blue,
            created_at: Utc::now(),
            parent_id: parent.map(Into::into),
        }
    }

    /// What each row is, in a word, for an assertion that reads as the rail does.
    fn shape(rows: &[Row<'_>]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::All => "all".into(),
                Row::Space { space, .. } => space.name.clone(),
                Row::Folder { folder, depth, .. } => {
                    format!("{}{}", "  ".repeat(depth + 1), folder.name)
                }
                Row::NewFolder { .. } => "  +".into(),
                Row::NoFolders => "  none".into(),
            })
            .collect()
    }

    #[test]
    fn folders_nest_under_their_space_by_name_whatever_their_order() {
        let spaces = [space("home"), space("work")];
        let folders = [
            folder("runbooks", "home", None),
            folder("api", "home", Some("runbooks")),
            folder("Archive", "home", None),
            folder("élan", "home", None),
        ];

        let rows = rows(&spaces, &folders, None, &HashSet::new());

        assert_eq!(
            shape(&rows),
            [
                "all",
                "home",
                "  Archive",
                "  élan",
                "  runbooks",
                "    api",
                "work",
                "  none"
            ]
        );
    }

    #[test]
    fn a_closed_row_hides_what_it_holds() {
        let spaces = [space("home"), space("work")];
        let folders = [
            folder("runbooks", "home", None),
            folder("api", "home", Some("runbooks")),
        ];
        let closed = HashSet::from(["runbooks".to_string(), "work".to_string()]);

        let rows = rows(&spaces, &folders, None, &closed);

        assert_eq!(shape(&rows), ["all", "home", "  runbooks", "work"]);
        assert!(matches!(
            rows[2],
            Row::Folder {
                nested: true,
                open: false,
                ..
            }
        ));
    }

    #[test]
    fn a_new_folder_is_offered_in_the_space_shown_alone() {
        let spaces = [space("home"), space("work")];

        let rows = rows(&spaces, &[], Some("work"), &HashSet::new());

        assert_eq!(shape(&rows), ["all", "home", "  none", "work", "  +"]);
    }

    #[test]
    fn a_folder_whose_parent_is_gone_reads_as_at_the_top() {
        let spaces = [space("home")];
        let folders = [folder("api", "home", Some("gone"))];

        let rows = rows(&spaces, &folders, None, &HashSet::new());

        assert_eq!(shape(&rows), ["all", "home", "  api"]);
    }
}
