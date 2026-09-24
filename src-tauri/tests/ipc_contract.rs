//! The JSON shape of everything that crosses the Tauri bridge, gathered here rather
//! than scattered beside each type: it is **one** contract, and the compiler checks
//! none of it.

use std::collections::BTreeMap;

use devnotes_lib::attachments::model::Attachment;
use devnotes_lib::db::iso8601;
use devnotes_lib::error::StorageError;
use devnotes_lib::error::ValidationError;
use devnotes_lib::error::{AppError, ErrorCode};
use devnotes_lib::notes::checklist::{ChecklistItem, NoteKind};
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{
    DisplayNote, Note, NoteDraft, NoteLifecycle, NotePatch, TagUsage, decorate,
};
use devnotes_lib::notes::trash;
use devnotes_lib::notes::view::{NoteFilter, NoteSection, NoteSectionKey, NotesQuery, NotesView};
use devnotes_lib::spaces::model::{Space, SpaceDraft};
use devnotes_lib::transfer;
use devnotes_lib::transfer::model::{Bundle, ImportReport};

const NOW: &str = "2026-07-25T09:00:00.000Z";

mod common;
use common::at;

fn sample() -> Note {
    Note {
        id: "n-1".to_string(),
        space_id: "s-1".to_string(),
        folder_id: None,
        title: "Title".to_string(),
        language: Language::Txt,
        content: "Contenu".to_string(),
        source: String::new(),
        tags: vec!["auth".to_string()],
        pinned: false,
        created_at: at(NOW),
        updated_at: at(NOW),
        lifecycle: NoteLifecycle::Permanent,
        kind: NoteKind::Snippet,
        items: Vec::new(),
        placeholder_values: BTreeMap::new(),
    }
}

fn displayed(note: Note) -> DisplayNote {
    decorate(note, at(NOW))
}

#[test]
fn a_note_serializes_with_camel_case_keys() {
    let json = serde_json::to_value(sample()).unwrap();

    assert!(json.get("spaceId").is_some());
    assert!(json.get("createdAt").is_some());
    assert!(json.get("updatedAt").is_some());
    assert!(json.get("space_id").is_none());
    assert!(json.get("created_at").is_none());
}

#[test]
fn a_permanent_lifecycle_serializes_as_a_tagged_object() {
    let json = serde_json::to_value(sample()).unwrap();

    assert_eq!(
        json["lifecycle"],
        serde_json::json!({ "kind": "permanent" })
    );
}

#[test]
fn an_expiring_lifecycle_serializes_flat_with_its_date() {
    let note = Note {
        lifecycle: NoteLifecycle::Expires {
            at: at("2026-08-01T00:00:00.000Z"),
        },
        ..sample()
    };

    let json = serde_json::to_value(note).unwrap();

    // The date is compared as an instant: the exact ISO spelling is not a contract.
    assert_eq!(json["lifecycle"]["kind"], "expires");
    assert_eq!(
        iso8601::parse(json["lifecycle"]["at"].as_str().unwrap()).unwrap(),
        at("2026-08-01T00:00:00.000Z")
    );
}

#[test]
fn a_patch_omitting_a_field_deserializes_to_none() {
    let patch: NotePatch = serde_json::from_value(serde_json::json!({
        "title": "New title"
    }))
    .unwrap();

    assert_eq!(patch.title.as_deref(), Some("New title"));
    assert!(patch.content.is_none());
    assert!(patch.tags.is_none());
    assert!(patch.lifecycle.is_none());
}

#[test]
fn a_draft_is_read_from_the_camel_case_payload_the_front_sends() {
    let draft: NoteDraft = serde_json::from_value(serde_json::json!({
        "spaceId": "s-1",
        "title": "",
        "language": "sql",
        "content": "SELECT 1",
        "source": "",
        "tags": ["db"],
        "pinned": true,
        "lifecycle": { "kind": "expires", "at": "2026-08-01T00:00:00.000Z" }
    }))
    .unwrap();

    assert_eq!(draft.space_id, "s-1");
    assert!(draft.pinned);
    assert!(matches!(draft.lifecycle, NoteLifecycle::Expires { .. }));
}

#[test]
fn a_decorated_note_serializes_flat_with_its_footer() {
    let json = serde_json::to_value(displayed(sample())).unwrap();

    assert_eq!(json["id"], "n-1");
    assert_eq!(json["spaceId"], "s-1");
    assert_eq!(json["expiringSoon"], false);
    assert_eq!(json["footer"]["kind"], "age");
    assert_eq!(
        iso8601::parse(json["footer"]["at"].as_str().unwrap()).unwrap(),
        at(NOW)
    );
    assert!(json.get("note").is_none());
}

