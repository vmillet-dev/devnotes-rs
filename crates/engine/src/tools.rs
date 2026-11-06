//! The Outils area: each tool a pure function of its module, a request in and an answer out.
//! A tool's own failure — a line that does not parse — is part of its answer; `AppError` is left
//! for the unexpected. No lock and no store: nothing here reads the library. ⚠️ One tool reads
//! files — the identity generator's lists, shipped as a resource — and takes the handle for
//! where they are.

pub use files::FileProblem;

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

/// Off the window's thread even for what answers in a microsecond: a large paste does not.
pub(crate) async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| StorageError::Unavailable.into())
}

/// The identity tool reads its lists from the application's resources, `identity/`.
pub fn generate_identity(
    paths: &Paths,
    request: &identity::IdentityRequest,
) -> identity::IdentityOutcome {
    let lists = paths.resources.join("identity");
    identity::generate(request, chrono::Local::now().date_naive(), &lists)
}

pub fn export_identities(
    paths: &Paths,
    request: &identity::IdentityRequest,
    batch: &identity::PeopleBatch,
    path: &str,
) -> identity::PeopleExport {
    let lists = paths.resources.join("identity");
    let today = chrono::Local::now().date_naive();
    identity::export(request, batch, path, today, &lists)
}

/// Bytes written where the user chose: never a note.
pub fn write_file(path: &str, bytes: &[u8]) -> Result<(), FileProblem> {
    files::write(path, bytes)
}

/// No randomness from the system is the unexpected: nothing weaker is drawn in its place.
pub fn generate_passwords(
    request: &random::PasswordRequest,
) -> Result<random::PasswordAnswer, AppError> {
    random::passwords(request).map_err(|_| StorageError::Unavailable.into())
}

pub fn generate_identifiers(
    request: &identifiers::IdentifiersRequest,
) -> Result<Vec<String>, AppError> {
    identifiers::generate(request).map_err(|_| StorageError::Unavailable.into())
}

/// A date written without an offset is read in the machine's zone, as the person typing it means.
pub fn describe_instant(request: &dates::InstantRequest) -> dates::InstantAnswer {
    dates::describe(request, &chrono::Local)
}

/// The two dates are read in the machine's zone, which the answer names.
pub fn measure_durations(request: &durations::DurationsRequest) -> durations::DurationsAnswer {
    durations::measure(request, durations::zone_named(local_zone().as_deref()))
}

/// The system's zone by its IANA name, when it gives one: the database itself is compiled in.
fn local_zone() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

pub fn search_time_zones(query: &str) -> Vec<zones::ZoneEntry> {
    zones::search(query, chrono::Utc::now())
}

pub fn place_in_zones(request: &zones::ZonesRequest) -> zones::ZonesAnswer {
    zones::place(request, local_zone().as_deref())
}

pub fn time_in_zone(zone: Option<&str>) -> String {
    zones::now_in(zone, local_zone().as_deref())
}

pub fn describe_cron(request: &cron::CronRequest) -> cron::CronAnswer {
    cron::describe(request, local_zone().as_deref(), chrono::Utc::now())
}

/// The token and the secret are zeroed once the answer is made.
pub fn decode_jwt(request: jwt::JwtRequest) -> jwt::JwtAnswer {
    jwt::decode(request, chrono::Utc::now())
}
