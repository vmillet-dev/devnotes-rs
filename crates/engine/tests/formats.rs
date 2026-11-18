//! The JSON shape of what the core writes and reads back (an export, its notes, spaces and
//! attachments), and the codes its errors carry: the compiler checks none of it.

use std::collections::BTreeMap;

use engine::attachments::model::Attachment;
use engine::db::iso8601;
use engine::error::StorageError;
use engine::error::ValidationError;
use engine::error::{AppError, ErrorCode};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{Note, NoteLifecycle};
use engine::notes::priority::Priority;
use engine::spaces::model::Space;
use engine::transfer;
use engine::transfer::model::Bundle;

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
        priority: Priority::None,
        items: Vec::new(),
        placeholder_values: BTreeMap::new(),
    }
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

/// `rename_all` has no effect while every field is one word: this test fails the day
/// a `created_at` is added without the attribute.
#[test]
fn a_space_serializes_with_camel_case_keys() {
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
fn a_schema_too_recent_degrades_to_storage_rather_than_leaking_a_dead_code() {
    // It aborts startup, so `storage` is the honest code.
    let error = AppError::from(StorageError::SchemaTooRecent(
        "2099-01-01-000000".to_string(),
    ));

    assert!(matches!(error.code, ErrorCode::Storage));
    assert!(error.detail.contains("2099-01-01-000000"));
}

#[test]
fn an_instant_is_written_as_a_string_read_back_as_a_date() {
    let json = serde_json::to_value(sample()).unwrap();

    let updated_at = json["updatedAt"].as_str().unwrap();
    assert!(iso8601::parse(updated_at).is_ok());
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
fn an_export_bundle_reads_back_the_notes_it_wrote() {
    // The file is a contract between two versions of DevNotes.
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
fn an_ordinary_note_still_reads_as_a_snippet() {
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
fn the_values_of_the_fields_are_written_as_a_named_map() {
    let mut note = sample();
    note.placeholder_values = BTreeMap::from([("host".to_string(), "db.internal".to_string())]);

    let json = serde_json::to_value(note).unwrap();

    assert_eq!(
        json["placeholderValues"],
        serde_json::json!({ "host": "db.internal" })
    );
}

#[test]
fn an_error_carries_its_code_and_what_to_interpolate() {
    let duplicate = AppError::from(StorageError::DuplicateSpaceName("Personal".to_string()));
    assert_eq!(duplicate.code, ErrorCode::DuplicateSpaceName);
    assert_eq!(duplicate.params["name"], "Personal");

    let missing = AppError::from(StorageError::HttpItemNotFound("r-1".to_string()));
    assert_eq!(missing.code, ErrorCode::HttpItemNotFound);
    assert_eq!(missing.params["id"], "r-1");

    let refused = AppError::from(StorageError::from(ValidationError::new(
        "folderId",
        "a folder cannot move into itself",
    )));
    assert_eq!(refused.code, ErrorCode::InvalidInput);
    assert_eq!(refused.params["field"], "folderId");

    assert!(AppError::storage_unavailable().params.is_empty());
}
