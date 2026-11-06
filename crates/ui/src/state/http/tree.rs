//! The collections laid flat for the rail: one row a line, its level kept as a depth. What
//! nests in what is the core's `HttpTree`; this only lays it out.

use super::containers::Container;
use std::collections::BTreeSet;

use engine::http::model::{
    HttpItem, HttpItemKind, HttpMethod, HttpNode, HttpRequestSummary, HttpTree, RequestKind,
};

/// What the rail writes before a request's name: its method, or the kind's own word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Method(HttpMethod),
    Graphql,
    Websocket,
}

impl Badge {
    pub fn of(request: &HttpRequestSummary) -> Self {
        match request.kind {
            RequestKind::Http => Self::Method(request.method),
            RequestKind::Graphql => Self::Graphql,
            RequestKind::Websocket => Self::Websocket,
        }
    }

    /// A long method in three letters, where room is counted: the rail at its floor, a tab.
    pub fn short(self) -> &'static str {
        match self {
            Self::Method(HttpMethod::Delete) => "DEL",
            Self::Method(HttpMethod::Options) => "OPT",
            Self::Method(method) => method.as_str(),
            Self::Graphql => "GQL",
            Self::Websocket => "WS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailRow {
    pub kind: HttpItemKind,
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub collection_id: String,
    /// The folder it sits in; `None` at a collection's root, and for a collection.
    pub folder_id: Option<String>,
    pub expanded: bool,
    /// A request's alone.
    pub badge: Option<Badge>,
}

/// A collection or a folder of the tree, for its settings.
pub fn container_of(tree: &HttpTree, id: &str) -> Option<Container> {
    fn inside(
        nodes: &[HttpNode],
        collection_id: &str,
        parent: Option<&str>,
        id: &str,
    ) -> Option<Container> {
        nodes.iter().find_map(|node| {
            let HttpNode::Folder { folder, children } = node else {
                return None;
            };
            if folder.id == id {
                return Some(Container {
                    item: HttpItem {
                        kind: HttpItemKind::Folder,
                        id: folder.id.clone(),
                    },
                    name: folder.name.clone(),
                    collection_id: collection_id.to_owned(),
                    folder_id: parent.map(str::to_owned),
                });
            }
            inside(children, collection_id, Some(&folder.id), id)
        })
    }
    tree.collections.iter().find_map(|node| {
        let collection = &node.collection;
        if collection.id == id {
            return Some(Container {
                item: HttpItem {
                    kind: HttpItemKind::Collection,
                    id: collection.id.clone(),
                },
                name: collection.name.clone(),
                collection_id: collection.id.clone(),
                folder_id: None,
            });
        }
        inside(&node.children, &collection.id, None, id)
    })
}

/// A place a request can be saved in: a collection's root or a folder, by its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePlace {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub path: String,
}

pub fn save_places(tree: &HttpTree) -> Vec<SavePlace> {
    let mut places = Vec::new();
    for node in &tree.collections {
        let collection = &node.collection;
        places.push(SavePlace {
            collection_id: collection.id.clone(),
            folder_id: None,
            path: collection.name.clone(),
        });
        folders_into(
            &mut places,
            &node.children,
            &collection.id,
            &collection.name,
        );
    }
    places
}

fn folders_into(places: &mut Vec<SavePlace>, nodes: &[HttpNode], collection_id: &str, path: &str) {
    for node in nodes {
        if let HttpNode::Folder { folder, children } = node {
            let here = format!("{path} / {}", folder.name);
            places.push(SavePlace {
                collection_id: collection_id.to_owned(),
                folder_id: Some(folder.id.clone()),
                path: here.clone(),
            });
            folders_into(places, children, collection_id, &here);
        }
    }
}

