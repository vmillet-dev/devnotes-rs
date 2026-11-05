//! The collections laid flat for the rail: one row a line, its level kept as a depth. What
//! nests in what is the core's `HttpTree`; this only lays it out.

use std::collections::BTreeSet;

use engine::http::model::{
    HttpItemKind, HttpMethod, HttpNode, HttpRequestSummary, HttpTree, RequestKind,
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
    fn a_long_method_is_cut_to_three_letters() {
        assert_eq!(Badge::Method(HttpMethod::Delete).short(), "DEL");
        assert_eq!(Badge::Method(HttpMethod::Options).short(), "OPT");
        assert_eq!(Badge::Method(HttpMethod::Patch).short(), "PATCH");
        assert_eq!(Badge::Graphql.short(), "GQL");
    }
}
