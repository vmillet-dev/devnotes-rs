//! The sets of variables a request resolves its `{{variables}}` from: the globals, shared by
//! every collection, and the environments — Local, Staging, Production — one of which is
//! active. The library's, beside the collections, each set sealed whole.

pub mod store;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use crate::error::ValidationError;
use crate::folders::model::FolderColour;

closed_enum! {
    /// A number is checked to be one; a secret is masked wherever it shows.
    pub enum VariableKind {
        #[default]
        Text = "text",
        Number = "number",
        Secret = "secret",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Variable {
    pub enabled: bool,
    pub name: String,
    pub value: String,
    pub kind: VariableKind,
    /// The request whose last send wrote it — by an extraction or a script —: « remplie par Login ».
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub filled_by: Option<FilledBy>,
}

/// A request that writes a variable at each send; no id for a draft never saved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct FilledBy {
    pub request_id: Option<String>,
    pub name: String,
}

impl Default for Variable {
    fn default() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            value: String::new(),
            kind: VariableKind::Text,
            filled_by: None,
        }
    }
}

/// What a set's table marks on a row: it is kept as typed, and left out when resolving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum VariableFault {
    /// Not a name a `{{…}}` can hold.
    Name,
    /// A number that does not read as one.
    Number,
    /// A name already set above it in the same table: the first one is the one used.
    Duplicate,
}

/// The sealed half of an environment: everything but its id and its rank.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EnvironmentDocument {
    pub name: String,
    pub colour: FolderColour,
    pub protected: bool,
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    /// One of the folders' five: a mark to tell Production from Staging at a glance.
    pub colour: FolderColour,
    /// Asks before sending anything that could change something.
    pub protected: bool,
    pub variables: Vec<Variable>,
    pub position: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpEnvironments {
    pub globals: Vec<Variable>,
    pub environments: Vec<Environment>,
}

/// A field left out is untouched.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentPatch {
    #[specta(optional)]
    pub name: Option<String>,
    #[specta(optional)]
    pub colour: Option<FolderColour>,
    #[specta(optional)]
    pub protected: Option<bool>,
    #[specta(optional)]
    pub variables: Option<Vec<Variable>>,
}

impl EnvironmentDocument {
    #[must_use]
    pub fn patched(mut self, patch: &EnvironmentPatch, name: Option<&str>) -> Self {
        if let Some(name) = name {
            self.name = name.to_string();
        }
        if let Some(colour) = patch.colour {
            self.colour = colour;
        }
        if let Some(protected) = patch.protected {
            self.protected = protected;
        }
        if let Some(variables) = &patch.variables {
            self.variables.clone_from(variables);
        }
        self
    }
}

pub fn validated_name(raw: &str) -> Result<String, ValidationError> {
    crate::name::readable(raw, "environment")
}

/// The letters a `{{name}}` holds, the same as a snippet's field: what the URL lights up.
#[must_use]
pub fn is_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn reads_as_number(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || value.parse::<f64>().is_ok_and(f64::is_finite)
}

/// Each row's fault, by its rank. A row switched off, or still without a name, is not judged.
#[must_use]
pub fn variable_faults(variables: &[Variable]) -> Vec<Option<VariableFault>> {
    let mut seen: Vec<&str> = Vec::new();
    variables
        .iter()
        .map(|variable| {
            let name = variable.name.trim();
            if !variable.enabled || name.is_empty() {
                return None;
            }
            if !is_variable_name(name) {
                return Some(VariableFault::Name);
            }
            if seen.contains(&name) {
                return Some(VariableFault::Duplicate);
            }
            seen.push(name);
            (variable.kind == VariableKind::Number && !reads_as_number(&variable.value))
                .then_some(VariableFault::Number)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str, value: &str, kind: VariableKind) -> Variable {
        Variable {
            name: name.to_string(),
            value: value.to_string(),
            kind,
            ..Variable::default()
        }
    }

    #[test]
    fn every_kind_survives_its_stored_spelling() {
        for kind in VariableKind::ALL {
            assert_eq!(kind.as_str().parse::<VariableKind>(), Ok(kind));
        }
    }

    #[test]
    fn a_name_holds_what_a_placeholder_holds() {
        for name in ["baseUrl", "access_token", "api-key", "v2"] {
            assert!(is_variable_name(name), "{name}");
        }
        for name in ["", "base url", "user.name", "clé", "{{x}}"] {
            assert!(!is_variable_name(name), "{name}");
        }
    }

    #[test]
    fn a_table_marks_a_bad_name_a_second_one_and_a_number_that_is_not() {
        let mut off = variable("token", "x", VariableKind::Text);
        off.enabled = false;
        assert_eq!(
            variable_faults(&[
                variable("baseUrl", "https://exemple.fr", VariableKind::Text),
                variable("base url", "", VariableKind::Text),
                variable("baseUrl", "other", VariableKind::Text),
                variable("pageSize", "50", VariableKind::Number),
                variable("ratio", "-1.5e3", VariableKind::Number),
                variable("limit", "fifty", VariableKind::Number),
                variable("unset", "  ", VariableKind::Number),
                variable("  ", "orphan", VariableKind::Text),
                off,
            ]),
            vec![
                None,
                Some(VariableFault::Name),
                Some(VariableFault::Duplicate),
                None,
                None,
                Some(VariableFault::Number),
                None,
                None,
                None,
            ]
        );
    }

    #[test]
    fn a_patch_changes_what_it_names_and_nothing_else() {
        let before = EnvironmentDocument {
            name: "Staging".to_string(),
            colour: FolderColour::Amber,
            protected: false,
            variables: vec![variable("a", "1", VariableKind::Text)],
        };
        let after = before.clone().patched(
            &EnvironmentPatch {
                protected: Some(true),
                ..EnvironmentPatch::default()
            },
            None,
        );
        assert_eq!(
            after,
            EnvironmentDocument {
                protected: true,
                ..before
            }
        );
    }

    #[test]
    fn a_document_written_before_a_field_existed_still_opens() {
        let document: EnvironmentDocument = serde_json::from_str(r#"{"name":"Local"}"#).unwrap();
        assert_eq!(document.name, "Local");
        assert!(!document.protected);
        assert_eq!(document.colour, FolderColour::default());
        let row: Variable = serde_json::from_str(r#"{"name":"a"}"#).unwrap();
        assert!(row.enabled);
        assert_eq!(row.kind, VariableKind::Text);
    }
}
