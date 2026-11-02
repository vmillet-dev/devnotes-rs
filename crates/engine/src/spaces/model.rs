//! No "All spaces" entry on the data side: it is a display mode, and creating one
//! would have notes filed into it.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::ValidationError;
use crate::name;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Space {
    pub id: String,
    /// Uniqueness is case-insensitive, decided by persistence.
    pub name: String,
    /// Hoisted to the head of the list, the way a pinned note is on the canvas.
    #[serde(default)]
    pub pinned: bool,
}

/// What deleting a space takes or moves: its notes, the trashed ones included, go to the refuge
/// and leave their folder; its folders go with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpaceContents {
    /// The live notes, which the confirmation names; the trashed ones move along silently.
    pub notes: u32,
    pub folders: u32,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpaceDraft {
    pub name: String,
}

impl SpaceDraft {
    pub fn validated_name(&self) -> Result<String, ValidationError> {
        name::readable(&self.name, "space")
    }
}

/// A space cannot be its own refuge: the `ON DELETE CASCADE` would sweep away the
/// notes right after the transfer, and persistence cannot tell — the space exists.
pub fn validate_move_target(id: &str, target_id: &str) -> Result<(), ValidationError> {
    if id == target_id {
        return Err(ValidationError::new(
            "targetSpaceId",
            "notes must be moved to another space",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str) -> SpaceDraft {
        SpaceDraft {
            name: name.to_string(),
        }
    }

    #[test]
    fn a_name_is_trimmed_before_being_stored() {
        assert_eq!(draft("  Personal  ").validated_name().unwrap(), "Personal");
    }

    #[test]
    fn a_space_cannot_be_its_own_move_target() {
        let error = validate_move_target("s-1", "s-1").unwrap_err();

        assert_eq!(error.field, "targetSpaceId");
    }

    #[test]
    fn another_space_is_an_acceptable_move_target() {
        assert!(validate_move_target("s-1", "s-2").is_ok());
    }
}
