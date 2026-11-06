//! The tools' calls: pure functions, each on the `compute` queue.

use engine::tools::bases::{
    self, Base64FileAnswer, DecodeAnswer, DecodeRequest, EncodeRequest, Encodings, SavedBytes,
};
use engine::tools::diff::{self, JsonDiffAnswer, JsonDiffRequest};
use engine::tools::encoding::{self, UrlCodecAnswer, UrlCodecRequest};
use engine::tools::generate::{self, LoremAnswer, LoremRequest};
use engine::tools::hash::{self, HashAnswer, HashRequest};
use engine::tools::jwt::{JwtAnswer, JwtRequest};
use engine::tools::random::{PasswordAnswer, PasswordRequest};
use engine::tools::stats::{self, StatsRequest, TextStats};
use engine::tools::text::{self, CaseAnswer, CaseRequest, LineBreaksAnswer, LineBreaksRequest};

use engine::tools::text_diff::{self, TextDiffAnswer, TextDiffRequest};

use engine::tools::url_parts::{self, UrlAnswer};

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
