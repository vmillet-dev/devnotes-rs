#![allow(clippy::needless_pass_by_value)]

//! The HTTP client's collections: they belong to the library, not to a space, and are sealed
//! like the notes. A collection travels in a file of its own, never in the library export.

pub mod codegen;
mod collection_run;
pub mod console;
pub mod cookies;
pub mod environments;
mod error;
pub mod extract;
mod flight;
pub mod graphql;
pub mod history;
pub mod model;
pub mod query;
pub mod response;
pub mod runner;
pub mod scripts;
pub mod search;
pub mod send;
pub mod settings;
pub mod store;
pub mod transport;
pub mod variables;
pub mod websocket;

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use crate::Engine;
use crate::db::{self, lock};
use crate::error::{AppError, FileContext};
use crate::tools::off_thread;
use collection_run::{RunStep, run_step, say_run, wait};
use flight::Flight;
use model::{
    HttpCollection, HttpContents, HttpFolder, HttpItem, HttpPlace, HttpRequest, HttpRequestDraft,
    HttpRequestPatch, HttpTree, NewPlace,
};

pub fn http_tree(engine: &Engine) -> Result<HttpTree, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::tree(&mut connection)?)
}

/// The palette's requests, ranked here: the URL is read from each sealed document.
pub fn search_http_requests(
    engine: &Engine,
    search: String,
) -> Result<Vec<search::RequestHit>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(search::rank(
        store::search_candidates(&mut connection)?,
        &search,
    ))
}

pub fn create_http_collection(engine: &Engine, name: String) -> Result<HttpCollection, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::create_collection(
        &mut connection,
        &name,
        Utc::now(),
    )?)
}

/// `parent_id` `None` puts it at the collection's root.
pub fn create_http_folder(
    engine: &Engine,
    collection_id: String,
    parent_id: Option<String>,
    name: String,
) -> Result<HttpFolder, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::create_folder(
        &mut connection,
        &collection_id,
        parent_id.as_deref(),
        &name,
        Utc::now(),
    )?)
}

pub fn create_http_request(
    engine: &Engine,
    draft: HttpRequestDraft,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&draft.name)?;

    let mut connection = lock(db)?;

    Ok(store::create_request(
        &mut connection,
        &draft,
        &name,
        Utc::now(),
    )?)
}

/// The first save of a request where nothing holds it yet: what `create` names is created
/// with it, or nothing is. Every name is checked before anything is written.
pub fn create_http_request_in(
    engine: &Engine,
    draft: HttpRequestDraft,
    create: NewPlace,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&draft.name)?;
    let collection = create
        .collection
        .as_deref()
        .map(model::validated_name)
        .transpose()?;
    let folder = create
        .folder
        .as_deref()
        .map(model::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(store::create_request_in(
        &mut connection,
        &draft,
        &name,
        collection.as_deref(),
        folder.as_deref(),
        Utc::now(),
    )?)
}

pub fn get_http_request(engine: &Engine, id: String) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::get_request(&mut connection, &id)?)
}

pub fn save_http_request(
    engine: &Engine,
    id: String,
    patch: HttpRequestPatch,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = patch
        .name
        .as_deref()
        .map(model::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(store::save_request(
        &mut connection,
        &id,
        &patch,
        name.as_deref(),
        Utc::now(),
    )?)
}

pub fn rename_http_item(engine: &Engine, item: HttpItem, name: String) -> Result<(), AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::rename(&mut connection, &item, &name)?)
}

/// What a deletion would take with it, for the confirmation that comes first.
pub fn count_http_contents(engine: &Engine, item: HttpItem) -> Result<HttpContents, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::contents(&mut connection, &item)?)
}

pub fn delete_http_item(engine: &Engine, item: HttpItem) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::delete(&mut connection, &item)?)
}

/// The copy's name comes from the front: no user-visible word is Rust's.
pub fn duplicate_http_item(
    engine: &Engine,
    item: HttpItem,
    name: String,
) -> Result<HttpItem, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::duplicate(&mut connection, &item, &name, Utc::now())?)
}

