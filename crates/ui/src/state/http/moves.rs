//! Where a gesture on the rail sends an item, and where it landed, read from the tree the core
//! answered. A move made with the keys is shown and said: at 8 px a level, a row above a folder
//! looks like a row inside it.

use std::collections::{HashMap, HashSet};

use engine::http::model::{HttpItem, HttpItemKind, HttpNode, HttpPlace, HttpTree};

/// Under a parent at a rank, or a collection among the others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RailMove {
    Move { item: HttpItem, place: HttpPlace },
    Reorder { id: String, index: u32 },
}

/// `Alt+↑/↓` within its parent, `Alt+→` into the folder above it, `Alt+←` out of its folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMove {
    Up,
    Down,
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Before,
    After,
    Inside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    Into,
    Root,
    Before,
    After,
}

/// Where a keyboard move left a row, in the terms a reader follows it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    /// The folder or collection that now holds it; `None` for a collection, held by nothing.
    pub parent_id: Option<String>,
    pub at: Where,
    /// The name of what holds it for `Into` and `Root`, of the row it passed otherwise.
    pub other: String,
}

/// Why a keyboard move was not made: said, since a key that does nothing looks unheard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    First,
    Last,
    NoFolderAbove,
    AtRoot,
    Collection,
}

struct Located {
    collection_id: String,
    folder_id: Option<String>,
}

/// Every item with its parent and its name, and every parent with what it holds, in order.
#[derive(Default)]
struct Index {
    located: HashMap<String, Located>,
    children: HashMap<String, Vec<HttpItem>>,
    names: HashMap<String, String>,
}

impl Index {
    fn of(tree: &HttpTree) -> Self {
        let mut index = Self::default();
        for node in &tree.collections {
            let id = &node.collection.id;
            index.located.insert(
                id.clone(),
                Located {
                    collection_id: id.clone(),
                    folder_id: None,
                },
            );
            index.names.insert(id.clone(), node.collection.name.clone());
            index.walk(&node.children, id, None);
        }
        index
    }

    fn walk(&mut self, nodes: &[HttpNode], collection_id: &str, folder_id: Option<&str>) {
        let mut items = Vec::new();
        for node in nodes {
            let (item, name) = match node {
                HttpNode::Folder { folder, .. } => {
                    (item(HttpItemKind::Folder, &folder.id), &folder.name)
                }
                HttpNode::Request { request } => {
                    (item(HttpItemKind::Request, &request.id), &request.name)
                }
            };
            self.located.insert(
                item.id.clone(),
                Located {
                    collection_id: collection_id.to_owned(),
                    folder_id: folder_id.map(str::to_owned),
                },
            );
            self.names.insert(item.id.clone(), name.clone());
            if let HttpNode::Folder { folder, children } = node {
                self.walk(children, collection_id, Some(&folder.id));
            }
            items.push(item);
        }
        self.children
            .insert(folder_id.unwrap_or(collection_id).to_owned(), items);
    }

    fn children_of(&self, parent: &str) -> &[HttpItem] {
        self.children.get(parent).map_or(&[], Vec::as_slice)
    }

    /// A folder and every folder below it.
    fn subtree(&self, folder_id: &str) -> HashSet<String> {
        let mut found = HashSet::from([folder_id.to_owned()]);
        let mut waiting = vec![folder_id.to_owned()];
        while let Some(id) = waiting.pop() {
            for child in self.children_of(&id) {
                if child.kind == HttpItemKind::Folder && found.insert(child.id.clone()) {
                    waiting.push(child.id.clone());
                }
            }
        }
        found
    }
}

fn item(kind: HttpItemKind, id: &str) -> HttpItem {
    HttpItem {
        kind,
        id: id.to_owned(),
    }
}

fn collection_order(tree: &HttpTree) -> Vec<&str> {
    tree.collections
        .iter()
        .map(|node| node.collection.id.as_str())
        .collect()
}

fn rank(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}

