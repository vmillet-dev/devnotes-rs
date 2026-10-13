#![allow(clippy::needless_pass_by_value)]

//! The HTTP client's collections: they belong to the library, not to a space, and are sealed
//! like the notes. A collection travels in a file of its own, never in the library export.

pub mod cookies;
pub mod environments;
pub mod graphql;
pub mod history;
pub mod model;
pub mod query;
pub mod response;
pub mod send;
pub mod settings;
pub mod store;
pub mod transport;
pub mod variables;
pub mod websocket;

use std::path::Path;

use chrono::Utc;
use tauri::{AppHandle, Manager, Runtime};

use crate::db::{blocking, lock};
use crate::error::{AppError, FileContext};
use crate::tools::off_thread;
use model::{
    HttpCollection, HttpContents, HttpFolder, HttpItem, HttpPlace, HttpRequest, HttpRequestDraft,
    HttpRequestPatch, HttpTree, NewPlace,
};

#[tauri::command]
#[specta::specta]
pub async fn http_tree<R: Runtime>(app: AppHandle<R>) -> Result<HttpTree, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::tree(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_collection<R: Runtime>(
    name: String,
    app: AppHandle<R>,
) -> Result<HttpCollection, AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(store::create_collection(
            &mut connection,
            &name,
            Utc::now(),
        )?)
    })
    .await
}

/// `parent_id` `None` puts it at the collection's root.
#[tauri::command]
#[specta::specta]
pub async fn create_http_folder<R: Runtime>(
    collection_id: String,
    parent_id: Option<String>,
    name: String,
    app: AppHandle<R>,
) -> Result<HttpFolder, AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(store::create_folder(
            &mut connection,
            &collection_id,
            parent_id.as_deref(),
            &name,
            Utc::now(),
        )?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_request<R: Runtime>(
    draft: HttpRequestDraft,
    app: AppHandle<R>,
) -> Result<HttpRequest, AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&draft.name)?;

        let mut connection = lock(db)?;

        Ok(store::create_request(
            &mut connection,
            &draft,
            &name,
            Utc::now(),
        )?)
    })
    .await
}