pub fn move_http_item(engine: &Engine, item: HttpItem, place: HttpPlace) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::move_item(&mut connection, &item, &place)?)
}

pub fn http_settings(
    engine: &Engine,
    item: HttpItem,
) -> Result<model::ContainerSettings, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::settings(&mut connection, &item)?)
}

pub fn save_http_settings(
    engine: &Engine,
    item: HttpItem,
    settings: model::ContainerSettings,
) -> Result<(), AppError> {
    // Refused where it is typed: stored, it failed every request under it, one by one.
    if let Some((row, fault)) = settings
        .headers
        .iter()
        .zip(model::header_faults(&settings.headers))
        .find_map(|(row, fault)| Some((row, fault?)))
    {
        return Err(send::SendError::Header {
            name: row.key.trim().to_string(),
            fault,
            set_by: transport::SetBy::Request,
        }
        .into());
    }
    let db = &engine.db;

    let mut connection = lock(db)?;
    // The dialog does not hold the run order: the one the runner left is kept.
    let settings = model::ContainerSettings {
        run_order: store::settings(&mut connection, &item)?.run_order,
        ..settings
    };

    Ok(store::save_settings(&mut connection, &item, &settings)?)
}

/// What a request placed there inherits, and from where: « héritée de la collection … ».
pub fn inherited_http_settings(
    engine: &Engine,
    collection_id: String,
    folder_id: Option<String>,
) -> Result<settings::Inherited, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::inherited(
        &mut connection,
        &collection_id,
        folder_id.as_deref(),
    )?)
}

