//! What a space and a folder share about their names.

use crate::error::ValidationError;

/// Trimmed and never blank. ⚠️ Trimming is not cosmetic: [`is_taken`] folds case but not
/// spaces, so "Perf" and " Perf " would coexist, identical on screen.
pub(crate) fn readable(raw: &str, what: &str) -> Result<String, ValidationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::new(
            "name",
            format!("a {what} must have a readable name"),
        ));
    }

    Ok(trimmed.to_string())
}

/// Whether `name` is already held, case aside, by anything but `except_id` — which is what
/// lets a rename correct the case of a name without being refused as its own duplicate.
///
/// ⚠️ Decided here rather than by a unique index: the names are sealed, and an index on
/// ciphertext catches nothing.
pub(crate) fn is_taken<'a>(
    held: impl IntoIterator<Item = (&'a str, &'a str)>,
    name: &str,
    except_id: Option<&str>,
) -> bool {
    let wanted = name.to_lowercase();

    held.into_iter()
        .any(|(id, existing)| Some(id) != except_id && existing.to_lowercase() == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_before_being_stored() {
        assert_eq!(readable("  Perf  ", "folder").unwrap(), "Perf");
    }

    #[test]
    fn a_blank_name_is_refused_with_its_field() {
        for blank in ["", "   ", "\t\n"] {
            assert_eq!(readable(blank, "space").unwrap_err().field, "name");
        }
    }

    #[test]
    fn a_name_is_taken_whatever_its_case() {
        assert!(is_taken([("s-1", "Personal")], "PERSONAL", None));
        assert!(!is_taken([("s-1", "Personal")], "Work", None));
    }

    #[test]
    fn a_name_does_not_collide_with_itself() {
        assert!(!is_taken([("s-1", "personal")], "Personal", Some("s-1")));
        assert!(is_taken([("s-1", "personal")], "Personal", Some("s-2")));
    }
}
