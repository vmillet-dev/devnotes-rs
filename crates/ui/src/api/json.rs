//! The visualiser's model: a text in, what to draw out.

use engine::json::model::{JsonQuery, JsonView};
use engine::json::read::StructureFormat;
use engine::notes::language::Language;

use crate::backend::{Backend, Pending};

/// On `compute`: a document of a few megabytes is not the window's thread's to parse.
pub fn explore(backend: &Backend, query: JsonQuery) -> Pending<JsonView> {
    backend.compute(move |_| Ok(engine::json::model::explore(&query)))
}

/// Which reader the snippet's language has, if any: only those get the Graph and Tree tabs.
pub fn explorable_format(language: Language) -> Option<StructureFormat> {
    engine::json::read::explorable_format(language)
}