/// What a body implies, whether a JSON one reads, and whether its method seldom carries one:
/// the editor says the three as it is typed, and decides none of them.
pub async fn describe_http_body(
    body: model::RequestBody,
    method: model::HttpMethod,
) -> Result<BodyAnswer, AppError> {
    off_thread(move || BodyAnswer {
        unusual: body.is_unusual_on(method),
        content_type: settings::implied_content_type(&body).map(str::to_string),
        problem: match &body {
            model::RequestBody::Json { text } if !text.trim().is_empty() => {
                crate::json::parse::parse(text).err()
            }
            _ => None,
        },
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyAnswer {
    pub content_type: Option<String>,
    pub problem: Option<crate::json::parse::JsonError>,
    /// Something is sent as the body of a `GET` or a `HEAD`: sent all the same, and said.
    pub unusual: bool,
}

/// Each header's fault by its rank, or none: the table marks the row and says why. The rule is
/// the model's; the table holds none.
pub async fn check_http_headers(
    rows: Vec<model::KeyValue>,
) -> Result<Vec<Option<model::HeaderFault>>, AppError> {
    off_thread(move || model::header_faults(&rows)).await
}

/// `Nom: valeur` typed in a name, or lines pasted from a browser's network tab, as rows.
pub async fn read_http_headers(text: String) -> Result<Vec<model::KeyValue>, AppError> {
    off_thread(move || model::headers_from_text(&text)).await
}

/// What a request sends without a row of its own — the body's type, what it inherits, the
/// application's `User-Agent` — read as a send reads it: the editor lists the answer and
/// decides none of it.
pub fn implied_http_headers(
    engine: &Engine,
    kind: model::RequestKind,
    document: model::RequestDocument,
    collection_id: Option<String>,
    folder_id: Option<String>,
) -> Result<Vec<send::ImpliedHeader>, AppError> {
    let db = &engine.db;
    let inherited = match collection_id {
        Some(collection_id) => {
            let mut connection = lock(db)?;
            store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
        }
        None => settings::inherit(&[]),
    };
    Ok(send::implied(kind, &document, &inherited))
}

/// The edited side wins: the query string rewritten from the table, or the table from the URL.
pub async fn sync_http_query(
    url: String,
    params: Vec<model::KeyValue>,
    edited: query::QuerySide,
) -> Result<query::SyncedQuery, AppError> {
    off_thread(move || query::sync(&url, &params, edited)).await
}

pub fn reorder_http_collection(engine: &Engine, id: String, index: u32) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::reorder_collection(&mut connection, &id, index)?)
}

/// What it inherits is read under the lock, its files outside it, and the send waits on nothing
/// but the network and its scripts: a 30-second request stalls no other command.
pub async fn send_http_request(
    engine: Arc<Engine>,
    request: send::SendRequest,
) -> Result<send::SentResponse, AppError> {
    let mut flight = Flight::new(engine, &request.id);
    let answer = flight.send(request).await;
    flight.keep().await;
    answer
}

/// What a request inherits from its collection and folders: nothing outside a collection.
fn inherited_of(
    connection: &mut crate::db::Library,
    collection_id: Option<&str>,
    folder_id: Option<&str>,
) -> Result<settings::Inherited, crate::error::StorageError> {
    match collection_id {
        Some(collection_id) => store::inherited(connection, collection_id, folder_id),
        None => Ok(settings::inherit(&[])),
    }
}

/// What a request resolves its `{{variables}}` from, the nearest first: its own, its folders
/// and collection, the environment chosen — none when it is gone — and the globals.
fn resolver_for(
    connection: &mut crate::db::Library,
    own: &[environments::Variable],
    inherited: &settings::Inherited,
    environment_id: Option<&str>,
) -> Result<variables::Resolver, crate::error::StorageError> {
    Ok(variables::Resolver::new(&layers_for(
        connection,
        own,
        inherited,
        environment_id,
    )?))
}

fn layers_for(
    connection: &mut crate::db::Library,
    own: &[environments::Variable],
    inherited: &settings::Inherited,
    environment_id: Option<&str>,
) -> Result<Vec<variables::VariableLayer>, crate::error::StorageError> {
    use variables::{VariableLayer, VariableScope};

    let sets = environments::store::list(connection)?;
    let mut layers = vec![VariableLayer {
        scope: VariableScope::Request,
        from: String::new(),
        variables: own.to_vec(),
    }];
    layers.extend(inherited.variables.iter().cloned());
    if let Some(environment) = sets
        .environments
        .into_iter()
        .find(|environment| Some(environment.id.as_str()) == environment_id)
    {
        layers.push(VariableLayer {
            scope: VariableScope::Environment,
            from: environment.name,
            variables: environment.variables,
        });
    }
    layers.push(VariableLayer {
        scope: VariableScope::Globals,
        from: String::new(),
        variables: sets.globals,
    });
    Ok(layers)
}

/// The request written as code for another tool, from what a send would write: its variables
/// resolved as asked, a secret left as its `{{name}}` unless it is asked for in clear.
pub fn generate_http_code(
    engine: &Engine,
    request: CodeRequestOf,
) -> Result<codegen::GeneratedCode, AppError> {
    let db = &engine.db;
    let CodeRequestOf {
        target,
        options,
        kind,
        method,
        document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    let mut connection = lock(db)?;
    let inherited = inherited_of(
        &mut connection,
        collection_id.as_deref(),
        folder_id.as_deref(),
    )?;
    let layers = layers_for(
        &mut connection,
        &document.variables,
        &inherited,
        environment_id.as_deref(),
    )?;
    Ok(codegen::generate(
        target, options, kind, method, &document, &inherited, &layers,
    ))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRequestOf {
    pub target: codegen::CodeTarget,
    pub options: codegen::CodeOptions,
    pub kind: model::RequestKind,
    pub method: model::HttpMethod,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// The `{{variables}}` a request holds, each with what it resolves to and from where: the
/// editor colours them by it, and names the ones nothing sets before a send does.
pub fn http_variables(
    engine: &Engine,
    request: VariablesRequest,
) -> Result<Vec<variables::UsedVariable>, AppError> {
    let db = &engine.db;
    let VariablesRequest {
        kind,
        document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    let mut connection = lock(db)?;
    let inherited = inherited_of(
        &mut connection,
        collection_id.as_deref(),
        folder_id.as_deref(),
    )?;
    let resolver = resolver_for(
        &mut connection,
        &document.variables,
        &inherited,
        environment_id.as_deref(),
    )?;
    Ok(resolver.used(&variables::texts(kind, &document, &inherited)))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesRequest {
    pub kind: model::RequestKind,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// The order a collection or a folder runs in: the one last arranged, the requests added since
/// at its end.
pub fn http_run_order(engine: &Engine, item: HttpItem) -> Result<Vec<model::RunEntry>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let tree = store::tree(&mut connection)?;
    let saved = store::settings(&mut connection, &item)?.run_order;

    Ok(runner::run_order(
        &saved,
        &runner::placed_under(&tree, &item),
    ))
}

/// The order the runner was left in, kept with the collection's or the folder's settings.
pub fn save_http_run_order(
    engine: &Engine,
    item: HttpItem,
    order: Vec<model::RunEntry>,
) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let settings = model::ContainerSettings {
        run_order: order,
        ..store::settings(&mut connection, &item)?
    };

    Ok(store::save_settings(&mut connection, &item, &settings)?)
}

/// A collection or a folder run, one request after the other, each sent as a send is — its
/// scripts, its tests, its extractions — but written in no history. Each step is said as an
/// event; answers once the run is over. The lock is held for reads alone, never over a send.
pub async fn run_http_collection(
    engine: Arc<Engine>,
    run: runner::RunRequest,
) -> Result<(), AppError> {
    let tree = db::run(Arc::clone(&engine), |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        Ok(store::tree(&mut connection)?)
    })
    .await?;
    let placed = runner::placed_under(&tree, &run.item);
    let order: Vec<runner::Placed> = run
        .order
        .iter()
        .filter_map(|id| placed.iter().find(|known| &known.id == id).cloned())
        .collect();
    let rows = run
        .data
        .as_ref()
        .map(|data| data.rows.clone())
        .unwrap_or_default();
    let iterations = if rows.is_empty() {
        run.iterations.max(1)
    } else {
        u32::try_from(rows.len()).unwrap_or(u32::MAX)
    };
    let runs = &engine.runs;
    runs.start(&run.id);
    let started = std::time::Instant::now();
    let mut overlay = Vec::new();
    let (mut cancelled, mut capped) = (false, false);
    'run: for iteration in 0..iterations {
        let row = rows.get(iteration as usize).cloned().unwrap_or_default();
        let mut at = (!order.is_empty()).then_some(0);
        let mut steps = 0_u32;
        while let Some(index) = at {
            if runs.cancelled(&run.id) {
                cancelled = true;
                break 'run;
            }
            steps += 1;
            let placed = &order[index];
            let step = RunStep {
                run: &run,
                iteration,
                iterations,
                index,
                steps,
                placed,
                row: &row,
            };
            let Some((result, next)) = run_step(&engine, step, &mut overlay).await else {
                cancelled = true;
                break 'run;
            };
            let failed = result.failed();
            say_run(
                &engine,
                &run.id,
                runner::RunStep::Ended {
                    result: Box::new(result),
                },
            );
            if run.stop_on_failure && failed {
                break 'run;
            }
            // `setNextRequest(null)` ends the run, as it ends a collection run elsewhere.
            if next == Some(runner::Next::End) {
                break 'run;
            }
            at = runner::next_index(&order, index, next.as_ref());
            if runner::capped(steps, at) {
                capped = true;
                break 'run;
            }
            if at.is_some() && !wait(runs, &run.id, run.delay_ms).await {
                cancelled = true;
                break 'run;
            }
        }
    }
    runs.finish(&run.id);
    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    say_run(
        &engine,
        &run.id,
        runner::RunStep::Finished {
            cancelled,
            millis,
            capped,
        },
    );
    Ok(())
}

/// « Annuler » on a run: the request under way is cancelled with it. `false` when no run of that
/// id was under way.
pub fn cancel_http_run(engine: &Engine, id: String) -> Result<bool, AppError> {
    let Some(at) = engine.runs.cancel(&id) else {
        return Ok(false);
    };
    if let Some(send) = at {
        engine.sending.cancel(&send);
    }
    Ok(true)
}

/// A run's data file read: a CSV with its header, or a JSON array of objects.
pub async fn read_http_run_data(path: String) -> Result<runner::RunData, AppError> {
    off_thread(move || runner::read_file(&path)).await?
}

/// `false` when that send had already ended.
pub fn cancel_http_send(engine: &Engine, id: String) -> Result<bool, AppError> {
    Ok(engine.sending.cancel(&id))
}

/// The whole body of the last answer to `id`, not the text shown: `false` when there is none.
pub async fn save_http_response(
    engine: Arc<Engine>,
    id: String,
    path: String,
) -> Result<bool, AppError> {
    crate::transfer::model::validate_path(&path)?;
    off_thread(move || engine.sending.save(&id, Path::new(&path)).context(&path))
        .await?
        .map_err(Into::into)
}

/// A closed tab lets its last body go.
pub fn forget_http_response(engine: &Engine, id: String) -> Result<(), AppError> {
    engine.sending.forget(&id);
    Ok(())
}

/// The last answer to `id` as an image to show, when it is one.
pub async fn http_response_image(
    engine: Arc<Engine>,
    id: String,
) -> Result<Option<String>, AppError> {
    off_thread(move || engine.sending.image(&id)).await
}

/// An answer's text made a note: the secrets the request sent, masked wherever it says them back.
pub async fn http_response_note(
    engine: Arc<Engine>,
    id: Option<String>,
    exchange: send::Exchange,
    text: String,
) -> Result<send::NoteText, AppError> {
    off_thread(move || engine.sending.note_text(id.as_deref(), &exchange, &text)).await
}

/// What was sent, newest first, a group a local day.
pub fn http_history(
    engine: &Engine,
    tz_offset_minutes: i32,
) -> Result<Vec<history::HistoryDay>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::by_day(
        history::store::list(&mut connection)?,
        tz_offset_minutes,
    ))
}

pub fn http_history_entry(engine: &Engine, id: String) -> Result<history::HistoryEntry, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::entry(&mut connection, &id)?)
}

