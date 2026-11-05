//! The HTTP collections, their requests, and what is sent.

use engine::http::{
    self, VariablesRequest,
    model::{
        HeaderFault, HttpContents, HttpItem, HttpMethod, HttpPlace, HttpRequest, HttpRequestDraft,
        HttpRequestPatch, HttpTree, KeyValue, RequestDocument, RequestKind,
    },
    query::{QuerySide, SyncedQuery},
    send::ImpliedHeader,
    variables::UsedVariable,
};

use crate::backend::{Backend, Pending};

pub fn tree(backend: &Backend) -> Pending<HttpTree> {
    backend.db(http::http_tree)
}

/// Read by id when its tab opens: the tree lists no document.
pub fn request(backend: &Backend, id: String) -> Pending<HttpRequest> {
    backend.db(move |engine| http::get_http_request(engine, id))
}

/// The tabs left open last time, those still in the library, in their order.
pub fn requests(backend: &Backend, ids: Vec<String>) -> Pending<Vec<HttpRequest>> {
    backend.db(move |engine| {
        Ok(ids
            .into_iter()
            .filter_map(|id| http::get_http_request(engine, id).ok())
            .collect())
    })
}

/// The draft written whole: what Rust kept comes back.
pub fn save_request(
    backend: &Backend,
    id: String,
    patch: HttpRequestPatch,
) -> Pending<HttpRequest> {
    backend.db(move |engine| http::save_http_request(engine, id, patch))
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

/// Each `{{name}}` of a text, by byte range: what the editor colours. Pure, bounded by a field
/// (D12).
pub fn spans(text: &str) -> Vec<(std::ops::Range<usize>, String)> {
    http::variables::spans(text)
}

/// Why each header row cannot be sent, if it cannot. Pure (D12).
pub fn header_faults(rows: &[KeyValue]) -> Vec<Option<HeaderFault>> {
    http::model::header_faults(rows)
}

/// `Name: value` lines read as header rows. Pure (D12).
pub fn read_headers(text: &str) -> Vec<KeyValue> {
    http::model::headers_from_text(text)
}

/// The edited side wins: the query string rewritten from the table, or the table from the URL.
/// Pure (D12).
pub fn sync_query(url: &str, params: &[KeyValue], edited: QuerySide) -> SyncedQuery {
    http::query::sync(url, params, edited)
}

/// What leaves with a request without a row of its own, read as a send reads it.
pub fn implied_headers(
    backend: &Backend,
    (kind, document): (RequestKind, RequestDocument),
    (collection_id, folder_id): (Option<String>, Option<String>),
) -> Pending<Vec<ImpliedHeader>> {
    backend.db(move |engine| {
        http::implied_http_headers(engine, kind, document, collection_id, folder_id)
    })
}

/// Each `{{name}}` a request holds, with what it resolves to and from where.
pub fn used_variables(backend: &Backend, request: VariablesRequest) -> Pending<Vec<UsedVariable>> {
    backend.db(move |engine| http::http_variables(engine, request))
}
