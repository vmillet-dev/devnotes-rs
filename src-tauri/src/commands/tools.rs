#![allow(clippy::needless_pass_by_value)]

use engine::error::AppError;
use engine::error::StorageError;
use engine::tools::bases;
use engine::tools::checks;
use engine::tools::cidr;
use engine::tools::colour;
use engine::tools::convert;
use engine::tools::cron;
use engine::tools::dates;
use engine::tools::diff;
use engine::tools::durations;
use engine::tools::encoding;
use engine::tools::generate;
use engine::tools::hash;
use engine::tools::identifiers;
use engine::tools::identity;
use engine::tools::jwt;
use engine::tools::permissions;
use engine::tools::qr;
use engine::tools::random;
use engine::tools::sizes;
use engine::tools::stats;
use engine::tools::text;
use engine::tools::text_diff;
use engine::tools::transfer;
use engine::tools::url_parts;
use engine::tools::zones;
use engine::{Engine, tools};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn convert_case(request: text::CaseRequest) -> Result<text::CaseAnswer, AppError> {
    tools::convert_case(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn fix_line_breaks(
    request: text::LineBreaksRequest,
) -> Result<text::LineBreaksAnswer, AppError> {
    tools::fix_line_breaks(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn parse_url(text: String) -> Result<url_parts::UrlAnswer, AppError> {
    tools::parse_url(text).await
}

#[tauri::command]
#[specta::specta]
pub async fn url_codec(
    request: encoding::UrlCodecRequest,
) -> Result<encoding::UrlCodecAnswer, AppError> {
    tools::url_codec(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn encode_bytes(request: bases::EncodeRequest) -> Result<bases::Encodings, AppError> {
    tools::encode_bytes(request).await
}

/// By its path: the bytes are read here and never cross the bridge.
#[tauri::command]
#[specta::specta]
pub async fn encode_base64_file(path: String) -> Result<bases::Base64FileAnswer, AppError> {
    tools::encode_base64_file(path).await
}

#[tauri::command]
#[specta::specta]
pub async fn decode_bytes(request: bases::DecodeRequest) -> Result<bases::DecodeAnswer, AppError> {
    tools::decode_bytes(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_bytes(
    request: bases::DecodeRequest,
    path: String,
) -> Result<bases::SavedBytes, AppError> {
    tools::save_bytes(request, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn hash_input(request: hash::HashRequest) -> Result<hash::HashAnswer, AppError> {
    tools::hash_input(request).await
}

/// No randomness from the system is the unexpected: nothing weaker is drawn in its place.
#[tauri::command]
#[specta::specta]
pub async fn generate_passwords(
    request: random::PasswordRequest,
) -> Result<random::PasswordAnswer, AppError> {
    tools::generate_passwords(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn generate_identifiers(
    request: identifiers::IdentifiersRequest,
) -> Result<Vec<String>, AppError> {
    tools::generate_identifiers(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_identifier(text: String) -> Result<identifiers::IdInspection, AppError> {
    tools::inspect_identifier(text).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_colour(
    request: colour::ColourRequest,
) -> Result<colour::ColourAnswer, AppError> {
    tools::describe_colour(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn convert_data(
    request: convert::ConvertRequest,
) -> Result<convert::ConvertAnswer, AppError> {
    tools::convert_data(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn generate_json(
    request: generate::GenerateRequest,
) -> Result<generate::GenerateAnswer, AppError> {
    tools::generate_json(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn lorem_ipsum(
    request: generate::LoremRequest,
) -> Result<generate::LoremAnswer, AppError> {
    tools::lorem_ipsum(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn diff_text(
    request: text_diff::TextDiffRequest,
) -> Result<text_diff::TextDiffAnswer, AppError> {
    tools::diff_text(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn diff_json(request: diff::JsonDiffRequest) -> Result<diff::JsonDiffAnswer, AppError> {
    tools::diff_json(request).await
}

/// A date written without an offset is read in the machine's zone, as the person typing it means.
#[tauri::command]
#[specta::specta]
pub async fn describe_instant(
    request: dates::InstantRequest,
) -> Result<dates::InstantAnswer, AppError> {
    tools::describe_instant(request).await
}

/// The two dates are read in the machine's zone, which the answer names.
#[tauri::command]
#[specta::specta]
pub async fn measure_durations(
    request: durations::DurationsRequest,
) -> Result<durations::DurationsAnswer, AppError> {
    tools::measure_durations(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn current_instant() -> Result<String, AppError> {
    tools::current_instant().await
}

#[tauri::command]
#[specta::specta]
pub async fn search_time_zones(query: String) -> Result<Vec<zones::ZoneEntry>, AppError> {
    tools::search_time_zones(query).await
}

#[tauri::command]
#[specta::specta]
pub async fn place_in_zones(request: zones::ZonesRequest) -> Result<zones::ZonesAnswer, AppError> {
    tools::place_in_zones(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn time_in_zone(zone: Option<String>) -> Result<String, AppError> {
    tools::time_in_zone(zone).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_cron(request: cron::CronRequest) -> Result<cron::CronAnswer, AppError> {
    tools::describe_cron(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn estimate_transfer(
    request: transfer::TransferRequest,
) -> Result<transfer::TransferAnswer, AppError> {
    tools::estimate_transfer(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn convert_size(request: sizes::SizesRequest) -> Result<sizes::SizesAnswer, AppError> {
    tools::convert_size(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_permissions(
    request: permissions::PermissionsRequest,
) -> Result<permissions::PermissionsAnswer, AppError> {
    tools::describe_permissions(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn check_digits(request: checks::CheckRequest) -> Result<checks::CheckAnswer, AppError> {
    tools::check_digits(request).await
}

/// The token and the secret are zeroed once the answer is made.
#[tauri::command]
#[specta::specta]
pub async fn decode_jwt(request: jwt::JwtRequest) -> Result<jwt::JwtAnswer, AppError> {
    tools::decode_jwt(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn text_stats(request: stats::StatsRequest) -> Result<stats::TextStats, AppError> {
    tools::text_stats(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_qr_code(request: qr::QrRequest) -> Result<qr::QrAnswer, AppError> {
    tools::describe_qr_code(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_qr_code(
    request: qr::QrRequest,
    format: qr::QrFormat,
    path: String,
) -> Result<qr::SavedCode, AppError> {
    tools::save_qr_code(request, format, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn describe_cidr(request: cidr::CidrRequest) -> Result<cidr::CidrAnswer, AppError> {
    tools::describe_cidr(request).await
}

#[tauri::command]
#[specta::specta]
pub async fn generate_identity(
    request: identity::IdentityRequest,
    engine: State<'_, Arc<Engine>>,
) -> Result<identity::IdentityOutcome, AppError> {
    tools::generate_identity(engine.paths(), request).await
}

#[tauri::command]
#[specta::specta]
pub async fn export_identities(
    request: identity::IdentityRequest,
    batch: identity::PeopleBatch,
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<identity::PeopleExport, AppError> {
    tools::export_identities(engine.paths(), request, batch, path).await
}

/// The image is written natively: its pixels never cross the bridge. `false` when there is no code.
#[tauri::command]
#[specta::specta]
pub async fn copy_qr_code(request: qr::QrRequest, app: AppHandle) -> Result<bool, AppError> {
    let Some((pixels, rgba)) = tools::qr_code_pixels(request).await? else {
        return Ok(false);
    };
    tauri_plugin_clipboard_manager::ClipboardExt::clipboard(&app)
        .write_image(&tauri::image::Image::new_owned(rgba, pixels, pixels))
        .map_err(|_| StorageError::Unavailable)?;
    Ok(true)
}