/// What « Vider l'historique » says it removes.
pub fn count_http_history(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::count(&mut connection)?)
}

pub fn clear_http_history(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::clear(&mut connection)?)
}

/// One entry gone, answered whole: its undo hands it back to `restore_http_history_entry`.
pub fn delete_http_history_entry(
    engine: &Engine,
    id: String,
) -> Result<history::HistoryEntry, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::delete(&mut connection, &id)?)
}

pub fn restore_http_history_entry(
    engine: &Engine,
    entry: history::HistoryEntry,
) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::restore(&mut connection, &entry)?)
}

/// A send put back onto the request it came from, as changes to save or not, the request's own
/// secrets where the record holds a mask. `None` when that request is gone, or the send came
/// from a draft: the front then opens one, as `http_history_draft` rebuilds it.
pub fn http_history_onto_request(
    engine: &Engine,
    id: String,
) -> Result<Option<history::HistoryDraft>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let entry = history::store::entry(&mut connection, &id)?;
    let Some(request_id) = entry.item.request_id.as_deref() else {
        return Ok(None);
    };
    let current = match store::get_request(&mut connection, request_id) {
        Ok(current) => current,
        Err(crate::error::StorageError::HttpItemNotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let above = store::inherited(
        &mut connection,
        &current.collection_id,
        current.folder_id.as_deref(),
    )?;
    Ok(Some(history::onto(&entry, &current, &above.headers)))
}

/// An entry rebuilt as a request to send again or save: its secrets are typed again.
pub fn http_history_draft(engine: &Engine, id: String) -> Result<history::HistoryDraft, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::draft_of(&history::store::entry(
        &mut connection,
        &id,
    )?))
}

