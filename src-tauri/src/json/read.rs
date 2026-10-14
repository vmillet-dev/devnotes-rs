//! The seam between a format and the visualiser: one reader per format, one tree out. A new
//! format is a reader that answers a `Node` whose spans are UTF-16 offsets into its own text.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::parse::{JsonError, Node, parse};
use super::yaml;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StructureFormat {
    Json,
    Yaml,
}

/// What the tree leaves out of the text, said rather than hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonNotice {
    /// A stream of several YAML documents: the first one is explored.
    FirstDocumentOnly,
}

#[derive(Debug)]
pub(crate) struct Read {
    pub(crate) root: Node,
    pub(crate) notices: Vec<JsonNotice>,
}

pub(crate) fn read(text: &str, format: StructureFormat) -> Result<Read, JsonError> {
    match format {
        StructureFormat::Json => parse(text).map(|root| Read {
            root,
            notices: Vec::new(),
        }),
        StructureFormat::Yaml => yaml::read(text),
    }
}
