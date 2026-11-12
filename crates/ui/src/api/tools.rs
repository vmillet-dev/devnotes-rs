//! The tools' calls: pure functions, each on the `compute` queue.

use engine::tools;
use engine::tools::bases::{
    self, Base64FileAnswer, DecodeAnswer, DecodeRequest, EncodeRequest, Encodings, SavedBytes,
};
use engine::tools::checks::{self, CheckAnswer, CheckRequest};
use engine::tools::cidr::{self, CidrAnswer, CidrRequest};
use engine::tools::colour::{self, ColourAnswer, ColourRequest};
use engine::tools::convert::{self, ConvertAnswer, ConvertRequest};
use engine::tools::cron::{CronAnswer, CronRequest};
use engine::tools::dates::{self, InstantAnswer, InstantRequest};
use engine::tools::diff::{self, JsonDiffAnswer, JsonDiffRequest};
use engine::tools::durations::{DurationsAnswer, DurationsRequest};
use engine::tools::encoding::{self, UrlCodecAnswer, UrlCodecRequest};
use engine::tools::generate::{self, GenerateAnswer, GenerateRequest, LoremAnswer, LoremRequest};
use engine::tools::hash::{self, HashAnswer, HashRequest};
use engine::tools::identifiers::{self, IdInspection, IdentifiersRequest};
use engine::tools::identity::{IdentityOutcome, IdentityRequest, PeopleBatch, PeopleExport};
use engine::tools::jwt::{JwtAnswer, JwtRequest};
use engine::tools::permissions::{self, PermissionsAnswer, PermissionsRequest};
use engine::tools::qr::{self, QrAnswer, QrFormat, QrRequest, SavedCode};
use engine::tools::random::{PasswordAnswer, PasswordRequest};
use engine::tools::sizes::{self, SizesAnswer, SizesRequest};
use engine::tools::stats::{self, StatsRequest, TextStats};
use engine::tools::text::{self, CaseAnswer, CaseRequest, LineBreaksAnswer, LineBreaksRequest};

use engine::tools::text_diff::{self, TextDiffAnswer, TextDiffRequest};

use engine::tools::transfer::{self, TransferAnswer, TransferRequest};
use engine::tools::url_parts::{self, UrlAnswer};
use engine::tools::zones::{ZoneEntry, ZonesAnswer, ZonesRequest};

use crate::backend::{Backend, Pending};

/// How the home's search reads a text: the core's fold, called directly (D12).
pub fn fold(text: &str) -> String {
    engine::notes::view::fold(text)
}

pub fn convert_case(backend: &Backend, request: CaseRequest) -> Pending<CaseAnswer> {
    backend.compute(move |_| Ok(text::convert_case(&request)))
}

pub fn fix_line_breaks(backend: &Backend, request: LineBreaksRequest) -> Pending<LineBreaksAnswer> {
    backend.compute(move |_| Ok(text::fix_line_breaks(&request)))
}

pub fn text_stats(backend: &Backend, request: StatsRequest) -> Pending<TextStats> {
    backend.compute(move |_| Ok(stats::count(&request)))
}

pub fn lorem(backend: &Backend, request: LoremRequest) -> Pending<LoremAnswer> {
    backend.compute(move |_| Ok(generate::lorem(&request)))
}

pub fn diff_text(backend: &Backend, request: TextDiffRequest) -> Pending<TextDiffAnswer> {
    backend.compute(move |_| Ok(text_diff::diff(&request)))
}

pub fn diff_json(backend: &Backend, request: JsonDiffRequest) -> Pending<JsonDiffAnswer> {
    backend.compute(move |_| Ok(diff::diff(&request)))
}

pub fn url_codec(backend: &Backend, request: UrlCodecRequest) -> Pending<UrlCodecAnswer> {
    backend.compute(move |_| Ok(encoding::url_codec(&request)))
}

pub fn parse_url(backend: &Backend, text: String) -> Pending<UrlAnswer> {
    backend.compute(move |_| Ok(url_parts::parse_url(&text)))
}

pub fn hash(backend: &Backend, request: HashRequest) -> Pending<HashAnswer> {
    backend.compute(move |_| Ok(hash::hash(request)))
}

pub fn generate_passwords(backend: &Backend, request: PasswordRequest) -> Pending<PasswordAnswer> {
    backend.compute(move |_| engine::tools::generate_passwords(&request))
}

pub fn decode_jwt(backend: &Backend, request: JwtRequest) -> Pending<JwtAnswer> {
    backend.compute(move |_| Ok(engine::tools::decode_jwt(request)))
}

pub fn encode_bytes(backend: &Backend, request: EncodeRequest) -> Pending<Encodings> {
    backend.compute(move |_| Ok(bases::encode(&request)))
}

