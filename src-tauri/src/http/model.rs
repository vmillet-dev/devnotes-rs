//! The HTTP collections as the interface sees them: a tree of collections, folders and
//! requests, a request's sealed document, and where an item can be moved to.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use crate::error::ValidationError;

closed_enum! {
    pub enum HttpMethod {
        #[default]
        Get = "GET",
        Post = "POST",
        Put = "PUT",
        Patch = "PATCH",
        Delete = "DELETE",
        Head = "HEAD",
        Options = "OPTIONS",
    }
}

closed_enum! {
    /// What the rail writes before a name: the method, or `QUERY` and `WS` for the other two.
    pub enum RequestKind {
        #[default]
        Http = "http",
        Graphql = "graphql",
        Websocket = "websocket",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HttpItemKind {
    Collection,
    Folder,
    Request,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpItem {
    pub kind: HttpItemKind,
    pub id: String,
}

/// What a collection or a folder hands its requests (#478, #484). Each field is
/// `#[serde(default)]`: one added later needs no migration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ContainerSettings {}

/// A request's parts, sealed as one document. Each field is `#[serde(default)]`: one added
/// later needs no migration, and a document written by an older version still opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestDocument {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpCollection {
    pub id: String,
    pub name: String,
    pub position: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpFolder {
    pub id: String,
    pub collection_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub position: u32,
}

/// A request as the tree lists it: no document, which is read by id when it opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSummary {
    pub id: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub id: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestDraft {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
}

/// A field left out is untouched.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestPatch {
    #[specta(optional)]
    pub name: Option<String>,
    #[specta(optional)]
    pub method: Option<HttpMethod>,
    #[specta(optional)]
    pub document: Option<RequestDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HttpNode {
    Folder {
        folder: HttpFolder,
        children: Vec<HttpNode>,
    },
    Request {
        request: HttpRequestSummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpCollectionNode {
    pub collection: HttpCollection,
    pub children: Vec<HttpNode>,
}

/// Built by Rust: the front draws it and never joins a request to its folder itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpTree {
    pub collections: Vec<HttpCollectionNode>,
}

/// What a deletion takes with it, said before it happens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpContents {
    pub folders: u32,
    pub requests: u32,
}

/// Under which parent, and at which rank among its folders and requests.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpPlace {
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub index: u32,
}

pub fn validated_name(raw: &str) -> Result<String, ValidationError> {
    crate::name::readable(raw, "collection, folder or request")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_method_and_kind_survives_its_stored_spelling() {
        for method in HttpMethod::ALL {
            assert_eq!(method.as_str().parse::<HttpMethod>(), Ok(method));
        }
        for kind in RequestKind::ALL {
            assert_eq!(kind.as_str().parse::<RequestKind>(), Ok(kind));
        }
    }

    #[test]
    fn a_document_written_before_a_field_existed_still_opens() {
        let document: RequestDocument =
            serde_json::from_str(r#"{"url":"https://exemple.fr"}"#).unwrap();

        assert_eq!(document.url, "https://exemple.fr");
        assert_eq!(document.description, "");
        assert_eq!(
            serde_json::from_str::<RequestDocument>(r#"{"later":1}"#).unwrap(),
            RequestDocument::default()
        );
    }
}
