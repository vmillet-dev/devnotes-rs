#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::http::BodyAnswer;
use engine::http::CodeRequestOf;
use engine::http::VariablesRequest;
use engine::http::WebsocketRequest;
use engine::http::codegen;
use engine::http::cookies;
use engine::http::environments;
use engine::http::graphql;
use engine::http::history;
use engine::http::model;
use engine::http::model::HttpCollection;
use engine::http::model::HttpContents;
use engine::http::model::HttpFolder;
use engine::http::model::HttpItem;
use engine::http::model::HttpPlace;
use engine::http::model::HttpRequest;
use engine::http::model::HttpRequestDraft;
use engine::http::model::HttpRequestPatch;
use engine::http::model::HttpTree;
use engine::http::model::NewPlace;
use engine::http::query;
use engine::http::runner;
use engine::http::search;
use engine::http::send;
use engine::http::settings;
use engine::http::variables;
use engine::{Engine, http};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn http_tree(engine: State<'_, Arc<Engine>>) -> Result<HttpTree, AppError> {
    run(&engine, http::http_tree).await
}

/// The palette's requests, ranked here: the URL is read from each sealed document.
#[tauri::command]
#[specta::specta]
pub async fn search_http_requests(
    search: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<search::RequestHit>, AppError> {
    run(&engine, move |engine| {
        http::search_http_requests(engine, search)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_collection(
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpCollection, AppError> {
    run(&engine, move |engine| {
        http::create_http_collection(engine, name)
    })
    .await
}

/// `parent_id` `None` puts it at the collection's root.
#[tauri::command]
#[specta::specta]
pub async fn create_http_folder(
    collection_id: String,
    parent_id: Option<String>,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpFolder, AppError> {
    run(&engine, move |engine| {
        http::create_http_folder(engine, collection_id, parent_id, name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_request(
    draft: HttpRequestDraft,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpRequest, AppError> {
    run(&engine, move |engine| {
        http::create_http_request(engine, draft)
    })
    .await
}

/// The first save of a request where nothing holds it yet: what `create` names is created
/// with it, or nothing is. Every name is checked before anything is written.
#[tauri::command]
#[specta::specta]
pub async fn create_http_request_in(
    draft: HttpRequestDraft,
    create: NewPlace,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpRequest, AppError> {
    run(&engine, move |engine| {
        http::create_http_request_in(engine, draft, create)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_http_request(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpRequest, AppError> {
    run(&engine, move |engine| http::get_http_request(engine, id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_request(
    id: String,
    patch: HttpRequestPatch,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpRequest, AppError> {
    run(&engine, move |engine| {
        http::save_http_request(engine, id, patch)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_http_item(
    item: HttpItem,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::rename_http_item(engine, item, name)
    })
    .await
}

/// What a deletion would take with it, for the confirmation that comes first.
#[tauri::command]
#[specta::specta]
pub async fn count_http_contents(
    item: HttpItem,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpContents, AppError> {
    run(&engine, move |engine| {
        http::count_http_contents(engine, item)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_item(
    item: HttpItem,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| http::delete_http_item(engine, item)).await
}

/// The copy's name comes from the front: no user-visible word is Rust's.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_http_item(
    item: HttpItem,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<HttpItem, AppError> {
    run(&engine, move |engine| {
        http::duplicate_http_item(engine, item, name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn move_http_item(
    item: HttpItem,
    place: HttpPlace,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::move_http_item(engine, item, place)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn http_settings(
    item: HttpItem,
    engine: State<'_, Arc<Engine>>,
) -> Result<model::ContainerSettings, AppError> {
    run(&engine, move |engine| http::http_settings(engine, item)).await
}

/// What a request placed there inherits, and from where: « héritée de la collection … ».
#[tauri::command]
#[specta::specta]
pub async fn inherited_http_settings(
    collection_id: String,
    folder_id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<settings::Inherited, AppError> {
    run(&engine, move |engine| {
        http::inherited_http_settings(engine, collection_id, folder_id)
    })
    .await
}

/// What a body implies, whether a JSON one reads, and whether its method seldom carries one:
/// the editor says the three as it is typed, and decides none of them.
#[tauri::command]
#[specta::specta]
pub async fn describe_http_body(
    body: model::RequestBody,
    method: model::HttpMethod,
) -> Result<BodyAnswer, AppError> {
    http::describe_http_body(body, method).await
}

/// Each header's fault by its rank, or none: the table marks the row and says why. The rule is
/// the model's; the table holds none.
#[tauri::command]
#[specta::specta]
pub async fn check_http_headers(
    rows: Vec<model::KeyValue>,
) -> Result<Vec<Option<model::HeaderFault>>, AppError> {
    http::check_http_headers(rows).await
}

/// `Nom: valeur` typed in a name, or lines pasted from a browser's network tab, as rows.
#[tauri::command]
#[specta::specta]
pub async fn read_http_headers(text: String) -> Result<Vec<model::KeyValue>, AppError> {
    http::read_http_headers(text).await
}

/// What a request sends without a row of its own — the body's type, what it inherits, the
/// application's `User-Agent` — read as a send reads it: the editor lists the answer and
/// decides none of it.
#[tauri::command]
#[specta::specta]
pub async fn implied_http_headers(
    kind: model::RequestKind,
    document: model::RequestDocument,
    collection_id: Option<String>,
    folder_id: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<send::ImpliedHeader>, AppError> {
    run(&engine, move |engine| {
        http::implied_http_headers(engine, kind, document, collection_id, folder_id)
    })
    .await
}

/// The edited side wins: the query string rewritten from the table, or the table from the URL.
#[tauri::command]
#[specta::specta]
pub async fn sync_http_query(
    url: String,
    params: Vec<model::KeyValue>,
    edited: query::QuerySide,
) -> Result<query::SyncedQuery, AppError> {
    http::sync_http_query(url, params, edited).await
}

#[tauri::command]
#[specta::specta]
pub async fn reorder_http_collection(
    id: String,
    index: u32,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::reorder_http_collection(engine, id, index)
    })
    .await
}

/// The request written as code for another tool, from what a send would write: its variables
/// resolved as asked, a secret left as its `{{name}}` unless it is asked for in clear.
#[tauri::command]
#[specta::specta]
pub async fn generate_http_code(
    request: CodeRequestOf,
    engine: State<'_, Arc<Engine>>,
) -> Result<codegen::GeneratedCode, AppError> {
    run(&engine, move |engine| {
        http::generate_http_code(engine, request)
    })
    .await
}

/// The `{{variables}}` a request holds, each with what it resolves to and from where: the
/// editor colours them by it, and names the ones nothing sets before a send does.
#[tauri::command]
#[specta::specta]
pub async fn http_variables(
    request: VariablesRequest,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<variables::UsedVariable>, AppError> {
    run(&engine, move |engine| http::http_variables(engine, request)).await
}

/// The order a collection or a folder runs in: the one last arranged, the requests added since
/// at its end.
#[tauri::command]
#[specta::specta]
pub async fn http_run_order(
    item: HttpItem,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<model::RunEntry>, AppError> {
    run(&engine, move |engine| http::http_run_order(engine, item)).await
}

/// The order the runner was left in, kept with the collection's or the folder's settings.
#[tauri::command]
#[specta::specta]
pub async fn save_http_run_order(
    item: HttpItem,
    order: Vec<model::RunEntry>,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::save_http_run_order(engine, item, order)
    })
    .await
}

/// A run's data file read: a CSV with its header, or a JSON array of objects.
#[tauri::command]
#[specta::specta]
pub async fn read_http_run_data(path: String) -> Result<runner::RunData, AppError> {
    http::read_http_run_data(path).await
}

/// What was sent, newest first, a group a local day.
#[tauri::command]
#[specta::specta]
pub async fn http_history(
    tz_offset_minutes: i32,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<history::HistoryDay>, AppError> {
    run(&engine, move |engine| {
        http::http_history(engine, tz_offset_minutes)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn http_history_entry(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<history::HistoryEntry, AppError> {
    run(&engine, move |engine| http::http_history_entry(engine, id)).await
}

/// What « Vider l'historique » says it removes.
#[tauri::command]
#[specta::specta]
pub async fn count_http_history(engine: State<'_, Arc<Engine>>) -> Result<u32, AppError> {
    run(&engine, http::count_http_history).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_http_history(engine: State<'_, Arc<Engine>>) -> Result<u32, AppError> {
    run(&engine, http::clear_http_history).await
}

/// One entry gone, answered whole: its undo hands it back to `restore_http_history_entry`.
#[tauri::command]
#[specta::specta]
pub async fn delete_http_history_entry(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<history::HistoryEntry, AppError> {
    run(&engine, move |engine| {
        http::delete_http_history_entry(engine, id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn restore_http_history_entry(
    entry: history::HistoryEntry,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::restore_http_history_entry(engine, entry)
    })
    .await
}

/// A send put back onto the request it came from, as changes to save or not, the request's own
/// secrets where the record holds a mask. `None` when that request is gone, or the send came
/// from a draft: the front then opens one, as `http_history_draft` rebuilds it.
#[tauri::command]
#[specta::specta]
pub async fn http_history_onto_request(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<history::HistoryDraft>, AppError> {
    run(&engine, move |engine| {
        http::http_history_onto_request(engine, id)
    })
    .await
}

/// An entry rebuilt as a request to send again or save: its secrets are typed again.
#[tauri::command]
#[specta::specta]
pub async fn http_history_draft(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<history::HistoryDraft, AppError> {
    run(&engine, move |engine| http::http_history_draft(engine, id)).await
}

/// The operations a GraphQL document names, and why its variables do not read.
#[tauri::command]
#[specta::specta]
pub async fn describe_graphql(
    document: graphql::GraphqlDocument,
) -> Result<graphql::GraphqlAnswer, AppError> {
    http::describe_graphql(document).await
}

/// The variables with a `null` for each one the operation declares and they lack: « Ajouter ».
#[tauri::command]
#[specta::specta]
pub async fn add_graphql_variables(document: graphql::GraphqlDocument) -> Result<String, AppError> {
    http::add_graphql_variables(document).await
}

/// The jar by domain, for the manager.
#[tauri::command]
#[specta::specta]
pub async fn http_cookies(
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<cookies::store::CookieDomain>, AppError> {
    run(&engine, http::http_cookies).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_cookie(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| http::delete_http_cookie(engine, id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_cookie_domain(
    domain: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| {
        http::delete_http_cookie_domain(engine, domain)
    })
    .await
}

/// What « Vider le bocal » says it removes.
#[tauri::command]
#[specta::specta]
pub async fn count_http_cookies(engine: State<'_, Arc<Engine>>) -> Result<u32, AppError> {
    run(&engine, http::count_http_cookies).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_http_cookies(engine: State<'_, Arc<Engine>>) -> Result<u32, AppError> {
    run(&engine, http::clear_http_cookies).await
}

/// The globals and every environment, in their order.
#[tauri::command]
#[specta::specta]
pub async fn http_environments(
    engine: State<'_, Arc<Engine>>,
) -> Result<environments::HttpEnvironments, AppError> {
    run(&engine, http::http_environments).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_environment(
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<environments::Environment, AppError> {
    run(&engine, move |engine| {
        http::create_http_environment(engine, name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_environment(
    id: String,
    patch: environments::EnvironmentPatch,
    engine: State<'_, Arc<Engine>>,
) -> Result<environments::Environment, AppError> {
    run(&engine, move |engine| {
        http::save_http_environment(engine, id, patch)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_globals(
    variables: Vec<environments::Variable>,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<environments::Variable>, AppError> {
    run(&engine, move |engine| {
        http::save_http_globals(engine, variables)
    })
    .await
}

/// The copy's name comes from the front: no user-visible word is Rust's.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_http_environment(
    id: String,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<environments::Environment, AppError> {
    run(&engine, move |engine| {
        http::duplicate_http_environment(engine, id, name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_environment(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::delete_http_environment(engine, id)
    })
    .await
}

/// Each variable's fault by its rank, or none: the table marks the row, the rule is the model's.
#[tauri::command]
#[specta::specta]
pub async fn check_http_variables(
    variables: Vec<environments::Variable>,
) -> Result<Vec<Option<environments::VariableFault>>, AppError> {
    http::check_http_variables(variables).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_settings(
    item: HttpItem,
    settings: model::ContainerSettings,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        http::save_http_settings(engine, item, settings)
    })
    .await
}

/// What it inherits is read under the lock, its files outside it, and the send waits on nothing
/// but the network and its scripts: a 30-second request stalls no other command.
#[tauri::command]
#[specta::specta]
pub async fn send_http_request(
    request: send::SendRequest,
    engine: State<'_, Arc<Engine>>,
) -> Result<send::SentResponse, AppError> {
    http::send_http_request(Arc::clone(&engine), request).await
}

/// A collection or a folder run, one request after the other, each sent as a send is — its
/// scripts, its tests, its extractions — but written in no history. Each step is said as an
/// event; answers once the run is over. The lock is held for reads alone, never over a send.
#[tauri::command]
#[specta::specta]
pub async fn run_http_collection(
    run: runner::RunRequest,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    http::run_http_collection(Arc::clone(&engine), run).await
}

/// « Annuler » on a run: the request under way is cancelled with it. `false` when no run of that
/// id was under way.
#[tauri::command]
#[specta::specta]
pub async fn cancel_http_run(id: String, engine: State<'_, Arc<Engine>>) -> Result<bool, AppError> {
    http::cancel_http_run(&engine, id)
}

/// `false` when that send had already ended.
#[tauri::command]
#[specta::specta]
pub async fn cancel_http_send(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<bool, AppError> {
    http::cancel_http_send(&engine, id)
}

/// The whole body of the last answer to `id`, not the text shown: `false` when there is none.
#[tauri::command]
#[specta::specta]
pub async fn save_http_response(
    id: String,
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<bool, AppError> {
    http::save_http_response(Arc::clone(&engine), id, path).await
}

/// A closed tab lets its last body go.
#[tauri::command]
#[specta::specta]
pub async fn forget_http_response(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    http::forget_http_response(&engine, id)
}

/// The last answer to `id` as an image to show, when it is one.
#[tauri::command]
#[specta::specta]
pub async fn http_response_image(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Option<String>, AppError> {
    http::http_response_image(Arc::clone(&engine), id).await
}

/// An answer's text made a note: the secrets the request sent, masked wherever it says them back.
#[tauri::command]
#[specta::specta]
pub async fn http_response_note(
    id: Option<String>,
    exchange: send::Exchange,
    text: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<send::NoteText, AppError> {
    http::http_response_note(Arc::clone(&engine), id, exchange, text).await
}

/// Its headers and auth composed like a request's, under the lock for what it inherits; the
/// handshake waited on without it. Answers once the socket is open.
#[tauri::command]
#[specta::specta]
pub async fn connect_websocket(
    request: WebsocketRequest,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    http::connect_websocket(Arc::clone(&engine), request).await
}

/// `false` when no socket is open under that id.
#[tauri::command]
#[specta::specta]
pub async fn send_websocket(
    id: String,
    text: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<bool, AppError> {
    http::send_websocket(&engine, id, text)
}

#[tauri::command]
#[specta::specta]
pub async fn close_websocket(id: String, engine: State<'_, Arc<Engine>>) -> Result<bool, AppError> {
    http::close_websocket(&engine, id)
}
