//! The passphrase an export can be protected with.
//!
//! A key of the export's own, never the library's: the file is meant to reach another machine,
//! and sending it must not hand over the library it came from. An unprotected export is still
//! written, in plaintext, for portability; the interface says which one it is producing.

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::vault::key::{Cost, SALT_BYTES, Vault, fresh_salt};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Bumped when a protected file written today would stop being readable.
const FORMAT_VERSION: u32 = 1;

/// What a reader needs to derive the same key, in the clear beside the payload: a salt is not
/// secret, and the cost has to be read before anything is derived.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub version: u32,
    pub algorithm: String,
    pub memory_kib: u32,
    pub passes: u32,
    pub lanes: u32,
    pub salt: String,
}

/// A fresh key for one export, and the recipe that reproduces it.
pub fn seal_with(passphrase: &str) -> Result<(Vault, Recipe), StorageError> {
    let cost = Cost::default();
    let salt = fresh_salt()?;
    let vault = Vault::derive(passphrase, &salt, cost)?;

    Ok((
        vault,
        Recipe {
            version: FORMAT_VERSION,
            algorithm: "argon2id".to_string(),
            memory_kib: cost.memory_kib,
            passes: cost.passes,
            lanes: cost.lanes,
            salt: BASE64.encode(salt),
        },
    ))
}

/// Answers [`StorageError::WrongPassphrase`] on a phrase that does not open the payload, which
/// a corrupt file looks the same as. The payload's tag is the only check.
pub fn open_with(passphrase: &str, recipe: &Recipe) -> Result<Vault, StorageError> {
    if recipe.version > FORMAT_VERSION {
        return Err(StorageError::ImportFormat(format!(
            "protected with format version {}, this version of DevNotes reads up to {FORMAT_VERSION}",
            recipe.version
        )));
    }
    if recipe.algorithm != "argon2id" {
        return Err(StorageError::ImportFormat(format!(
            "protected with \"{}\", which this version of DevNotes cannot reproduce",
            recipe.algorithm
        )));
    }

    let salt = BASE64
        .decode(&recipe.salt)
        .map_err(|_| StorageError::ImportFormat("the salt is not base64".to_string()))?;
    if salt.len() != SALT_BYTES {
        return Err(StorageError::ImportFormat(
            "the salt is the wrong size".to_string(),
        ));
    }

    Vault::derive(
        passphrase,
        &salt,
        Cost {
            memory_kib: recipe.memory_kib,
            passes: recipe.passes,
            lanes: recipe.lanes,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_sealed_for_an_export_opens_with_the_same_phrase() {
        let (vault, recipe) = seal_with("a shared phrase").unwrap();
        let sealed = vault.seal_bytes(b"the bundle").unwrap();

        let reader = open_with("a shared phrase", &recipe).unwrap();

        assert_eq!(reader.open_bytes(&sealed).unwrap(), b"the bundle");
    }

    #[test]
    fn another_phrase_will_not_open_it() {
        let (vault, recipe) = seal_with("a shared phrase").unwrap();
        let sealed = vault.seal_bytes(b"the bundle").unwrap();

        let reader = open_with("the wrong phrase", &recipe).unwrap();

        assert!(reader.open_bytes(&sealed).is_err());
    }

    /// Two exports under the same phrase must not share a key: one opened file would open
    /// every other.
    #[test]
    fn two_exports_under_one_phrase_do_not_share_a_key() {
        let (first, first_recipe) = seal_with("a shared phrase").unwrap();
        let (_, second_recipe) = seal_with("a shared phrase").unwrap();

        assert_ne!(first_recipe.salt, second_recipe.salt);

        let sealed = first.seal_bytes(b"the bundle").unwrap();
        let other = open_with("a shared phrase", &second_recipe).unwrap();
        assert!(other.open_bytes(&sealed).is_err());
    }

    #[test]
    fn a_recipe_from_a_newer_version_says_so() {
        let (_, mut recipe) = seal_with("a shared phrase").unwrap();
        recipe.version = FORMAT_VERSION + 1;

        let error = open_with("a shared phrase", &recipe).unwrap_err();

        assert!(format!("{error}").contains("reads up to"));
    }

    #[test]
    fn an_unknown_derivation_is_named_rather_than_guessed_at() {
        let (_, mut recipe) = seal_with("a shared phrase").unwrap();
        recipe.algorithm = "scrypt".to_string();

        let error = open_with("a shared phrase", &recipe).unwrap_err();

        assert!(format!("{error}").contains("scrypt"));
    }

    /// The recipe is what travels in the clear, so it must carry nothing that matters.
    #[test]
    fn the_recipe_holds_no_phrase_and_no_key() {
        let (_, recipe) = seal_with("correct horse battery staple").unwrap();

        let written = serde_json::to_string(&recipe).unwrap();

        assert!(!written.contains("correct horse"));
        assert!(written.contains("argon2id"));
    }
}