pub fn encode_base64_file(backend: &Backend, path: String) -> Pending<Base64FileAnswer> {
    backend.compute(move |_| Ok(bases::encode_file(&path)))
}

pub fn decode_bytes(backend: &Backend, request: DecodeRequest) -> Pending<DecodeAnswer> {
    backend.compute(move |_| Ok(bases::decode(&request)))
}

pub fn save_bytes(backend: &Backend, request: DecodeRequest, path: String) -> Pending<SavedBytes> {
    backend.compute(move |_| Ok(bases::save(&request, &path)))
}

pub fn describe_instant(backend: &Backend, request: InstantRequest) -> Pending<InstantAnswer> {
    backend.compute(move |_| Ok(engine::tools::describe_instant(&request)))
}

pub fn measure_durations(backend: &Backend, request: DurationsRequest) -> Pending<DurationsAnswer> {
    backend.compute(move |_| Ok(engine::tools::measure_durations(&request)))
}

pub fn current_instant(backend: &Backend) -> Pending<String> {
    backend.compute(|_| Ok(dates::now()))
}

pub fn search_time_zones(backend: &Backend, query: String) -> Pending<Vec<ZoneEntry>> {
    backend.compute(move |_| Ok(engine::tools::search_time_zones(&query)))
}

pub fn place_in_zones(backend: &Backend, request: ZonesRequest) -> Pending<ZonesAnswer> {
    backend.compute(move |_| Ok(engine::tools::place_in_zones(&request)))
}

pub fn time_in_zone(backend: &Backend, zone: Option<String>) -> Pending<String> {
    backend.compute(move |_| Ok(engine::tools::time_in_zone(zone.as_deref())))
}

pub fn describe_cron(backend: &Backend, request: CronRequest) -> Pending<CronAnswer> {
    backend.compute(move |_| Ok(engine::tools::describe_cron(&request)))
}

pub fn convert_size(backend: &Backend, request: SizesRequest) -> Pending<SizesAnswer> {
    backend.compute(move |_| Ok(sizes::convert(&request)))
}

pub fn estimate_transfer(backend: &Backend, request: TransferRequest) -> Pending<TransferAnswer> {
    backend.compute(move |_| Ok(transfer::estimate(&request)))
}

pub fn describe_permissions(
    backend: &Backend,
    request: PermissionsRequest,
) -> Pending<PermissionsAnswer> {
    backend.compute(move |_| Ok(permissions::describe(&request)))
}

pub fn check_digits(backend: &Backend, request: CheckRequest) -> Pending<CheckAnswer> {
    backend.compute(move |_| Ok(checks::check(&request)))
}

pub fn describe_cidr(backend: &Backend, request: CidrRequest) -> Pending<CidrAnswer> {
    backend.compute(move |_| Ok(cidr::describe(&request)))
}

pub fn convert_data(backend: &Backend, request: ConvertRequest) -> Pending<ConvertAnswer> {
    backend.compute(move |_| Ok(convert::convert(&request)))
}

pub fn generate_json(backend: &Backend, request: GenerateRequest) -> Pending<GenerateAnswer> {
    backend.compute(move |_| Ok(generate::generate(&request)))
}

pub fn generate_identifiers(
    backend: &Backend,
    request: IdentifiersRequest,
) -> Pending<Vec<String>> {
    backend.compute(move |_| tools::generate_identifiers(&request))
}

pub fn inspect_identifier(backend: &Backend, text: String) -> Pending<IdInspection> {
    backend.compute(move |_| Ok(identifiers::inspect(&text)))
}

pub fn describe_colour(backend: &Backend, request: ColourRequest) -> Pending<ColourAnswer> {
    backend.compute(move |_| Ok(colour::describe(&request)))
}

/// What the core answers of a code, with its pixels for the screen.
pub type QrDrawing = (QrAnswer, Option<(u32, Vec<u8>)>);

pub fn draw_qr_code(backend: &Backend, request: QrRequest) -> Pending<QrDrawing> {
    backend.compute(move |_| Ok((qr::describe(&request), qr::rgba(&request))))
}

pub fn save_qr_code(
    backend: &Backend,
    request: QrRequest,
    format: QrFormat,
    path: String,
) -> Pending<SavedCode> {
    backend.compute(move |_| Ok(qr::save(&request, format, &path)))
}

pub fn generate_identity(backend: &Backend, request: IdentityRequest) -> Pending<IdentityOutcome> {
    backend.compute(move |engine| Ok(tools::generate_identity(engine.paths(), &request)))
}

pub fn export_identities(
    backend: &Backend,
    request: IdentityRequest,
    batch: PeopleBatch,
    path: String,
) -> Pending<PeopleExport> {
    backend.compute(move |engine| {
        Ok(tools::export_identities(
            engine.paths(),
            &request,
            &batch,
            &path,
        ))
    })
}