pub fn key_move(tree: &HttpTree, moved: &HttpItem, key: KeyMove) -> Option<RailMove> {
    if moved.kind == HttpItemKind::Collection {
        let order = collection_order(tree);
        let at = order.iter().position(|id| *id == moved.id)?;
        let to = match key {
            KeyMove::Up => at.checked_sub(1)?,
            KeyMove::Down => at + 1,
            KeyMove::In | KeyMove::Out => return None,
        };
        return (to < order.len()).then(|| RailMove::Reorder {
            id: moved.id.clone(),
            index: rank(to),
        });
    }
    let index = Index::of(tree);
    let here = index.located.get(&moved.id)?;
    let parent = here.folder_id.as_deref().unwrap_or(&here.collection_id);
    let siblings = index.children_of(parent);
    let at = siblings.iter().position(|sibling| sibling == moved)?;
    let placed = |folder_id: Option<&str>, at: usize| RailMove::Move {
        item: moved.clone(),
        place: HttpPlace {
            collection_id: here.collection_id.clone(),
            folder_id: folder_id.map(str::to_owned),
            index: rank(at),
        },
    };
    match key {
        KeyMove::Up => Some(placed(here.folder_id.as_deref(), at.checked_sub(1)?)),
        KeyMove::Down => {
            (at + 1 < siblings.len()).then(|| placed(here.folder_id.as_deref(), at + 1))
        }
        KeyMove::In => {
            let above = siblings.get(at.checked_sub(1)?)?;
            (above.kind == HttpItemKind::Folder)
                .then(|| placed(Some(&above.id), index.children_of(&above.id).len()))
        }
        KeyMove::Out => {
            let folder_id = here.folder_id.as_deref()?;
            let folder = index.located.get(folder_id)?;
            let around =
                index.children_of(folder.folder_id.as_deref().unwrap_or(&folder.collection_id));
            let at = around.iter().position(|sibling| sibling.id == folder_id)?;
            Some(placed(folder.folder_id.as_deref(), at + 1))
        }
    }
}

/// Read from the tree as it is **after** the move: where `moved` now sits, and beside or inside
/// what. `None` when it is no longer there.
pub fn landing(tree: &HttpTree, moved: &HttpItem, key: KeyMove) -> Option<Landing> {
    let index = Index::of(tree);
    let here = index.located.get(&moved.id)?;
    let collection = moved.kind == HttpItemKind::Collection;
    let parent_id = (!collection).then(|| {
        here.folder_id
            .clone()
            .unwrap_or_else(|| here.collection_id.clone())
    });
    let around: Vec<&str> = match &parent_id {
        None => collection_order(tree),
        Some(parent) => index
            .children_of(parent)
            .iter()
            .map(|sibling| sibling.id.as_str())
            .collect(),
    };
    let at = around.iter().position(|id| *id == moved.id)?;
    let name = |at: Option<usize>| {
        at.and_then(|at| around.get(at))
            .and_then(|id| index.names.get(*id))
    };
    // Up and down are read against the row that was passed: it is the one the eye was on.
    let passed = match key {
        KeyMove::Up => name(Some(at + 1)).map(|other| (Where::Before, other)),
        KeyMove::Down => name(at.checked_sub(1)).map(|other| (Where::After, other)),
        KeyMove::In | KeyMove::Out => None,
    };
    if let Some((at, other)) = passed {
        return Some(Landing {
            parent_id,
            at,
            other: other.clone(),
        });
    }
    let parent = parent_id?;
    Some(Landing {
        at: if here.folder_id.is_none() {
            Where::Root
        } else {
            Where::Into
        },
        other: index.names.get(&parent).cloned().unwrap_or_default(),
        parent_id: Some(parent),
    })
}

/// Why `key_move` answered `None`.
pub fn refusal(moved: &HttpItem, key: KeyMove) -> Refusal {
    match key {
        KeyMove::Up => Refusal::First,
        KeyMove::Down => Refusal::Last,
        _ if moved.kind == HttpItemKind::Collection => Refusal::Collection,
        KeyMove::In => Refusal::NoFolderAbove,
        KeyMove::Out => Refusal::AtRoot,
    }
}