/// The rows a reader sees: what a collapsed collection or folder holds is left out.
/// `collapsed` holds what was closed, not what was opened: a fresh tree shows whole.
pub fn rail_rows(tree: &HttpTree, collapsed: &BTreeSet<String>) -> Vec<RailRow> {
    let mut rows = Vec::new();
    for node in &tree.collections {
        let collection = &node.collection;
        let expanded = !collapsed.contains(&collection.id);
        rows.push(RailRow {
            kind: HttpItemKind::Collection,
            id: collection.id.clone(),
            name: collection.name.clone(),
            depth: 0,
            collection_id: collection.id.clone(),
            folder_id: None,
            expanded,
            badge: None,
        });
        if expanded {
            let at = Level {
                collection_id: &collection.id,
                folder_id: None,
                depth: 1,
            };
            walk(&mut rows, &node.children, &at, collapsed);
        }
    }
    rows
}

struct Level<'a> {
    collection_id: &'a str,
    folder_id: Option<&'a str>,
    depth: usize,
}

fn walk(rows: &mut Vec<RailRow>, nodes: &[HttpNode], at: &Level<'_>, collapsed: &BTreeSet<String>) {
    for node in nodes {
        match node {
            HttpNode::Folder { folder, children } => {
                let expanded = !collapsed.contains(&folder.id);
                rows.push(RailRow {
                    kind: HttpItemKind::Folder,
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                    depth: at.depth,
                    collection_id: at.collection_id.to_owned(),
                    folder_id: at.folder_id.map(str::to_owned),
                    expanded,
                    badge: None,
                });
                if expanded {
                    let inside = Level {
                        collection_id: at.collection_id,
                        folder_id: Some(&folder.id),
                        depth: at.depth + 1,
                    };
                    walk(rows, children, &inside, collapsed);
                }
            }
            HttpNode::Request { request } => rows.push(RailRow {
                kind: HttpItemKind::Request,
                id: request.id.clone(),
                name: request.name.clone(),
                depth: at.depth,
                collection_id: at.collection_id.to_owned(),
                folder_id: at.folder_id.map(str::to_owned),
                expanded: false,
                badge: Some(Badge::of(request)),
            }),
        }
    }
}

/// A request as the tree holds it — its name and where it sits — or `None` once it is gone.
pub fn find_request<'a>(tree: &'a HttpTree, id: &str) -> Option<&'a HttpRequestSummary> {
    fn walk<'a>(nodes: &'a [HttpNode], id: &str) -> Option<&'a HttpRequestSummary> {
        nodes.iter().find_map(|node| match node {
            HttpNode::Request { request } => (request.id == id).then_some(request),
            HttpNode::Folder { children, .. } => walk(children, id),
        })
    }
    tree.collections
        .iter()
        .find_map(|node| walk(&node.children, id))
}