#[test]
fn a_source_footer_serializes_with_the_kind_the_front_discriminates_on() {
    let note = Note {
        pinned: true,
        source: "API Gateway / Auth".to_string(),
        ..sample()
    };

    let json = serde_json::to_value(displayed(note)).unwrap();

    assert_eq!(
        json["footer"],
        serde_json::json!({ "kind": "source", "value": "API Gateway" })
    );
}

#[test]
fn a_view_serializes_with_camel_case_keys() {
    let view = NotesView {
        sections: vec![NoteSection {
            key: NoteSectionKey::Week,
            notes: vec![displayed(sample())],
            has_expiring_notes: false,
            show_create_ghost: true,
        }],
        available_tags: vec!["auth".to_string()],
        available_languages: vec![Language::Json],
        is_filtering: false,
        matched: 1,
    };

    let json = serde_json::to_value(view).unwrap();

    assert!(json.get("availableTags").is_some());
    assert!(json.get("availableLanguages").is_some());
    assert!(json.get("isFiltering").is_some());
    assert!(json.get("available_tags").is_none());
    assert!(json.get("available_languages").is_none());
    assert!(json["sections"][0].get("hasExpiringNotes").is_some());
    assert!(json["sections"][0].get("showCreateGhost").is_some());
}

#[test]
fn a_section_key_serializes_as_the_translation_key_the_front_expects() {
    let section = NoteSection {
        key: NoteSectionKey::Older,
        notes: Vec::new(),
        has_expiring_notes: false,
        show_create_ghost: false,
    };

    let json = serde_json::to_value(section).unwrap();

    assert_eq!(json["key"], "older");
}

#[test]
fn a_query_is_read_from_the_camel_case_payload_the_front_sends() {
    let query: NotesQuery = serde_json::from_value(serde_json::json!({
        "spaceId": "s-1",
        "search": "deploy",
        "filter": "untriaged",
        "tags": ["urgent"],
        "languages": ["json", "yml"],
        "now": NOW,
        "tzOffsetMinutes": -120,
        "pinnedFirst": true
    }))
    .unwrap();

    assert_eq!(query.space_id.as_deref(), Some("s-1"));
    assert_eq!(query.filter, NoteFilter::Untriaged);
    assert_eq!(query.languages, [Language::Json, Language::Yml]);
    assert_eq!(query.tz_offset_minutes, -120);
    assert!(query.pinned_first);
}

#[test]
fn a_null_space_is_read_as_every_space() {
    let query: NotesQuery = serde_json::from_value(serde_json::json!({
        "spaceId": null,
        "search": "",
        "filter": "all",
        "tags": [],
        "languages": [],
        "now": NOW,
        "tzOffsetMinutes": 0,
        "pinnedFirst": false
    }))
    .unwrap();

    assert!(query.space_id.is_none());
}

/// `rename_all` has no effect while every field is one word: this test fails the day
/// a `created_at` is added without the attribute.
#[test]
fn a_space_serializes_with_the_keys_the_front_reads() {
    let json = serde_json::to_value(Space {
        id: "s-1".to_string(),
        name: "Personal".to_string(),
        pinned: true,
    })
    .unwrap();

    assert_eq!(
        json,
        serde_json::json!({ "id": "s-1", "name": "Personal", "pinned": true })
    );
}

#[test]
fn a_space_draft_is_read_from_the_payload_the_front_sends() {
    let draft: SpaceDraft =
        serde_json::from_value(serde_json::json!({ "name": "Boulot" })).unwrap();

    assert_eq!(draft.name, "Boulot");
}

#[test]
fn a_code_serializes_in_camel_case() {
    let json = serde_json::to_value(AppError::from(StorageError::NoteNotFound(
        "n-1".to_string(),
    )))
    .unwrap();

    assert_eq!(json["code"], "noteNotFound");
}

#[test]
fn a_duplicate_space_name_carries_the_name_as_a_parameter() {
    let json = serde_json::to_value(AppError::from(StorageError::DuplicateSpaceName(
        "Personal".to_string(),
    )))
    .unwrap();

    assert_eq!(json["code"], "duplicateSpaceName");
    assert_eq!(json["params"]["name"], "Personal");
}

#[test]
fn every_error_carries_a_non_empty_detail() {
    let errors = [
        StorageError::NoteNotFound("n-1".to_string()),
        StorageError::SpaceNotFound("s-1".to_string()),
        StorageError::DuplicateSpaceName("Personal".to_string()),
        StorageError::SchemaTooRecent("2099-01-01-000000".to_string()),
        StorageError::Migration("base verrouillée".to_string()),
    ];

    for error in errors {
        assert!(!AppError::from(error).detail.is_empty());
    }
}

