use engine::error::ErrorCode;
use engine::notes::create_note;
use engine::transfer::model::ExportScope;
use engine::transfer::{
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
    source.call(|e| create_note(e, snippet(&space))).unwrap();
    let path = scratch
        .path()
        .join("notes.devnotes")
        .to_string_lossy()
        .to_string();

    let exported = source
        .wait(|e| export_notes(e, path.clone(), ExportScope::Library, None))
        .unwrap();
    let protected = export_is_protected(path.clone()).unwrap();
    let imported = target.wait(|e| import_notes(e, path, None)).unwrap();

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
        code(session.wait(|e| export_notes(
            e,
            path,
            ExportScope::Library,
            Some("short".to_string())
        ))),
        ErrorCode::InvalidInput
    ));
}

#[test]
fn a_shared_selection_reads_as_markdown() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;

    let markdown = session.call(|e| share_notes(e, vec![id])).unwrap();

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
    session.call(|e| create_note(e, snippet(&full))).unwrap();
    let path = scratch.path().join("empty.devnotes");

    let counted = session
        .call(|e| {
            count_export(
                e,
                ExportScope::Space {
                    space_id: empty.clone(),
                },
            )
        })
        .unwrap();
    let report = session
        .wait(|e| {
            export_notes(
                e,
                path.to_string_lossy().to_string(),
                ExportScope::Space { space_id: empty },
                None,
            )
        })
        .unwrap();

    assert_eq!(counted, 0);
    assert_eq!(report.notes, 0);
    assert!(!path.exists());
    assert_eq!(
        session
            .call(|e| count_export(e, ExportScope::Library))
            .unwrap(),
        1
    );
}

/// The export is the one copy that needs no passphrase: an attachment whose file cannot be
/// opened is left out and counted, as a missing one is, rather than costing the whole file
/// (#1478, N52).
#[test]
fn an_attachment_that_cannot_be_opened_is_left_out_of_the_export_and_counted() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(&scratch.path().join("library"));
    let space = session.space("Personal");
    let note = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;
    let source = scratch.path().join("capture.png");
    std::fs::write(&source, b"\x89PNG").unwrap();
    let attachment = session
        .call(|e| engine::attachments::attach_file(e, note, source.to_string_lossy().to_string()))
        .unwrap();
    let stored = engine::attachments::files::directory(
        session.engine.db().lock().unwrap().as_ref().unwrap(),
    )
    .join(attachment.stored_name());
    std::fs::write(&stored, b"not sealed").unwrap();
    let path = scratch
        .path()
        .join("notes.devnotes")
        .to_string_lossy()
        .to_string();

    let report = session
        .call(|e| export_notes(e, path, ExportScope::Library, None))
        .unwrap();

    assert_eq!((report.notes, report.attachments), (1, 0));
    assert_eq!(report.attachments_unreadable, 1);
}
