//! The seam between a format and the visualiser: one reader per format, one tree out. A new
//! format is a reader that answers a `Node` whose spans are UTF-16 offsets into its own text.

use serde::{Deserialize, Serialize};

use super::parse::{JsonError, Node, parse};
use super::{toml, xml, yaml};
use crate::notes::language::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StructureFormat {
    Json,
    Yaml,
    Toml,
    Xml,
}

/// What the tree leaves out of the text, said rather than hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
        StructureFormat::Toml => toml::read(text),
        StructureFormat::Xml => xml::read(text),
    }
}

/// The snippets whose language has a reader; HTML is not XML, and gets none.
pub fn explorable_format(language: Language) -> Option<StructureFormat> {
    match language {
        Language::Json => Some(StructureFormat::Json),
        Language::Yml => Some(StructureFormat::Yaml),
        Language::Toml => Some(StructureFormat::Toml),
        Language::Xml => Some(StructureFormat::Xml),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_languages_with_a_reader_are_explored() {
        assert_eq!(
            explorable_format(Language::Yml),
            Some(StructureFormat::Yaml)
        );
        assert_eq!(explorable_format(Language::Xml), Some(StructureFormat::Xml));
        assert_eq!(explorable_format(Language::Html), None);
        assert_eq!(explorable_format(Language::Jsonld), None);
    }
}