/// The operations a GraphQL document names, and why its variables do not read.
pub async fn describe_graphql(
    document: graphql::GraphqlDocument,
) -> Result<graphql::GraphqlAnswer, AppError> {
    off_thread(move || graphql::describe(&document)).await
}

/// The variables with a `null` for each one the operation declares and they lack: « Ajouter ».
pub async fn add_graphql_variables(document: graphql::GraphqlDocument) -> Result<String, AppError> {
    off_thread(move || graphql::with_missing(&document)).await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketRequest {
    /// Chosen by the front: what it sends on and closes, and what each event names.
    pub id: String,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// Its headers and auth composed like a request's, under the lock for what it inherits; the
/// handshake waited on without it. Answers once the socket is open.
pub async fn connect_websocket(
    engine: Arc<Engine>,
    request: WebsocketRequest,
) -> Result<(), AppError> {
    let WebsocketRequest {
        id,
        mut document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    document.body = model::RequestBody::None;
    let outgoing = db::run(Arc::clone(&engine), move |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        let inherited = inherited_of(
            &mut connection,
            collection_id.as_deref(),
            folder_id.as_deref(),
        )?;
        let resolver = resolver_for(
            &mut connection,
            &document.variables,
            &inherited,
            environment_id.as_deref(),
        )?;
        let (mut document, inherited) = variables::resolve(&document, &inherited, &resolver);
        document.url = websocket::socket_url(&document.url)?;
        Ok(send::compose(
            model::RequestKind::Http,
            model::HttpMethod::Get,
            &document,
            &inherited,
        )
        .map(|outgoing| (outgoing, document.websocket.protocols))?)
    })
    .await?;

    let (outgoing, protocols) = outgoing;
    Ok(engine
        .sockets
        .connect(
            &engine.runtime,
            Arc::clone(&engine.events),
            id,
            outgoing,
            &protocols,
        )
        .await?)
}

/// `false` when no socket is open under that id.
pub fn send_websocket(engine: &Engine, id: String, text: String) -> Result<bool, AppError> {
    Ok(engine.sockets.send(&id, text))
}

pub fn close_websocket(engine: &Engine, id: String) -> Result<bool, AppError> {
    Ok(engine.sockets.close(&id))
}

/// The jar by domain, for the manager.
pub fn http_cookies(engine: &Engine) -> Result<Vec<cookies::store::CookieDomain>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::by_domain(&mut connection)?)
}