/// Where a drop on `target` sends `dragged`, or `None` when it would change nothing or cannot
/// be: a folder into itself, a collection among anything but collections.
pub fn drop_move(
    tree: &HttpTree,
    dragged: &HttpItem,
    target: &HttpItem,
    zone: DropZone,
) -> Option<RailMove> {
    if dragged == target {
        return None;
    }
    if dragged.kind == HttpItemKind::Collection {
        if target.kind != HttpItemKind::Collection {
            return None;
        }
        let all = collection_order(tree);
        let others: Vec<&str> = all.iter().copied().filter(|id| *id != dragged.id).collect();
        let at =
            others.iter().position(|id| *id == target.id)? + usize::from(zone != DropZone::Before);
        return (all.iter().position(|id| *id == dragged.id) != Some(at)).then(|| {
            RailMove::Reorder {
                id: dragged.id.clone(),
                index: rank(at),
            }
        });
    }
    let index = Index::of(tree);
    let from = index.located.get(&dragged.id)?;
    let onto = index.located.get(&target.id)?;
    let into = target.kind == HttpItemKind::Collection
        || (target.kind == HttpItemKind::Folder && zone == DropZone::Inside);
    let folder_id = if into {
        (target.kind == HttpItemKind::Folder).then(|| target.id.clone())
    } else {
        onto.folder_id.clone()
    };
    let collection_id = onto.collection_id.clone();
    if dragged.kind == HttpItemKind::Folder
        && folder_id
            .as_ref()
            .is_some_and(|folder| index.subtree(&dragged.id).contains(folder))
    {
        return None;
    }
    let parent = folder_id.as_deref().unwrap_or(&collection_id);
    let siblings = index.children_of(parent);
    let others: Vec<&HttpItem> = siblings.iter().filter(|other| *other != dragged).collect();
    let at = if into {
        others.len()
    } else {
        others.iter().position(|other| *other == target)? + usize::from(zone != DropZone::Before)
    };
    let unchanged = from.collection_id == collection_id
        && from.folder_id == folder_id
        && siblings.iter().position(|other| other == dragged) == Some(at);
    (!unchanged).then(|| RailMove::Move {
        item: dragged.clone(),
        place: HttpPlace {
            collection_id,
            folder_id,
            index: rank(at),
        },
    })
}

#[cfg(test)]
mod tests {
    use engine::http::model::{HttpMethod, RequestKind};

    use super::super::tree::tests::{collection, folder, request};
    use super::*;

    fn get(id: &str) -> HttpNode {
        request(id, HttpMethod::Get, RequestKind::Http)
    }

    /// `API[Auth[Login] Factures[Lister Une] Seule]`, `Autre[]`.
    fn sample() -> HttpTree {
        HttpTree {
            collections: vec![
                collection(
                    "API",
                    vec![
                        folder("Auth", vec![get("Login")]),
                        folder("Factures", vec![get("Lister"), get("Une")]),
                        get("Seule"),
                    ],
                ),
                collection("Autre", Vec::new()),
            ],
        }
    }

    fn request_item(id: &str) -> HttpItem {
        item(HttpItemKind::Request, id)
    }

