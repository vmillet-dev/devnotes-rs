//! The Outils area: each tool a pure function of its module, a request in and an answer out.
//! A tool's own failure — a line that does not parse — is part of its answer; `AppError` is left
//! for the unexpected. No lock and no store: nothing here reads the library. ⚠️ One tool reads
//! files — the identity generator's lists, shipped as a resource — and takes the handle for
//! where they are.

pub mod bases;
pub mod checks;
pub mod cidr;
pub mod colour;
pub mod convert;
pub mod cron;
pub mod dates;
pub mod diff;
pub mod durations;
pub mod encoding;
pub(crate) mod files;
pub mod generate;
pub mod hash;
pub mod identifiers;
pub mod identity;
pub mod image;
pub mod jwt;
pub(crate) mod numbers;
pub mod permissions;
pub mod qr;
pub mod random;
pub mod sizes;
pub mod stats;
pub mod text;
pub mod text_diff;
pub mod transfer;
pub mod url_parts;
pub mod zones;

use crate::Paths;
use crate::error::{AppError, StorageError};

/// Off the window's thread even for a tool that answers in a microsecond: a large paste does not.
pub(crate) async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| StorageError::Unavailable.into())
}

pub async fn convert_case(request: text::CaseRequest) -> Result<text::CaseAnswer, AppError> {
    off_thread(move || text::convert_case(&request)).await
}

pub async fn fix_line_breaks(
    request: text::LineBreaksRequest,
) -> Result<text::LineBreaksAnswer, AppError> {
    off_thread(move || text::fix_line_breaks(&request)).await
}

pub async fn parse_url(text: String) -> Result<url_parts::UrlAnswer, AppError> {
    off_thread(move || url_parts::parse_url(&text)).await
}

pub async fn url_codec(
    request: encoding::UrlCodecRequest,
) -> Result<encoding::UrlCodecAnswer, AppError> {
    off_thread(move || encoding::url_codec(&request)).await
}

pub async fn encode_bytes(request: bases::EncodeRequest) -> Result<bases::Encodings, AppError> {
    off_thread(move || bases::encode(&request)).await
}

/// By its path: the bytes are read here and never cross the bridge.
pub async fn encode_base64_file(path: String) -> Result<bases::Base64FileAnswer, AppError> {
    off_thread(move || bases::encode_file(&path)).await
}

/// The identity tool reads its lists from the application's resources, `identity/`.
pub async fn generate_identity(
    paths: &Paths,
    request: identity::IdentityRequest,
) -> Result<identity::IdentityOutcome, AppError> {
    let lists = paths.resources.join("identity");
    off_thread(move || identity::generate(&request, chrono::Local::now().date_naive(), &lists))
        .await
}

pub async fn export_identities(
    paths: &Paths,
    request: identity::IdentityRequest,
    batch: identity::PeopleBatch,
    path: String,
) -> Result<identity::PeopleExport, AppError> {
    let lists = paths.resources.join("identity");
    off_thread(move || {
        identity::export(
            &request,
            &batch,
            &path,
            chrono::Local::now().date_naive(),
            &lists,
        )
    })
    .await
}

pub async fn decode_bytes(request: bases::DecodeRequest) -> Result<bases::DecodeAnswer, AppError> {
    off_thread(move || bases::decode(&request)).await
}

pub async fn save_bytes(
    request: bases::DecodeRequest,
    path: String,
) -> Result<bases::SavedBytes, AppError> {
    off_thread(move || bases::save(&request, &path)).await
}

pub async fn hash_input(request: hash::HashRequest) -> Result<hash::HashAnswer, AppError> {
    off_thread(move || hash::hash(request)).await
}