/// The first save of a request where nothing holds it yet: what `create` names is created
/// with it, or nothing is. Every name is checked before anything is written.
#[tauri::command]
#[specta::specta]
pub async fn create_http_request_in<R: Runtime>(
    draft: HttpRequestDraft,
    create: NewPlace,
    app: AppHandle<R>,
) -> Result<HttpRequest, AppError> {
    blocking(app, move |_, db| {
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
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_http_request<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<HttpRequest, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::get_request(&mut connection, &id)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_request<R: Runtime>(
    id: String,
    patch: HttpRequestPatch,
    app: AppHandle<R>,
) -> Result<HttpRequest, AppError> {
    blocking(app, move |_, db| {
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
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_http_item<R: Runtime>(
    item: HttpItem,
    name: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(store::rename(&mut connection, &item, &name)?)
    })
    .await
}

/// What a deletion would take with it, for the confirmation that comes first.
#[tauri::command]
#[specta::specta]
pub async fn count_http_contents<R: Runtime>(
    item: HttpItem,
    app: AppHandle<R>,
) -> Result<HttpContents, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::contents(&mut connection, &item)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_item<R: Runtime>(
    item: HttpItem,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::delete(&mut connection, &item)?)
    })
    .await
}

/// The copy's name comes from the front: no user-visible word is Rust's.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_http_item<R: Runtime>(
    item: HttpItem,
    name: String,
    app: AppHandle<R>,
) -> Result<HttpItem, AppError> {
    blocking(app, move |_, db| {
        let name = model::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(store::duplicate(&mut connection, &item, &name, Utc::now())?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn move_http_item<R: Runtime>(
    item: HttpItem,
    place: HttpPlace,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::move_item(&mut connection, &item, &place)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn http_settings<R: Runtime>(
    item: HttpItem,
    app: AppHandle<R>,
) -> Result<model::ContainerSettings, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::settings(&mut connection, &item)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_settings<R: Runtime>(
    item: HttpItem,
    settings: model::ContainerSettings,
    app: AppHandle<R>,
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
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::save_settings(&mut connection, &item, &settings)?)
    })
    .await
}

/// What a request placed there inherits, and from where: « héritée de la collection … ».
#[tauri::command]
#[specta::specta]
pub async fn inherited_http_settings<R: Runtime>(
    collection_id: String,
    folder_id: Option<String>,
    app: AppHandle<R>,
) -> Result<settings::Inherited, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::inherited(
            &mut connection,
            &collection_id,
            folder_id.as_deref(),
        )?)
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

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BodyAnswer {
    pub content_type: Option<String>,
    pub problem: Option<crate::json::parse::JsonError>,
    /// Something is sent as the body of a `GET` or a `HEAD`: sent all the same, and said.
    pub unusual: bool,
}

/// Each header's fault by its rank, or none: the table marks the row and says why. The rule is
/// the model's; the table holds none.
#[tauri::command]
#[specta::specta]
pub async fn check_http_headers(
    rows: Vec<model::KeyValue>,
) -> Result<Vec<Option<model::HeaderFault>>, AppError> {
    off_thread(move || model::header_faults(&rows)).await
}

/// `Nom: valeur` typed in a name, or lines pasted from a browser's network tab, as rows.
#[tauri::command]
#[specta::specta]
pub async fn read_http_headers(text: String) -> Result<Vec<model::KeyValue>, AppError> {
    off_thread(move || model::headers_from_text(&text)).await
}

/// What a request sends without a row of its own — the body's type, what it inherits, the
/// application's `User-Agent` — read as a send reads it: the editor lists the answer and
/// decides none of it.
#[tauri::command]
#[specta::specta]
pub async fn implied_http_headers<R: Runtime>(
    kind: model::RequestKind,
    document: model::RequestDocument,
    collection_id: Option<String>,
    folder_id: Option<String>,
    app: AppHandle<R>,
) -> Result<Vec<send::ImpliedHeader>, AppError> {
    blocking(app, move |_, db| {
        let inherited = match collection_id {
            Some(collection_id) => {
                let mut connection = lock(db)?;
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        Ok(send::implied(kind, &document, &inherited))
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
    off_thread(move || query::sync(&url, &params, edited)).await
}

#[tauri::command]
#[specta::specta]
pub async fn reorder_http_collection<R: Runtime>(
    id: String,
    index: u32,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::reorder_collection(&mut connection, &id, index)?)
    })
    .await
}

/// What it inherits is read under the lock, its files outside it, and the send waits on nothing
/// but the network: a 30-second request stalls no other command.
#[tauri::command]
#[specta::specta]
pub async fn send_http_request<R: Runtime>(
    request: send::SendRequest,
    app: AppHandle<R>,
) -> Result<send::SentResponse, AppError> {
    let send::SendRequest {
        id,
        kind,
        method,
        document,
        collection_id,
        folder_id,
        name,
        request_id,
        environment_id,
    } = request;
    let outgoing = blocking(app.clone(), move |_, db| {
        let mut connection = lock(db)?;
        let inherited = match collection_id {
            Some(collection_id) => {
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        let resolver = resolver_for(
            &mut connection,
            &document.variables,
            &inherited,
            environment_id.as_deref(),
        )?;
        let (document, inherited) = variables::resolve(&document, &inherited, &resolver);
        let mut outgoing = send::compose(kind, method, &document, &inherited)?;
        if outgoing.transport.use_cookies {
            outgoing.jar = Some(cookies::store::jar(&mut connection, Utc::now())?);
        }
        Ok(outgoing)
    })
    .await?;

    let exchange = send::exchange(&outgoing);
    let secrets = outgoing.secrets.clone();
    let (answer, cookies_set) = app.state::<send::Sending>().run(&id, outgoing).await;
    let outcome = match &answer {
        Ok(response) => Ok(response.clone()),
        Err(send::SendError::Cancelled) => return Err(send::SendError::Cancelled.into()),
        Err(error) => {
            let error = AppError::from(error.clone());
            Err((error.code, error.params))
        }
    };
    let sent = history::Sent {
        kind,
        name,
        request_id,
        exchange,
        secrets,
        outcome,
    };
    keep(app, sent, cookies_set).await;

    Ok(answer?)
}

/// What a request resolves its `{{variables}}` from, the nearest first: its own, its folders
/// and collection, the environment chosen — none when it is gone — and the globals.
fn resolver_for(
    connection: &mut crate::db::Library,
    own: &[environments::Variable],
    inherited: &settings::Inherited,
    environment_id: Option<&str>,
) -> Result<variables::Resolver, crate::error::StorageError> {
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
    Ok(variables::Resolver::new(&layers))
}

/// The `{{variables}}` a request holds, each with what it resolves to and from where: the
/// editor colours them by it, and names the ones nothing sets before a send does.
#[tauri::command]
#[specta::specta]
pub async fn http_variables<R: Runtime>(
    request: VariablesRequest,
    app: AppHandle<R>,
) -> Result<Vec<variables::UsedVariable>, AppError> {
    blocking(app, move |_, db| {
        let VariablesRequest {
            kind,
            document,
            collection_id,
            folder_id,
            environment_id,
        } = request;
        let mut connection = lock(db)?;
        let inherited = match collection_id {
            Some(collection_id) => {
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        let resolver = resolver_for(
            &mut connection,
            &document.variables,
            &inherited,
            environment_id.as_deref(),
        )?;
        Ok(resolver.used(&variables::texts(kind, &document, &inherited)))
    })
    .await
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VariablesRequest {
    pub kind: model::RequestKind,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// The history and the jar, written under the lock again, and never at the answer's expense.
async fn keep<R: Runtime>(
    app: AppHandle<R>,
    sent: history::Sent,
    cookies_set: Vec<cookies::Change>,
) {
    let kept = blocking(app, move |_, db| {
        let now = Utc::now();
        let (summary, record) = history::record_of(&sent);
        let mut connection = lock(db)?;
        history::store::record(
            &mut connection,
            sent.request_id.as_deref(),
            &summary,
            &record,
            now,
        )?;
        cookies::store::apply(&mut connection, &cookies_set, now)?;
        Ok(())
    })
    .await;
    if let Err(error) = kept {
        log::warn!(
            "A send's history or cookies were not kept: {}",
            error.detail
        );
    }
}

/// `false` when that send had already ended.
#[tauri::command]
#[specta::specta]
pub async fn cancel_http_send<R: Runtime>(id: String, app: AppHandle<R>) -> Result<bool, AppError> {
    Ok(app.state::<send::Sending>().cancel(&id))
}

/// The whole body of the last answer to `id`, not the text shown: `false` when there is none.
#[tauri::command]
#[specta::specta]
pub async fn save_http_response<R: Runtime>(
    id: String,
    path: String,
    app: AppHandle<R>,
) -> Result<bool, AppError> {
    crate::transfer::model::validate_path(&path)?;
    off_thread(move || {
        app.state::<send::Sending>()
            .save(&id, Path::new(&path))
            .context(&path)
    })
    .await?
    .map_err(Into::into)
}

/// A closed tab lets its last body go.
#[tauri::command]
#[specta::specta]
pub async fn forget_http_response<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    app.state::<send::Sending>().forget(&id);
    Ok(())
}

/// The last answer to `id` as an image to show, when it is one.
#[tauri::command]
#[specta::specta]
pub async fn http_response_image<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<Option<String>, AppError> {
    off_thread(move || app.state::<send::Sending>().image(&id)).await
}

/// What was sent, newest first, a group a local day.
#[tauri::command]
#[specta::specta]
pub async fn http_history<R: Runtime>(
    tz_offset_minutes: i32,
    app: AppHandle<R>,
) -> Result<Vec<history::HistoryDay>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::by_day(
            history::store::list(&mut connection)?,
            tz_offset_minutes,
        ))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn http_history_entry<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<history::HistoryEntry, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::store::entry(&mut connection, &id)?)
    })
    .await
}

/// What « Vider l'historique » says it removes.
#[tauri::command]
#[specta::specta]
pub async fn count_http_history<R: Runtime>(app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::store::count(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_http_history<R: Runtime>(app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::store::clear(&mut connection)?)
    })
    .await
}

/// One entry gone, answered whole: its undo hands it back to `restore_http_history_entry`.
#[tauri::command]
#[specta::specta]
pub async fn delete_http_history_entry<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<history::HistoryEntry, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::store::delete(&mut connection, &id)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn restore_http_history_entry<R: Runtime>(
    entry: history::HistoryEntry,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::store::restore(&mut connection, &entry)?)
    })
    .await
}

