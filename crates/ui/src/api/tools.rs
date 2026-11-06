//! The tools' calls: pure functions, each on the `compute` queue.

use engine::tools::diff::{self, JsonDiffAnswer, JsonDiffRequest};
use engine::tools::generate::{self, LoremAnswer, LoremRequest};
use engine::tools::stats::{self, StatsRequest, TextStats};
use engine::tools::text::{self, CaseAnswer, CaseRequest, LineBreaksAnswer, LineBreaksRequest};

use engine::tools::text_diff::{self, TextDiffAnswer, TextDiffRequest};

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