    fn folder_item(id: &str) -> HttpItem {
        item(HttpItemKind::Folder, id)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "what key_move and drop_move answer"
    )]
    fn placed(moved: HttpItem, folder: Option<&str>, index: u32) -> Option<RailMove> {
        Some(RailMove::Move {
            item: moved,
            place: HttpPlace {
                collection_id: "API".to_owned(),
                folder_id: folder.map(str::to_owned),
                index,
            },
        })
    }

    #[test]
    fn the_arrows_move_within_the_parent_into_the_folder_above_and_out() {
        let tree = sample();
        let une = request_item("Une");
        let seule = request_item("Seule");

        assert_eq!(
            key_move(&tree, &une, KeyMove::Up),
            placed(une.clone(), Some("Factures"), 0)
        );
        assert_eq!(key_move(&tree, &une, KeyMove::Down), None);
        assert_eq!(key_move(&tree, &une, KeyMove::Out), placed(une, None, 2));
        assert_eq!(
            key_move(&tree, &seule, KeyMove::In),
            placed(seule.clone(), Some("Factures"), 2)
        );
        assert_eq!(key_move(&tree, &seule, KeyMove::Out), None);
    }

    #[test]
    fn a_collection_moves_among_collections_alone() {
        let tree = sample();
        let api = item(HttpItemKind::Collection, "API");

        assert_eq!(
            key_move(&tree, &api, KeyMove::Down),
            Some(RailMove::Reorder {
                id: "API".to_owned(),
                index: 1
            })
        );
        assert_eq!(key_move(&tree, &api, KeyMove::Up), None);
        assert_eq!(key_move(&tree, &api, KeyMove::In), None);
        assert_eq!(refusal(&api, KeyMove::In), Refusal::Collection);
    }

    #[test]
    fn every_refusal_is_named() {
        let login = request_item("Login");

        assert_eq!(refusal(&login, KeyMove::Up), Refusal::First);
        assert_eq!(refusal(&login, KeyMove::Down), Refusal::Last);
        assert_eq!(refusal(&login, KeyMove::In), Refusal::NoFolderAbove);
        assert_eq!(refusal(&login, KeyMove::Out), Refusal::AtRoot);
    }

    #[test]
    fn a_landing_names_the_row_passed_or_what_now_holds_it() {
        let tree = sample();

        let up = landing(&tree, &request_item("Lister"), KeyMove::Up).unwrap();
        assert_eq!((up.at, up.other.as_str()), (Where::Before, "Une"));
        let down = landing(&tree, &request_item("Une"), KeyMove::Down).unwrap();
        assert_eq!((down.at, down.other.as_str()), (Where::After, "Lister"));
        let into = landing(&tree, &request_item("Login"), KeyMove::In).unwrap();
        assert_eq!(into.at, Where::Into);
        assert_eq!(
            (into.parent_id.as_deref(), into.other.as_str()),
            (Some("Auth"), "Auth")
        );
        let root = landing(&tree, &request_item("Seule"), KeyMove::Out).unwrap();
        assert_eq!((root.at, root.other.as_str()), (Where::Root, "API"));
        assert_eq!(landing(&tree, &request_item("Gone"), KeyMove::Up), None);
    }

    #[test]
    fn a_drop_lands_before_after_or_inside() {
        let tree = sample();
        let seule = request_item("Seule");

        assert_eq!(
            drop_move(&tree, &seule, &request_item("Lister"), DropZone::Before),
            placed(seule.clone(), Some("Factures"), 0)
        );
        assert_eq!(
            drop_move(&tree, &seule, &folder_item("Auth"), DropZone::Inside),
            placed(seule.clone(), Some("Auth"), 1)
        );
        assert_eq!(
            drop_move(&tree, &seule, &folder_item("Auth"), DropZone::After),
            placed(seule, None, 1)
        );
    }

    #[test]
    fn a_drop_that_changes_nothing_or_cannot_be_is_none() {
        let tree = sample();
        let factures = folder_item("Factures");

        assert_eq!(
            drop_move(&tree, &factures, &factures, DropZone::Inside),
            None
        );
        assert_eq!(
            drop_move(&tree, &factures, &folder_item("Auth"), DropZone::After),
            None
        );
        assert_eq!(
            drop_move(&tree, &factures, &request_item("Une"), DropZone::After),
            None
        );
        let api = item(HttpItemKind::Collection, "API");
        assert_eq!(drop_move(&tree, &api, &factures, DropZone::Before), None);
        assert_eq!(
            drop_move(
                &tree,
                &api,
                &item(HttpItemKind::Collection, "Autre"),
                DropZone::After
            ),
            Some(RailMove::Reorder {
                id: "API".to_owned(),
                index: 1
            })
        );
    }
}