/// A send put back onto the request it came from, as changes to save or not, the request's own
/// secrets where the record holds a mask. `None` when that request is gone, or the send came
/// from a draft: the front then opens one, as `http_history_draft` rebuilds it.
#[tauri::command]
#[specta::specta]
pub async fn http_history_onto_request<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<Option<history::HistoryDraft>, AppError> {
    blocking(app, move |_, db| {
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
    })
    .await
}

/// An entry rebuilt as a request to send again or save: its secrets are typed again.
#[tauri::command]
#[specta::specta]
pub async fn http_history_draft<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<history::HistoryDraft, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(history::draft_of(&history::store::entry(
            &mut connection,
            &id,
        )?))
    })
    .await
}

/// The operations a GraphQL document names, and why its variables do not read.
#[tauri::command]
#[specta::specta]
pub async fn describe_graphql(
    document: graphql::GraphqlDocument,
) -> Result<graphql::GraphqlAnswer, AppError> {
    off_thread(move || graphql::describe(&document)).await
}

/// The variables with a `null` for each one the operation declares and they lack: « Ajouter ».
#[tauri::command]
#[specta::specta]
pub async fn add_graphql_variables(document: graphql::GraphqlDocument) -> Result<String, AppError> {
    off_thread(move || graphql::with_missing(&document)).await
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
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
#[tauri::command]
#[specta::specta]
pub async fn connect_websocket<R: Runtime>(
    request: WebsocketRequest,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    let WebsocketRequest {
        id,
        mut document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    document.body = model::RequestBody::None;
    let outgoing = blocking(app.clone(), move |_, db| {
        let mut connection = lock(db)?;
        let inherited = match collection_id {
            Some(collection_id) => {
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
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
    Ok(app
        .state::<websocket::Sockets>()
        .connect(app.clone(), id, outgoing, &protocols)
        .await?)
}

/// `false` when no socket is open under that id.
#[tauri::command]
#[specta::specta]
pub async fn send_websocket<R: Runtime>(
    id: String,
    text: String,
    app: AppHandle<R>,
) -> Result<bool, AppError> {
    Ok(app.state::<websocket::Sockets>().send(&id, text))
}

#[tauri::command]
#[specta::specta]
pub async fn close_websocket<R: Runtime>(id: String, app: AppHandle<R>) -> Result<bool, AppError> {
    Ok(app.state::<websocket::Sockets>().close(&id))
}

/// The jar by domain, for the manager.
#[tauri::command]
#[specta::specta]
pub async fn http_cookies<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<cookies::store::CookieDomain>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(cookies::store::by_domain(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_cookie<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(cookies::store::delete(&mut connection, &id)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_cookie_domain<R: Runtime>(
    domain: String,
    app: AppHandle<R>,
) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(cookies::store::delete_domain(&mut connection, &domain)?)
    })
    .await
}

/// What « Vider le bocal » says it removes.
#[tauri::command]
#[specta::specta]
pub async fn count_http_cookies<R: Runtime>(app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(cookies::store::count(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_http_cookies<R: Runtime>(app: AppHandle<R>) -> Result<u32, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(cookies::store::clear(&mut connection)?)
    })
    .await
}

/// The globals and every environment, in their order.
#[tauri::command]
#[specta::specta]
pub async fn http_environments<R: Runtime>(
    app: AppHandle<R>,
) -> Result<environments::HttpEnvironments, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(environments::store::list(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_http_environment<R: Runtime>(
    name: String,
    app: AppHandle<R>,
) -> Result<environments::Environment, AppError> {
    blocking(app, move |_, db| {
        let name = environments::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(environments::store::create(
            &mut connection,
            &name,
            Utc::now(),
        )?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_environment<R: Runtime>(
    id: String,
    patch: environments::EnvironmentPatch,
    app: AppHandle<R>,
) -> Result<environments::Environment, AppError> {
    blocking(app, move |_, db| {
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
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_http_globals<R: Runtime>(
    variables: Vec<environments::Variable>,
    app: AppHandle<R>,
) -> Result<Vec<environments::Variable>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(environments::store::save_globals(
            &mut connection,
            &variables,
            Utc::now(),
        )?)
    })
    .await
}

/// The copy's name comes from the front: no user-visible word is Rust's.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_http_environment<R: Runtime>(
    id: String,
    name: String,
    app: AppHandle<R>,
) -> Result<environments::Environment, AppError> {
    blocking(app, move |_, db| {
        let name = environments::validated_name(&name)?;

        let mut connection = lock(db)?;

        Ok(environments::store::duplicate(
            &mut connection,
            &id,
            &name,
            Utc::now(),
        )?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_http_environment<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(environments::store::delete(&mut connection, &id)?)
    })
    .await
}

/// Each variable's fault by its rank, or none: the table marks the row, the rule is the model's.
#[tauri::command]
#[specta::specta]
pub async fn check_http_variables(
    variables: Vec<environments::Variable>,
) -> Result<Vec<Option<environments::VariableFault>>, AppError> {
    off_thread(move || environments::variable_faults(&variables)).await
}
