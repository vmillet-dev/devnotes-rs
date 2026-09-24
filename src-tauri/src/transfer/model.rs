//! The format reuses the domain types rather than duplicating them: a field added to
//! `Note` is exported without anyone thinking about it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Write};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::attachments::model::Attachment;
use crate::error::{StorageError, ValidationError};
use crate::folders::model::Folder;
use crate::notes::checklist::{self, NoteKind};
use crate::notes::language::Language;
use crate::notes::model::Note;
use crate::spaces::model::Space;

/// Bumped when a file written today would stop being readable.
///
/// ⚠️ An added enum variant does not bump this: the file still parses, and
/// [`read_bundle`] brings the unknown value down to the default. Bumping would refuse a
/// 500-note file over one note's unknown `language`.
pub const FORMAT_VERSION: u32 = 1;

/// What an export takes.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExportScope {
    Library,
    Space {
        #[serde(rename = "spaceId")]
        space_id: String,
    },
    /// A selection: the ids the canvas had ticked.
    Notes {
        ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub spaces: Vec<Space>,
    /// The folders actually cited, for the same reason only the cited spaces travel.
    /// ⚠️ `default` so an export written before folders stays readable — and no
    /// `FORMAT_VERSION` bump, because such a file still parses.
    ///
    /// ⚠️ No geometry: `Folder` carries none. Where a zone sits is columns only the board
    /// query reads, so a board received from elsewhere cannot land on top of yours.
    #[serde(default)]
    pub folders: Vec<Folder>,
    pub notes: Vec<Note>,
    /// The records only — the bytes are entries of the archive, keyed by
    /// [`crate::attachments::model::stored_name`]. ⚠️ `default` so a `.json` export written
    /// before the archive existed still parses.
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub notes: u32,
    pub spaces: u32,
    pub folders: u32,
    /// What actually went into the archive. A record whose file has gone missing is left
    /// out rather than failing the export.
    pub attachments: u32,
    /// ⚠️ `false` means the file is readable by anyone who has it — every note, every
    /// screenshot. The interface says which of the two it wrote, because the file is the
    /// one thing here most likely to leave the machine.
    pub protected: bool,
}

/// `skipped`: notes already present or whose space is missing from the file — an import
/// has to be replayable without duplicating.
#[derive(Debug, Clone, Copy, Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub spaces_created: u32,
    /// Matched by name inside the destination space, and created when absent — the rule
    /// spaces already follow. Every library operation reports, including when it changed
    /// nothing.
    pub folders_created: u32,
    pub notes_imported: u32,
    pub notes_skipped: u32,
    /// Imported with a `language` or `kind` this build does not know brought down to the
    /// default. Counted so the loss is said rather than discovered.
    pub notes_degraded: u32,
    pub attachments_imported: u32,
    /// Records the archive named but did not carry. Counted rather than swallowed: the
    /// note arrives with a thumbnail that will never load, and only this says why.
    pub attachments_missing: u32,
}

/// A bundle read from a file, and the ids [`read_bundle`] had to degrade.
#[derive(Debug)]
pub struct IncomingBundle {
    pub bundle: Bundle,
    pub degraded: BTreeSet<String>,
}

/// ⚠️ The version is read off the raw JSON, before the bundle is built: a file from a
/// future format may not deserialise at all, and the designed message beats serde's.
pub fn read_bundle(json: &str) -> Result<IncomingBundle, StorageError> {
    let mut value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| StorageError::ImportFormat(error.to_string()))?;

    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();

    if version > u64::from(FORMAT_VERSION) {
        return Err(StorageError::ImportFormat(format!(
            "format version {version}, this version of DevNotes reads up to {FORMAT_VERSION}"
        )));
    }

    let degraded = degrade_unknown_values(&mut value);

    let bundle: Bundle = serde_json::from_value(value)
        .map_err(|error| StorageError::ImportFormat(error.to_string()))?;

    Ok(IncomingBundle { bundle, degraded })
}