#[test]
fn a_refused_value_names_the_field_at_fault() {
    let json = serde_json::to_value(AppError::from(ValidationError::new(
        "language",
        "\"from-the-future\" is not a known language",
    )))
    .unwrap();

    assert_eq!(json["code"], "invalidInput");
    assert_eq!(json["params"]["field"], "language");
}

#[test]
fn a_schema_too_recent_degrades_to_storage_rather_than_leaking_a_dead_code() {
    // It cannot cross the bridge — it aborts startup — so `storage` is the honest code.
    let error = AppError::from(StorageError::SchemaTooRecent(
        "2099-01-01-000000".to_string(),
    ));

    assert!(matches!(error.code, ErrorCode::Storage));
    assert!(error.detail.contains("2099-01-01-000000"));
}

#[test]
fn params_are_absent_rather_than_null_when_there_is_nothing_to_interpolate() {
    let json = serde_json::to_value(AppError::storage_unavailable()).unwrap();

    assert_eq!(json["code"], "storageUnavailable");
    assert_eq!(json["params"], serde_json::json!({}));
}

#[test]
fn an_unreadable_reference_instant_is_refused_at_the_bridge() {
    let refused = serde_json::from_value::<NotesQuery>(serde_json::json!({
        "spaceId": null,
        "search": "",
        "filter": "all",
        "tags": [],
        "languages": [],
        "now": "hier",
        "tzOffsetMinutes": 0
    }));

    assert!(refused.is_err());
}

#[test]
fn an_unknown_language_is_refused_at_the_bridge() {
    let refused = serde_json::from_value::<NoteDraft>(serde_json::json!({
        "spaceId": "s-1",
        "title": "",
        "language": "from-the-future",
        "content": "",
        "source": "",
        "tags": [],
        "pinned": false,
        "lifecycle": { "kind": "permanent" }
    }));

    assert!(refused.is_err());
}

#[test]
fn an_instant_crosses_as_a_string_the_front_can_read_as_a_date() {
    // JSON has no date type: the **column** demands its milliseconds, not the bridge.
    let json = serde_json::to_value(sample()).unwrap();

    let updated_at = json["updatedAt"].as_str().unwrap();
    assert!(iso8601::parse(updated_at).is_ok());
}

#[test]
fn a_trashed_note_is_a_note_with_two_dates_more() {
    let json = serde_json::to_value(trash::trashed(sample(), at(NOW))).unwrap();

    assert_eq!(json["id"], "n-1");
    assert!(iso8601::parse(json["deletedAt"].as_str().unwrap()).is_ok());
    assert!(iso8601::parse(json["purgeAt"].as_str().unwrap()).is_ok());
}

#[test]
fn a_decorated_note_announces_its_fields_and_its_attachments() {
    let mut note = sample();
    note.content = "psql -h {{host}} -p {{port=5432}}".to_string();
    note.placeholder_values = BTreeMap::from([("host".to_string(), "db.internal".to_string())]);

    let json = serde_json::to_value(displayed(note)).unwrap();

    assert_eq!(json["placeholders"][0]["name"], "host");
    assert_eq!(json["placeholders"][1]["defaultValue"], "5432");
    assert_eq!(json["placeholders"][0]["value"], "db.internal");
    assert_eq!(json["placeholders"][1]["value"], "");
    assert_eq!(json["attachmentCount"], 0);
    assert_eq!(json["truncated"], false);
}

#[test]
fn an_attachment_serializes_with_camel_case_keys() {
    let json = serde_json::to_value(Attachment {
        id: "a-1".to_string(),
        note_id: "n-1".to_string(),
        file_name: "capture.png".to_string(),
        mime_type: "image/png".to_string(),
        byte_size: 42,
        created_at: at(NOW),
    })
    .unwrap();

    assert_eq!(json["noteId"], "n-1");
    assert_eq!(json["fileName"], "capture.png");
    assert_eq!(json["mimeType"], "image/png");
    assert_eq!(json["byteSize"], 42);
    assert!(json.get("note_id").is_none());
}

#[test]
fn a_tag_usage_carries_its_count_under_a_camel_case_key() {
    let json = serde_json::to_value(TagUsage {
        tag: "auth".to_string(),
        note_count: 3,
    })
    .unwrap();

    assert_eq!(json["tag"], "auth");
    assert_eq!(json["noteCount"], 3);
}

#[test]
fn an_import_report_names_what_it_skipped() {
    let json = serde_json::to_value(ImportReport {
        spaces_created: 1,
        folders_created: 5,
        notes_imported: 2,
        notes_skipped: 3,
        notes_degraded: 4,
        attachments_imported: 1,
        attachments_missing: 0,
    })
    .unwrap();

    assert_eq!(json["spacesCreated"], 1);
    assert_eq!(json["foldersCreated"], 5);
    assert_eq!(json["notesImported"], 2);
    assert_eq!(json["notesSkipped"], 3);
    assert_eq!(json["notesDegraded"], 4);
}

