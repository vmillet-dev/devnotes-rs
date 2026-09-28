use devnotes_lib::attachments::{
    attach_file, delete_attachment, list_attachments, read_attachment, save_attachment,
};
use devnotes_lib::error::ErrorCode;
use devnotes_lib::notes::create_note;

use super::common::snippet;
use super::{Session, code};

/// Stored sealed beside the library, handed back as the `data:` URI the preview reads, and
/// saved in the clear where the user chose.
#[test]
fn a_file_is_attached_read_saved_and_deleted() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(scratch.path());
    let space = session.space("Personal");
    let note = session
        .call(|app| create_note(snippet(&space), app))
        .unwrap()
        .note
        .id;
    let source = scratch.path().join("capture.png");
    std::fs::write(&source, b"\x89PNG").unwrap();
    let saved = scratch.path().join("saved.png");

    let attachment = session
        .call(|app| attach_file(note.clone(), source.to_string_lossy().to_string(), app))
        .unwrap();
    let listed = session
        .call(|app| list_attachments(note.clone(), app))
        .unwrap();
    let uri = session
        .call(|app| read_attachment(attachment.id.clone(), app))
        .unwrap();
    session
        .call(|app| {
            save_attachment(
                attachment.id.clone(),
                saved.to_string_lossy().to_string(),
                app,
            )
        })
        .unwrap();
    session
        .call(|app| delete_attachment(attachment.id.clone(), app))
        .unwrap();

    assert_eq!(attachment.file_name, "capture.png");
    assert_eq!(listed.len(), 1);
    assert_eq!(uri, "data:image/png;base64,iVBORw==");
    assert_eq!(std::fs::read(&saved).unwrap(), b"\x89PNG");
    assert!(
        session
            .call(|app| list_attachments(note, app))
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        code(session.call(|app| read_attachment(attachment.id, app))),
        ErrorCode::AttachmentNotFound
    ));
}

#[test]
fn a_file_that_cannot_be_read_is_named_as_such() {
    let scratch = tempfile::tempdir().unwrap();
    let session = Session::on_disk(scratch.path());
    let missing = scratch.path().join("gone.png");

    assert!(matches!(
        code(session.call(|app| {
            attach_file(
                "n-1".to_string(),
                missing.to_string_lossy().to_string(),
                app,
            )
        })),
        ErrorCode::FileAccess
    ));
}