/// Every request a deletion takes: the request itself, or all a folder or a collection holds.
pub fn requests_under(tree: &HttpTree, item: &HttpItem) -> Vec<String> {
    fn all(nodes: &[HttpNode], found: &mut Vec<String>) {
        for node in nodes {
            match node {
                HttpNode::Request { request } => found.push(request.id.clone()),
                HttpNode::Folder { children, .. } => all(children, found),
            }
        }
    }
    fn folder<'a>(nodes: &'a [HttpNode], id: &str) -> Option<&'a [HttpNode]> {
        nodes.iter().find_map(|node| match node {
            HttpNode::Folder {
                folder: held,
                children,
            } if held.id == id => Some(children.as_slice()),
            HttpNode::Folder { children, .. } => folder(children, id),
            HttpNode::Request { .. } => None,
        })
    }
    let mut found = Vec::new();
    match item.kind {
        HttpItemKind::Request => found.push(item.id.clone()),
        HttpItemKind::Folder => {
            if let Some(children) = tree
                .collections
                .iter()
                .find_map(|node| folder(&node.children, &item.id))
            {
                all(children, &mut found);
            }
        }
        HttpItemKind::Collection => {
            if let Some(node) = tree
                .collections
                .iter()
                .find(|node| node.collection.id == item.id)
            {
                all(&node.children, &mut found);
            }
        }
    }
    found
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::Utc;
    use engine::http::model::{HttpCollection, HttpCollectionNode, HttpFolder};

    use super::*;

    pub(crate) fn request(id: &str, method: HttpMethod, kind: RequestKind) -> HttpNode {
        HttpNode::Request {
            request: HttpRequestSummary {
                id: id.to_owned(),
                collection_id: String::new(),
                folder_id: None,
                name: id.to_owned(),
                kind,
                method,
                position: 0,
            },
        }
    }

    pub(crate) fn folder(id: &str, children: Vec<HttpNode>) -> HttpNode {
        HttpNode::Folder {
            folder: HttpFolder {
                id: id.to_owned(),
                collection_id: String::new(),
                parent_id: None,
                name: id.to_owned(),
                position: 0,
            },
            children,
        }
    }

    pub(crate) fn collection(id: &str, children: Vec<HttpNode>) -> HttpCollectionNode {
        HttpCollectionNode {
            collection: HttpCollection {
                id: id.to_owned(),
                name: id.to_owned(),
                position: 0,
                created_at: Utc::now(),
            },
            children,
        }
    }

    /// `API[Auth[Login] Factures]`
    fn sample() -> HttpTree {
        HttpTree {
            collections: vec![collection(
                "API",
                vec![
                    folder(
                        "Auth",
                        vec![request("Login", HttpMethod::Post, RequestKind::Http)],
                    ),
                    request("Socket", HttpMethod::Get, RequestKind::Websocket),
                ],
            )],
        }
    }

    fn outline(rows: &[RailRow]) -> Vec<(usize, &str, Option<&str>)> {
        rows.iter()
            .map(|row| (row.depth, row.name.as_str(), row.folder_id.as_deref()))
            .collect()
    }

    #[test]
    fn a_tree_is_laid_flat_with_its_levels() {
        let rows = rail_rows(&sample(), &BTreeSet::new());

        assert_eq!(
            outline(&rows),
            [
                (0, "API", None),
                (1, "Auth", None),
                (2, "Login", Some("Auth")),
                (1, "Socket", None),
            ]
        );
        assert!(rows.iter().all(|row| row.collection_id == "API"));
        assert_eq!(rows[2].badge, Some(Badge::Method(HttpMethod::Post)));
        assert_eq!(rows[3].badge, Some(Badge::Websocket));
    }

    #[test]
    fn what_a_collapsed_folder_holds_is_left_out() {
        let collapsed = BTreeSet::from(["Auth".to_owned()]);

        let rows = rail_rows(&sample(), &collapsed);

        assert_eq!(
            outline(&rows),
            [(0, "API", None), (1, "Auth", None), (1, "Socket", None)]
        );
        assert!(!rows[1].expanded);
    }

    #[test]
    fn a_collapsed_collection_is_its_row_alone() {
        let collapsed = BTreeSet::from(["API".to_owned()]);

        assert_eq!(rail_rows(&sample(), &collapsed).len(), 1);
    }

    #[test]
    fn a_deletion_takes_what_its_item_holds() {
        let tree = sample();
        let item = |kind, id: &str| HttpItem {
            kind,
            id: id.to_owned(),
        };

        assert_eq!(
            requests_under(&tree, &item(HttpItemKind::Folder, "Auth")),
            ["Login"]
        );
        assert_eq!(
            requests_under(&tree, &item(HttpItemKind::Collection, "API")),
            ["Login", "Socket"]
        );
        assert_eq!(
            find_request(&tree, "Login").map(|request| request.name.as_str()),
            Some("Login")
        );
        assert!(find_request(&tree, "Auth").is_none());
    }

    #[test]
    fn a_long_method_is_cut_to_three_letters() {
        assert_eq!(Badge::Method(HttpMethod::Delete).short(), "DEL");
        assert_eq!(Badge::Method(HttpMethod::Options).short(), "OPT");
        assert_eq!(Badge::Method(HttpMethod::Patch).short(), "PATCH");
        assert_eq!(Badge::Graphql.short(), "GQL");
    }
}