/// `Note` deserialises `language` and `kind` as closed enums, so one value from a newer
/// DevNotes would fail the whole import. This degrades instead, like the database read
/// already does (`notes::store`, `TryFrom<NoteRow>`) — the title, body, tags and deadline
/// all still arrive, and the report says how many were touched.
///
/// ⚠️ What stays strict is the bridge: a value the front end cannot name is never written.
fn degrade_unknown_values(bundle: &mut serde_json::Value) -> BTreeSet<String> {
    let mut degraded = BTreeSet::new();

    let Some(notes) = bundle
        .get_mut("notes")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return degraded;
    };

    for note in notes {
        let language = degrade_field::<Language>(note, "language");
        let kind = degrade_field::<NoteKind>(note, "kind");

        if (language || kind)
            && let Some(id) = note.get("id").and_then(serde_json::Value::as_str)
        {
            degraded.insert(id.to_string());
        }
    }

    degraded
}

/// A field that is absent, or holds something other than a string, is left for serde to
/// judge: a malformed file is malformed, not a file from a newer version.
fn degrade_field<T: FromStr + Default + Display>(
    note: &mut serde_json::Value,
    field: &str,
) -> bool {
    let Some(current) = note.get(field).and_then(serde_json::Value::as_str) else {
        return false;
    };

    if current.parse::<T>().is_ok() {
        return false;
    }

    note[field] = serde_json::Value::String(T::default().to_string());
    true
}

pub fn validate_path(path: &str) -> Result<(), ValidationError> {
    if path.trim().is_empty() {
        return Err(ValidationError::new("path", "no file chosen"));
    }

    Ok(())
}

/// Longer than the longest run of backticks in the content, or a note that already holds
/// a Markdown block would cut its own in two.
fn fence_for(content: &str) -> String {
    let longest = content.split(|c| c != '`').map(str::len).max().unwrap_or(0);

    "`".repeat(longest.max(2) + 1)
}