#[test]
fn an_export_bundle_reads_back_the_notes_it_wrote() {
    // The file is a contract between two versions of DevNotes, not only between Rust
    // and the front end.
    let bundle = Bundle {
        version: transfer::model::FORMAT_VERSION,
        exported_at: at(NOW),
        spaces: vec![Space {
            id: "s-1".to_string(),
            name: "Personal".to_string(),
            pinned: false,
        }],
        folders: Vec::new(),
        notes: vec![sample()],
        attachments: Vec::new(),
    };
    let json = serde_json::to_string(&bundle).unwrap();

    assert!(json.contains("\"exportedAt\""));
    let read = transfer::model::read_bundle(&json).unwrap();
    assert_eq!(read.bundle.notes[0].id, "n-1");
}

#[test]
fn a_note_announces_its_kind_and_its_items() {
    let note = Note {
        kind: NoteKind::Checklist,
        items: vec![ChecklistItem {
            text: "Relire".to_string(),
            done: true,
        }],
        ..sample()
    };

    let json = serde_json::to_value(note).unwrap();

    assert_eq!(json["kind"], serde_json::json!("checklist"));
    assert_eq!(json["items"][0]["text"], serde_json::json!("Relire"));
    assert_eq!(json["items"][0]["done"], serde_json::json!(true));
}

#[test]
fn an_ordinary_note_still_crosses_as_a_snippet() {
    let json = serde_json::to_value(sample()).unwrap();

    assert_eq!(json["kind"], serde_json::json!("snippet"));
    assert_eq!(json["items"], serde_json::json!([]));
}

#[test]
fn an_export_written_before_todo_lists_existed_still_reads() {
    // `Bundle` deserializes `Note` itself: without `#[serde(default)]` every file
    // already exported would become unreadable.
    let json = serde_json::json!({
        "version": transfer::model::FORMAT_VERSION,
        "exportedAt": NOW,
        "spaces": [],
        "notes": [{
            "id": "n-1",
            "spaceId": "s-1",
            "title": "Title",
            "language": "txt",
            "content": "Contenu",
            "source": "",
            "tags": [],
            "pinned": false,
            "createdAt": NOW,
            "updatedAt": NOW,
            "lifecycle": { "kind": "permanent" }
        }],
    })
    .to_string();

    let read = transfer::model::read_bundle(&json).unwrap();

    assert_eq!(read.bundle.notes[0].kind, NoteKind::Snippet);
    assert!(read.bundle.notes[0].items.is_empty());
    assert!(read.bundle.notes[0].placeholder_values.is_empty());
}

#[test]
fn the_values_of_the_fields_cross_as_a_named_map() {
    let mut note = sample();
    note.placeholder_values = BTreeMap::from([("host".to_string(), "db.internal".to_string())]);

    let json = serde_json::to_value(note).unwrap();

    assert_eq!(
        json["placeholderValues"],
        serde_json::json!({ "host": "db.internal" })
    );
}

#[test]
fn a_patch_omitting_the_items_deserializes_to_none() {
    let patch: NotePatch = serde_json::from_value(serde_json::json!({ "title": "T" })).unwrap();

    assert!(patch.items.is_none());
    assert!(patch.kind.is_none());
}

#[test]
fn every_error_code_crosses_as_a_camel_case_string() {
    for (error, expected) in [
        (
            StorageError::AttachmentNotFound("a-1".to_string()),
            "attachmentNotFound",
        ),
        (StorageError::File("disk".to_string()), "fileAccess"),
        (
            StorageError::RevisionNotFound("r-1".to_string()),
            "revisionNotFound",
        ),
        (
            StorageError::LibraryNotFound("l-1".to_string()),
            "libraryNotFound",
        ),
        (StorageError::LibraryOpen, "libraryOpen"),
        (StorageError::LastLibrary, "lastLibrary"),
        (
            StorageError::NothingToSetAside("here".to_string()),
            "nothingToSetAside",
        ),
        (
            StorageError::BackupNotFound("2026-07-25_09-00-00".to_string()),
            "backupNotFound",
        ),
        (
            StorageError::BackupUnopenable("2026-07-25_09-00-00".to_string()),
            "backupUnopenable",
        ),
        (
            StorageError::ImportFormat("nope".to_string()),
            "importFormat",
        ),
    ] {
        let json = serde_json::to_value(AppError::from(error)).unwrap();

        assert_eq!(json["code"], expected);
    }
}
