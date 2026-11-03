//! A `preferences.json`, as the reference wrote it and as the core reads `automaticBackups` from
//! it: one flat object of `devnotes.*` keys, each a string. Two of them (plan § 4.2): the
//! application's, in the profile, and the open library's, in its directory, for
//! `devnotes.notes.*`.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde_json::Value;

/// What a library keeps for itself; every other key is the application's.
pub const LIBRARY_PREFIX: &str = "devnotes.notes.";

/// Every key the file held, those this version does not know included: they are written back
/// as they were.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Preferences(BTreeMap<String, Value>);

impl Preferences {
    /// An empty set for a file that is not there or does not read: the defaults are a fine
    /// answer to either.
    pub fn read(file: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(file) else {
            return Self::default();
        };
        serde_json::from_str(&text)
            .map(Self)
            .inspect_err(|error| log::warn!("A preferences file is unreadable: {error}"))
            .unwrap_or_default()
    }

    /// Staged then renamed: a crash halfway leaves the previous file, never half of one.
    pub fn write(&self, file: &Path) -> io::Result<()> {
        if let Some(directory) = file.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let staged = file.with_extension("json.part");
        std::fs::write(&staged, serde_json::to_vec_pretty(&self.0)?)?;
        std::fs::rename(&staged, file)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.0
            .insert(key.to_string(), Value::String(value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_this_version_does_not_know_survives_a_write() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("preferences.json");
        std::fs::write(
            &file,
            r#"{ "devnotes.theme": "dark", "devnotes.later": { "kept": [1, 2] } }"#,
        )
        .unwrap();

        let mut preferences = Preferences::read(&file);
        preferences.set("devnotes.theme", "light");
        preferences.write(&file).unwrap();

        let again: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(again["devnotes.theme"], "light");
        assert_eq!(again["devnotes.later"]["kept"][1], 2);
        assert!(!file.with_extension("json.part").exists());
    }

    #[test]
    fn a_missing_or_damaged_file_reads_as_no_preference() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("preferences.json");
        assert_eq!(Preferences::read(&file), Preferences::default());

        std::fs::write(&file, "{ half").unwrap();
        assert_eq!(Preferences::read(&file), Preferences::default());
    }
}
