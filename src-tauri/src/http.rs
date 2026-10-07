#![allow(clippy::needless_pass_by_value)]

//! The HTTP client's collections: they belong to the library, not to a space, and are sealed
//! like the notes. A collection travels in a file of its own, never in the library export.

pub mod model;
pub mod query;
pub mod response;
pub mod send;
pub mod settings;
pub mod store;

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
        method,
        document,
        collection_id,
        folder_id,
    } = request;
    let outgoing = blocking(app.clone(), move |_, db| {
        let inherited = match collection_id {
            Some(collection_id) => {
                let mut connection = lock(db)?;
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        Ok(send::compose(method, &document, &inherited)?)
    })
    .await?;

    Ok(app.state::<send::Sending>().run(&id, outgoing).await?)
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