/// A todo list comes out as a task list: it has no content, and an empty block pastes
/// nowhere.
pub fn to_markdown(notes: &[Note], space_names: &BTreeMap<String, String>) -> String {
    let mut out = String::new();

    for note in notes {
        if !out.is_empty() {
            out.push('\n');
        }

        let title = if note.title.trim().is_empty() {
            "—"
        } else {
            note.title.trim()
        };
        let _ = writeln!(out, "## {title}\n");

        let mut meta: Vec<String> = Vec::new();
        if let Some(space) = space_names.get(&note.space_id) {
            meta.push(space.clone());
        }
        if !note.source.trim().is_empty() {
            meta.push(note.source.trim().to_string());
        }
        if !note.tags.is_empty() {
            meta.push(
                note.tags
                    .iter()
                    .map(|tag| format!("#{tag}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        if !meta.is_empty() {
            let _ = writeln!(out, "_{}_\n", meta.join(" · "));
        }

        if note.kind == NoteKind::Checklist {
            let _ = writeln!(out, "{}", checklist::to_markdown(&note.items));
            continue;
        }

        let fence = fence_for(&note.content);
        let _ = writeln!(
            out,
            "{fence}{}\n{}\n{fence}",
            note.language,
            note.content.trim_end()
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::checklist::ChecklistItem;
    use crate::notes::fixtures::note as sample;

    fn spaces() -> BTreeMap<String, String> {
        BTreeMap::from([("s-1".to_string(), "Personal".to_string())])
    }

    #[test]
    fn a_shared_note_carries_its_space_context_and_tags() {
        let mut note = sample();
        note.source = "API Gateway / Auth".to_string();

        let markdown = to_markdown(&[note], &spaces());

        assert!(markdown.contains("## Title"));
        assert!(markdown.contains("_Personal · API Gateway / Auth · #auth_"));
        assert!(markdown.contains("```txt\nContent\n```"));
    }

    #[test]
    fn a_note_already_containing_a_fence_is_not_cut_in_half() {
        let mut note = sample();
        note.content = "```sh\necho hi\n```".to_string();

        let markdown = to_markdown(&[note], &spaces());

        assert!(markdown.contains("````txt"));
        assert!(markdown.ends_with("````\n"));
    }

    #[test]
    fn a_todo_list_is_shared_as_a_markdown_task_list() {
        let mut note = sample();
        note.kind = NoteKind::Checklist;
        note.content = String::new();
        note.items = vec![
            ChecklistItem {
                text: "Review".to_string(),
                done: true,
            },
            ChecklistItem {
                text: "Ship".to_string(),
                done: false,
            },
        ];

        let markdown = to_markdown(&[note], &spaces());

        assert!(markdown.contains("- [x] Review"));
        assert!(markdown.contains("- [ ] Ship"));
        assert!(!markdown.contains("```"));
    }

    #[test]
    fn an_untitled_note_still_gets_a_heading() {
        let mut note = sample();
        note.title = "   ".to_string();

        assert!(to_markdown(&[note], &spaces()).starts_with("## —"));
    }

    #[test]
    fn a_bundle_from_a_newer_version_is_refused_rather_than_half_read() {
        // The version is read off the raw JSON first, so the answer names the version
        // and not a serde field.
        let json = serde_json::json!({
            "version": FORMAT_VERSION + 1,
            "exportedAt": "2026-07-25T09:00:00.000Z",
            "spaces": [],
            "notes": [{ "shape": "from the future" }],
        })
        .to_string();

        let StorageError::ImportFormat(message) = read_bundle(&json).unwrap_err() else {
            panic!("expected a format error");
        };

        assert!(message.contains("format version"));
    }

    /// The file a newer DevNotes writes once its `Language` has grown a variant.
    fn bundle_with(field: &str, value: serde_json::Value) -> String {
        let mut json = serde_json::json!({
            "version": FORMAT_VERSION,
            "exportedAt": "2026-07-25T09:00:00.000Z",
            "spaces": [{ "id": "s-1", "name": "Personal" }],
            "notes": [serde_json::to_value(sample()).unwrap()],
        });
        json["notes"][0][field] = value;

        json.to_string()
    }

    #[test]
    fn an_unknown_language_is_brought_down_to_the_default_rather_than_refused() {
        let read = read_bundle(&bundle_with("language", "from-the-future".into())).unwrap();

        assert_eq!(read.bundle.notes[0].language, Language::default());
        assert_eq!(read.degraded.len(), 1);
        assert!(read.degraded.contains(&read.bundle.notes[0].id));
    }

    #[test]
    fn an_unknown_kind_is_brought_down_the_same_way() {
        let read = read_bundle(&bundle_with("kind", "from-the-future".into())).unwrap();

        assert_eq!(read.bundle.notes[0].kind, NoteKind::default());
        assert_eq!(read.degraded.len(), 1);
    }

    #[test]
    fn a_known_value_is_left_exactly_as_written() {
        let read = read_bundle(&bundle_with("language", "sql".into())).unwrap();

        assert_eq!(read.bundle.notes[0].language.to_string(), "sql");
        assert!(read.degraded.is_empty());
    }

    /// Only a string this build cannot name is degraded.
    #[test]
    fn a_language_that_is_not_a_string_is_still_a_format_error() {
        assert!(matches!(
            read_bundle(&bundle_with("language", 42.into())).unwrap_err(),
            StorageError::ImportFormat(_)
        ));
    }

    #[test]
    fn a_file_that_is_not_a_bundle_is_reported_as_such() {
        assert!(matches!(
            read_bundle("{\"hello\":true}").unwrap_err(),
            StorageError::ImportFormat(_)
        ));
    }

    #[test]
    fn a_bundle_round_trips_through_its_own_format() {
        let bundle = Bundle {
            version: FORMAT_VERSION,
            exported_at: sample().created_at,
            spaces: vec![Space {
                id: "s-1".to_string(),
                name: "Personal".to_string(),
                pinned: false,
            }],
            folders: Vec::new(),
            notes: vec![sample()],
            attachments: Vec::new(),
        };

        let read = read_bundle(&serde_json::to_string(&bundle).unwrap()).unwrap();

        assert_eq!(read.bundle.notes.len(), 1);
        assert_eq!(read.bundle.notes[0].title, "Title");
        assert_eq!(read.bundle.spaces[0].name, "Personal");
        assert!(read.degraded.is_empty());
    }
}