pub fn delete_http_cookie(engine: &Engine, id: String) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::delete(&mut connection, &id)?)
}

pub fn delete_http_cookie_domain(engine: &Engine, domain: String) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::delete_domain(&mut connection, &domain)?)
}

/// What « Vider le bocal » says it removes.
pub fn count_http_cookies(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::count(&mut connection)?)
}

pub fn clear_http_cookies(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::clear(&mut connection)?)
}

/// The globals and every environment, in their order.
pub fn http_environments(engine: &Engine) -> Result<environments::HttpEnvironments, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::list(&mut connection)?)
}

pub fn create_http_environment(
    engine: &Engine,
    name: String,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = environments::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(environments::store::create(
        &mut connection,
        &name,
        Utc::now(),
    )?)
}

pub fn save_http_environment(
    engine: &Engine,
    id: String,
    patch: environments::EnvironmentPatch,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = patch
        .name
        .as_deref()
        .map(environments::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(environments::store::save(
        &mut connection,
        &id,
        &patch,
        name.as_deref(),
    )?)
}

pub fn save_http_globals(
    engine: &Engine,
    variables: Vec<environments::Variable>,
) -> Result<Vec<environments::Variable>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::save_globals(
        &mut connection,
        &variables,
        Utc::now(),
    )?)
}

/// The copy's name comes from the front: no user-visible word is Rust's.
pub fn duplicate_http_environment(
    engine: &Engine,
    id: String,
    name: String,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = environments::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(environments::store::duplicate(
        &mut connection,
        &id,
        &name,
        Utc::now(),
    )?)
}

pub fn delete_http_environment(engine: &Engine, id: String) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::delete(&mut connection, &id)?)
}

/// Each variable's fault by its rank, or none: the table marks the row, the rule is the model's.
pub async fn check_http_variables(
    variables: Vec<environments::Variable>,
) -> Result<Vec<Option<environments::VariableFault>>, AppError> {
    off_thread(move || environments::variable_faults(&variables)).await
}