/// No randomness from the system is the unexpected: nothing weaker is drawn in its place.
pub async fn generate_passwords(
    request: random::PasswordRequest,
) -> Result<random::PasswordAnswer, AppError> {
    off_thread(move || random::passwords(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

pub async fn generate_identifiers(
    request: identifiers::IdentifiersRequest,
) -> Result<Vec<String>, AppError> {
    off_thread(move || identifiers::generate(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

pub async fn inspect_identifier(text: String) -> Result<identifiers::IdInspection, AppError> {
    off_thread(move || identifiers::inspect(&text)).await
}

pub async fn describe_colour(
    request: colour::ColourRequest,
) -> Result<colour::ColourAnswer, AppError> {
    off_thread(move || colour::describe(&request)).await
}

pub async fn convert_data(
    request: convert::ConvertRequest,
) -> Result<convert::ConvertAnswer, AppError> {
    off_thread(move || convert::convert(&request)).await
}

pub async fn generate_json(
    request: generate::GenerateRequest,
) -> Result<generate::GenerateAnswer, AppError> {
    off_thread(move || generate::generate(&request)).await
}

pub async fn lorem_ipsum(
    request: generate::LoremRequest,
) -> Result<generate::LoremAnswer, AppError> {
    off_thread(move || generate::lorem(&request)).await
}

pub async fn diff_text(
    request: text_diff::TextDiffRequest,
) -> Result<text_diff::TextDiffAnswer, AppError> {
    off_thread(move || text_diff::diff(&request)).await
}

pub async fn diff_json(request: diff::JsonDiffRequest) -> Result<diff::JsonDiffAnswer, AppError> {
    off_thread(move || diff::diff(&request)).await
}

/// A date written without an offset is read in the machine's zone, as the person typing it means.
pub async fn describe_instant(
    request: dates::InstantRequest,
) -> Result<dates::InstantAnswer, AppError> {
    off_thread(move || dates::describe(&request, &chrono::Local)).await
}

/// The two dates are read in the machine's zone, which the answer names.
pub async fn measure_durations(
    request: durations::DurationsRequest,
) -> Result<durations::DurationsAnswer, AppError> {
    off_thread(move || durations::measure(&request, durations::zone_named(local_zone().as_deref())))
        .await
}

pub async fn current_instant() -> Result<String, AppError> {
    off_thread(dates::now).await
}

/// The system's zone by its IANA name, when it gives one: the database itself is compiled in.
fn local_zone() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

pub async fn search_time_zones(query: String) -> Result<Vec<zones::ZoneEntry>, AppError> {
    off_thread(move || zones::search(&query, chrono::Utc::now())).await
}

pub async fn place_in_zones(request: zones::ZonesRequest) -> Result<zones::ZonesAnswer, AppError> {
    off_thread(move || zones::place(&request, local_zone().as_deref())).await
}

pub async fn time_in_zone(zone: Option<String>) -> Result<String, AppError> {
    off_thread(move || zones::now_in(zone.as_deref(), local_zone().as_deref())).await
}

pub async fn describe_cron(request: cron::CronRequest) -> Result<cron::CronAnswer, AppError> {
    off_thread(move || cron::describe(&request, local_zone().as_deref(), chrono::Utc::now())).await
}

pub async fn estimate_transfer(
    request: transfer::TransferRequest,
) -> Result<transfer::TransferAnswer, AppError> {
    off_thread(move || transfer::estimate(&request)).await
}

pub async fn convert_size(request: sizes::SizesRequest) -> Result<sizes::SizesAnswer, AppError> {
    off_thread(move || sizes::convert(&request)).await
}

pub async fn describe_permissions(
    request: permissions::PermissionsRequest,
) -> Result<permissions::PermissionsAnswer, AppError> {
    off_thread(move || permissions::describe(&request)).await
}

pub async fn check_digits(request: checks::CheckRequest) -> Result<checks::CheckAnswer, AppError> {
    off_thread(move || checks::check(&request)).await
}

/// The token and the secret are zeroed once the answer is made.
pub async fn decode_jwt(request: jwt::JwtRequest) -> Result<jwt::JwtAnswer, AppError> {
    off_thread(move || jwt::decode(request, chrono::Utc::now())).await
}

pub async fn text_stats(request: stats::StatsRequest) -> Result<stats::TextStats, AppError> {
    off_thread(move || stats::count(&request)).await
}

pub async fn describe_qr_code(request: qr::QrRequest) -> Result<qr::QrAnswer, AppError> {
    off_thread(move || qr::describe(&request)).await
}

pub async fn save_qr_code(
    request: qr::QrRequest,
    format: qr::QrFormat,
    path: String,
) -> Result<qr::SavedCode, AppError> {
    off_thread(move || qr::save(&request, format, &path)).await
}

/// The QR code as RGBA pixels, `pixels` a side, for the shell to copy. `None` when there is no code.
pub async fn qr_code_pixels(request: qr::QrRequest) -> Result<Option<(u32, Vec<u8>)>, AppError> {
    off_thread(move || qr::rgba(&request)).await
}

pub async fn describe_cidr(request: cidr::CidrRequest) -> Result<cidr::CidrAnswer, AppError> {
    off_thread(move || cidr::describe(&request)).await
}
