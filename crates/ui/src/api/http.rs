//! The HTTP collections, their requests, and what is sent.

use engine::http::{
    self,
    model::{
        HttpContents, HttpItem, HttpMethod, HttpPlace, HttpRequestDraft, HttpTree, RequestDocument,
        RequestKind,
    },
};

use crate::backend::{Backend, Pending};

pub fn tree(backend: &Backend) -> Pending<HttpTree> {
    backend.db(http::http_tree)
}

/// What a write answers: the request it created, for its tab to open.
pub type Written = Option<String>;

pub fn create_collection(backend: &Backend, name: String) -> Pending<Written> {
    backend.db(move |engine| http::create_http_collection(engine, name).map(|_| None))
}

/// At a collection's root when `parent_id` is `None`.
pub fn create_folder(
    backend: &Backend,
    collection_id: String,
    parent_id: Option<String>,
    name: String,
) -> Pending<Written> {
    backend.db(move |engine| {
        http::create_http_folder(engine, collection_id, parent_id, name).map(|_| None)
    })
}

/// An empty request of `kind`, `GET` until it is given a method.
pub fn create_request(
    backend: &Backend,
    (collection_id, folder_id): (String, Option<String>),
    name: String,
    kind: RequestKind,
) -> Pending<Written> {
    let draft = HttpRequestDraft {
        collection_id,
        folder_id,
        name,
        kind,
        method: HttpMethod::Get,
        document: RequestDocument::default(),
    };
    backend
        .db(move |engine| http::create_http_request(engine, draft).map(|created| Some(created.id)))
}

pub fn rename(backend: &Backend, item: HttpItem, name: String) -> Pending<Written> {
    backend.db(move |engine| http::rename_http_item(engine, item, name).map(|()| None))
}

/// The copy's name is the interface's: no user-visible word is the core's.
pub fn duplicate(backend: &Backend, item: HttpItem, name: String) -> Pending<Written> {
    backend.db(move |engine| http::duplicate_http_item(engine, item, name).map(|_| None))
}

/// What a deletion would take with it, read before the question is shown.
pub fn contents(backend: &Backend, item: HttpItem) -> Pending<HttpContents> {
    backend.db(move |engine| http::count_http_contents(engine, item))
}

pub fn delete(backend: &Backend, item: HttpItem) -> Pending<Written> {
    backend.db(move |engine| http::delete_http_item(engine, item).map(|()| None))
}

pub fn move_item(backend: &Backend, item: HttpItem, place: HttpPlace) -> Pending<Written> {
    backend.db(move |engine| http::move_http_item(engine, item, place).map(|()| None))
}

pub fn reorder_collection(backend: &Backend, id: String, index: u32) -> Pending<Written> {
    backend.db(move |engine| http::reorder_http_collection(engine, id, index).map(|()| None))
}
