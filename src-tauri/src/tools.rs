//! The Outils area: each tool a pure function of its module, a request in and an answer out.
//! A tool's own failure — a line that does not parse — is part of its answer; `AppError` is left
//! for the unexpected. No lock and no store: nothing here reads the library.

pub mod checks;
pub mod colour;
pub mod convert;
pub mod cron;
pub mod dates;
pub mod diff;
pub mod encoding;
pub(crate) mod files;
pub mod generate;
pub mod hash;
pub mod identifiers;
pub mod jwt;
pub(crate) mod numbers;
pub mod percentages;
pub mod permissions;
pub mod random;
pub mod sizes;
pub mod stats;
pub mod text;
pub mod text_diff;
pub mod url_parts;
pub mod zones;

use crate::error::{AppError, StorageError};

/// Off the window's thread even for a tool that answers in a microsecond: a large paste does not.
async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn convert_case(request: text::CaseRequest) -> Result<text::CaseAnswer, AppError> {
    off_thread(move || text::convert_case(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn fix_line_breaks(
    request: text::LineBreaksRequest,
) -> Result<text::LineBreaksAnswer, AppError> {
    off_thread(move || text::fix_line_breaks(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn parse_url(text: String) -> Result<url_parts::UrlAnswer, AppError> {
    off_thread(move || url_parts::parse_url(&text)).await
}

#[tauri::command]
#[specta::specta]
pub async fn url_codec(
    request: encoding::UrlCodecRequest,
) -> Result<encoding::UrlCodecAnswer, AppError> {
    off_thread(move || encoding::url_codec(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn encode_base64(
    text: String,
    options: encoding::Base64Options,
) -> Result<String, AppError> {
    off_thread(move || encoding::encode_base64(&text, options)).await
}

/// By its path: the bytes are read here and never cross the bridge.
#[tauri::command]
#[specta::specta]
pub async fn encode_base64_file(
    path: String,
    options: encoding::Base64Options,
) -> Result<encoding::Base64FileAnswer, AppError> {
    off_thread(move || encoding::encode_base64_file(&path, options)).await
}

#[tauri::command]
#[specta::specta]
pub async fn decode_base64(
    text: String,
    options: encoding::Base64Options,
) -> Result<encoding::Base64Decoded, AppError> {
    off_thread(move || encoding::decode_base64(&text, options)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_base64(
    text: String,
    options: encoding::Base64Options,
    path: String,
) -> Result<encoding::Base64Saved, AppError> {
    off_thread(move || encoding::save_base64(&text, options, &path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn hash_input(request: hash::HashRequest) -> Result<hash::HashAnswer, AppError> {
    off_thread(move || hash::hash(request)).await
}

/// No randomness from the system is the unexpected: nothing weaker is drawn in its place.
#[tauri::command]
#[specta::specta]
pub async fn generate_passwords(
    request: random::PasswordRequest,
) -> Result<random::PasswordAnswer, AppError> {
    off_thread(move || random::passwords(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn generate_identifiers(
    request: identifiers::IdentifiersRequest,
) -> Result<Vec<String>, AppError> {
    off_thread(move || identifiers::generate(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_identifier(text: String) -> Result<identifiers::IdInspection, AppError> {
    off_thread(move || identifiers::inspect(&text)).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_colour(
    request: colour::ColourRequest,
) -> Result<colour::ColourAnswer, AppError> {
    off_thread(move || colour::describe(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn convert_data(
    request: convert::ConvertRequest,
) -> Result<convert::ConvertAnswer, AppError> {
    off_thread(move || convert::convert(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn generate_json(
    request: generate::GenerateRequest,
) -> Result<generate::GenerateAnswer, AppError> {
    off_thread(move || generate::generate(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn lorem_ipsum(
    request: generate::LoremRequest,
) -> Result<generate::LoremAnswer, AppError> {
    off_thread(move || generate::lorem(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn diff_text(
    request: text_diff::TextDiffRequest,
) -> Result<text_diff::TextDiffAnswer, AppError> {
    off_thread(move || text_diff::diff(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn diff_json(request: diff::JsonDiffRequest) -> Result<diff::JsonDiffAnswer, AppError> {
    off_thread(move || diff::diff(&request)).await
}

/// A date written without an offset is read in the machine's zone, as the person typing it means.
#[tauri::command]
#[specta::specta]
pub async fn describe_instant(
    request: dates::InstantRequest,
) -> Result<dates::InstantAnswer, AppError> {
    off_thread(move || dates::describe(&request, &chrono::Local)).await
}

#[tauri::command]
#[specta::specta]
pub async fn current_instant() -> Result<String, AppError> {
    off_thread(dates::now).await
}

/// The system's zone by its IANA name, when it gives one: the database itself is compiled in.
fn local_zone() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

#[tauri::command]
#[specta::specta]
pub async fn search_time_zones(query: String) -> Result<Vec<zones::ZoneEntry>, AppError> {
    off_thread(move || zones::search(&query, chrono::Utc::now())).await
}

#[tauri::command]
#[specta::specta]
pub async fn place_in_zones(request: zones::ZonesRequest) -> Result<zones::ZonesAnswer, AppError> {
    off_thread(move || zones::place(&request, local_zone().as_deref())).await
}

#[tauri::command]
#[specta::specta]
pub async fn time_in_zone(zone: Option<String>) -> Result<String, AppError> {
    off_thread(move || zones::now_in(zone.as_deref(), local_zone().as_deref())).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_cron(request: cron::CronRequest) -> Result<cron::CronAnswer, AppError> {
    off_thread(move || cron::describe(&request, local_zone().as_deref(), chrono::Utc::now())).await
}

#[tauri::command]
#[specta::specta]
pub async fn convert_size(request: sizes::SizesRequest) -> Result<sizes::SizesAnswer, AppError> {
    off_thread(move || sizes::convert(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn answer_percentages(
    request: percentages::PercentagesRequest,
) -> Result<percentages::PercentagesAnswer, AppError> {
    off_thread(move || percentages::answer_all(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_permissions(
    request: permissions::PermissionsRequest,
) -> Result<permissions::PermissionsAnswer, AppError> {
    off_thread(move || permissions::describe(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn check_digits(request: checks::CheckRequest) -> Result<checks::CheckAnswer, AppError> {
    off_thread(move || checks::check(&request)).await
}

/// The token and the secret are zeroed once the answer is made.
#[tauri::command]
#[specta::specta]
pub async fn decode_jwt(request: jwt::JwtRequest) -> Result<jwt::JwtAnswer, AppError> {
    off_thread(move || jwt::decode(request, chrono::Utc::now())).await
}

#[tauri::command]
#[specta::specta]
pub async fn text_stats(request: stats::StatsRequest) -> Result<stats::TextStats, AppError> {
    off_thread(move || stats::count(&request)).await
}
