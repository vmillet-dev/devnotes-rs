#![allow(clippy::needless_pass_by_value)]

//! The HTTP client's collections: they belong to the library, not to a space, and are sealed
//! like the notes. A collection travels in a file of its own, never in the library export.

pub mod graphql;
pub mod history;
pub mod model;
pub mod query;
pub mod response;
pub mod send;
pub mod settings;
pub mod store;
pub mod websocket;

use std::path::Path;

use chrono::Utc;
use tauri::{AppHandle, Manager, Runtime};

use crate::db::{blocking, lock};
use crate::error::{AppError, FileContext};
use crate::tools::off_thread;
use model::{
    HttpCollection, HttpContents, HttpFolder, HttpItem, HttpPlace, HttpRequest, HttpRequestDraft,
    HttpRequestPatch, HttpTree,
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

/// What a body implies, and whether a JSON one reads: the editor says both as it is typed.
#[tauri::command]
#[specta::specta]
pub async fn describe_http_body(body: model::RequestBody) -> Result<BodyAnswer, AppError> {
    off_thread(move || BodyAnswer {
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
    } = request;
    let outgoing = blocking(app.clone(), move |_, db| {
        let inherited = match collection_id {
            Some(collection_id) => {
                let mut connection = lock(db)?;
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        Ok(send::compose(kind, method, &document, &inherited)?)
    })
    .await?;

    let exchange = send::exchange(&outgoing);
    let secrets = outgoing.secrets.clone();
    let answer = app.state::<send::Sending>().run(&id, outgoing).await;
    let outcome = match &answer {
        Ok(response) => Ok(response.clone()),
        Err(send::SendError::Cancelled) => return Err(send::SendError::Cancelled.into()),
        Err(error) => Err(AppError::from(error.clone()).code),
    };
    let sent = history::Sent {
        name,
        request_id,
        exchange,
        secrets,
        outcome,
    };
    // Written under the lock again, and never at the answer's expense.
    let recorded = blocking(app, move |_, db| {
        let (summary, record) = history::record_of(&sent);
        let mut connection = lock(db)?;
        history::store::record(
            &mut connection,
            method,
            sent.request_id.as_deref(),
            &summary,
            &record,
            Utc::now(),
        )?;
        Ok(())
    })
    .await;
    if let Err(error) = recorded {
        log::warn!("A send was not recorded in the history: {}", error.detail);
    }

    Ok(answer?)
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

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketRequest {
    /// Chosen by the front: what it sends on and closes, and what each event names.
    pub id: String,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
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
    } = request;
    document.url = websocket::socket_url(&document.url);
    document.body = model::RequestBody::None;
    let outgoing = blocking(app.clone(), move |_, db| {
        let inherited = match collection_id {
            Some(collection_id) => {
                let mut connection = lock(db)?;
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
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
