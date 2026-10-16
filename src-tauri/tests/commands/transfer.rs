use devnotes_lib::error::ErrorCode;
use devnotes_lib::notes::create_note;
use devnotes_lib::transfer::model::ExportScope;
use devnotes_lib::transfer::{
    count_export, export_is_protected, export_notes, import_notes, share_notes,
};

use super::common::snippet;
use super::{Session, code};

/// Out of one library and into another, which gains the space and the note.
#[test]
fn an_export_is_imported_into_another_library() {
    let scratch = tempfile::tempdir().unwrap();
    let source = Session::on_disk(&scratch.path().join("source"));
    let target = Session::on_disk(&scratch.path().join("target"));
    let space = source.space("Personal");
    source
        .call(|app| create_note(snippet(&space), app))
        .unwrap();
    let path = scratch
        .path()
        .join("notes.devnotes")
        .to_string_lossy()
        .to_string();

    let exported = source
        .call(|app| export_notes(path.clone(), ExportScope::Library, None, app))
        .unwrap();
    let protected = export_is_protected(path.clone()).unwrap();
    let imported = target.call(|app| import_notes(path, None, app)).unwrap();

    assert_eq!((exported.notes, exported.protected), (1, false));
    assert!(!protected);
    assert_eq!(imported.notes_imported, 1);
    assert_eq!(target.spaces(), ["Personal"]);
}

/// The floor that holds for a library's phrase holds for a file meant to travel.
#[test]
fn an_export_under_a_short_phrase_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(scratch.path());
    let path = scratch
        .path()
        .join("notes.devnotes")
        .to_string_lossy()
        .to_string();

    assert!(matches!(
        code(session.call(|app| {
            export_notes(path, ExportScope::Library, Some("short".to_string()), app)
        })),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_shared_selection_reads_as_markdown() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = session
        .call(|app| create_note(snippet(&space), app))
        .unwrap()
        .note
        .id;

    let markdown = session.call(|app| share_notes(vec![id], app)).unwrap();

    assert!(markdown.contains("Titre"));
    assert!(markdown.contains("Contenu"));
}

/// Counted before any dialog, and an export of nothing leaves no file behind.
#[test]
fn an_empty_scope_is_counted_as_such_and_exports_no_file() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(&scratch.path().join("library"));
    let full = session.space("Personal");
    let empty = session.space("Vide");
    session
        .call(|app| create_note(snippet(&full), app))
        .unwrap();
    let path = scratch.path().join("empty.devnotes");

    let counted = session
        .call(|app| {
            count_export(
                ExportScope::Space {
                    space_id: empty.clone(),
                },
                app,
            )
        })
        .unwrap();
    let report = session
        .call(|app| {
            export_notes(
                path.to_string_lossy().to_string(),
                ExportScope::Space { space_id: empty },
                None,
                app,
            )
        })
        .unwrap();

    assert_eq!(counted, 0);
    assert_eq!(report.notes, 0);
    assert!(!path.exists());
    assert_eq!(
        session
            .call(|app| count_export(ExportScope::Library, app))
            .unwrap(),
        1
    );
}
