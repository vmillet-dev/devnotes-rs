use devnotes_lib::db::Library;

use devnotes_lib::attachments::model::Attachment;
use devnotes_lib::attachments::store as attachments;
use devnotes_lib::db::open_in_memory;
use devnotes_lib::notes::checklist::NoteKind;
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::NoteDraft;
use devnotes_lib::notes::store as notes;
use devnotes_lib::spaces::store as spaces;
use devnotes_lib::transfer::bundle::{self, collect, merge as merge_bundle};
use devnotes_lib::transfer::file;
use devnotes_lib::transfer::file::Payload;
use devnotes_lib::transfer::model::{self, Bundle, ExportScope, ImportReport, IncomingBundle};

mod common;
use common::{snippet, t0};

fn draft(space_id: &str, title: &str) -> NoteDraft {
    NoteDraft {
        title: title.to_string(),
        language: Language::Sql,
        content: "select 1".to_string(),
        source: "API / Auth".to_string(),
        tags: vec!["auth".to_string()],
        ..snippet(space_id)
    }
}

/// No attachment travels in these scenarios — the archive itself is covered in
/// `transfer::file`. `Payload::Empty` hands over no bytes, so the directory is never
/// written to.
fn merge(connection: &mut Library, bundle: IncomingBundle) -> ImportReport {
    merge_bundle(
        connection,
        bundle,
        &mut Payload::Empty,
        &std::env::temp_dir(),
    )
    .unwrap()
}

/// A populated database, the way a user would have one.
fn library() -> Library {
    let mut connection = open_in_memory().unwrap();
    let personal = spaces::create(&mut connection, "Personal").unwrap().id;
    let boulot = spaces::create(&mut connection, "Boulot").unwrap().id;
    notes::create(&mut connection, draft(&personal, "Première"), t0()).unwrap();
    notes::create(&mut connection, draft(&boulot, "Seconde"), t0()).unwrap();

    connection
}

fn exported(connection: &mut Library) -> Bundle {
    let all = notes::all(connection, None).unwrap();
    collect(connection, all).unwrap()
}

/// The file as it is really written and read back, serialization included.
fn round_tripped(bundle: &Bundle) -> IncomingBundle {
    model::read_bundle(&serde_json::to_string(bundle).unwrap()).unwrap()
}

/// The file a newer DevNotes would write: the same bundle, with a value in `field`
/// that this build has never heard of.
fn written_by_a_newer_version(bundle: &Bundle, field: &str, value: &str) -> IncomingBundle {
    let mut json: serde_json::Value = serde_json::to_value(bundle).unwrap();
    json["notes"][0][field] = serde_json::Value::String(value.to_string());

    model::read_bundle(&json.to_string()).unwrap()
}

#[test]
fn a_library_moves_whole_to_another_machine() {
    let mut source = library();
    let bundle = round_tripped(&exported(&mut source));

    let mut target = open_in_memory().unwrap();
    let report = merge(&mut target, bundle);

    assert_eq!(report.notes_imported, 2);
    assert_eq!(report.spaces_created, 2);
    assert_eq!(report.notes_skipped, 0);

    let arrived = notes::all(&mut target, None).unwrap();
    let mut titles: Vec<&str> = arrived.iter().map(|note| note.title.as_str()).collect();
    titles.sort_unstable();
    assert_eq!(titles, ["Première", "Seconde"]);
    assert_eq!(arrived[0].content, "select 1");
    assert_eq!(arrived[0].tags, ["auth"]);
    assert_eq!(arrived[0].created_at, t0());
    assert_eq!(spaces::list(&mut target).unwrap().len(), 2);
}

#[test]
fn only_the_spaces_actually_cited_travel() {
    let mut source = library();
    let personal = spaces::list(&mut source).unwrap()[1].id.clone();
    let single = notes::all(&mut source, Some(&personal)).unwrap();

    let bundle = collect(&mut source, single).unwrap();

    assert_eq!(bundle.spaces.len(), 1);
    assert_eq!(bundle.notes.len(), 1);
}

#[test]
fn an_export_scope_takes_the_library_a_space_or_a_selection() {
    let mut source = library();
    let personal = spaces::list(&mut source).unwrap()[1].id.clone();
    let one = notes::all(&mut source, None).unwrap()[0].id.clone();

    let count =
        |source: &mut Library, scope: ExportScope| bundle::of(source, &scope).unwrap().notes.len();

    assert_eq!(count(&mut source, ExportScope::Library), 2);
    assert_eq!(
        count(&mut source, ExportScope::Space { space_id: personal }),
        1
    );
    assert_eq!(count(&mut source, ExportScope::Notes { ids: vec![one] }), 1);
}

/// The shape the front end sends, `kind` and camelCase included.
#[test]
fn an_export_scope_reads_the_way_the_bindings_write_it() {
    let scope: ExportScope = serde_json::from_str(r#"{"kind":"space","spaceId":"s-1"}"#).unwrap();

    assert!(matches!(scope, ExportScope::Space { space_id } if space_id == "s-1"));
}

#[test]
fn importing_the_same_file_twice_adds_nothing_the_second_time() {
    let mut source = library();
    let file = exported(&mut source);

    let mut target = open_in_memory().unwrap();
    merge(&mut target, round_tripped(&file));
    let second = merge(&mut target, round_tripped(&file));

    assert_eq!(second.notes_imported, 0);
    assert_eq!(second.notes_skipped, 2);
    assert_eq!(second.spaces_created, 0);
    assert_eq!(notes::all(&mut target, None).unwrap().len(), 2);
}

#[test]
fn reimporting_into_the_base_it_came_from_changes_nothing() {
    let mut library = library();
    let bundle = round_tripped(&exported(&mut library));

    let report = merge(&mut library, bundle);

    assert_eq!(report.notes_imported, 0);
    assert_eq!(report.notes_skipped, 2);
    assert_eq!(notes::all(&mut library, None).unwrap().len(), 2);
}

#[test]
fn a_space_of_the_same_name_is_reused_rather_than_duplicated() {
    let mut source = library();
    let bundle = round_tripped(&exported(&mut source));

    let mut target = open_in_memory().unwrap();
    spaces::create(&mut target, "PERSONAL").unwrap();

    let report = merge(&mut target, bundle);

    assert_eq!(report.spaces_created, 1);
    assert_eq!(spaces::list(&mut target).unwrap().len(), 2);
}

#[test]
fn a_note_whose_space_is_missing_from_the_file_is_skipped_not_misfiled() {
    let mut source = library();
    let mut incoming = round_tripped(&exported(&mut source));
    incoming.bundle.spaces.clear();

    let mut target = open_in_memory().unwrap();
    let report = merge(&mut target, incoming);

    assert_eq!(report.notes_imported, 0);
    assert_eq!(report.notes_skipped, 2);
}

#[test]
fn a_trashed_note_does_not_leave_with_the_export() {
    let mut source = library();
    let thrown = notes::all(&mut source, None).unwrap()[0].id.clone();
    notes::trash::trash(&mut source, &thrown, t0()).unwrap();

    let bundle = exported(&mut source);

    assert_eq!(bundle.notes.len(), 1);
}

/// The case the whole guard exists for: one note in an unknown language must not
/// cost the other 499.
#[test]
fn a_note_in_an_unknown_language_arrives_without_taking_the_file_down() {
    let mut source = library();
    let file = exported(&mut source);

    let mut target = open_in_memory().unwrap();
    let report = merge(
        &mut target,
        written_by_a_newer_version(&file, "language", "from-the-future"),
    );

    assert_eq!(report.notes_imported, 2);
    assert_eq!(report.notes_degraded, 1);

    let arrived = notes::all(&mut target, None).unwrap();
    assert_eq!(arrived.len(), 2);
    assert!(
        arrived
            .iter()
            .any(|note| note.language == Language::default())
    );
}

#[test]
fn a_note_of_an_unknown_kind_is_degraded_the_same_way() {
    let mut source = library();
    let file = exported(&mut source);

    let mut target = open_in_memory().unwrap();
    let report = merge(
        &mut target,
        written_by_a_newer_version(&file, "kind", "from-the-future"),
    );

    assert_eq!(report.notes_imported, 2);
    assert_eq!(report.notes_degraded, 1);
    assert!(
        notes::all(&mut target, None)
            .unwrap()
            .iter()
            .all(|note| note.kind == NoteKind::default())
    );
}

/// The count follows what came in, not what the file carried: the second import
/// adds nothing, so it has nothing to report as degraded either.
#[test]
fn a_degraded_note_is_counted_once_and_not_again_on_a_second_import() {
    let mut source = library();
    let file = exported(&mut source);

    let mut target = open_in_memory().unwrap();
    merge(
        &mut target,
        written_by_a_newer_version(&file, "language", "from-the-future"),
    );

    let second = merge(
        &mut target,
        written_by_a_newer_version(&file, "language", "from-the-future"),
    );

    assert_eq!(second.notes_imported, 0);
    assert_eq!(second.notes_degraded, 0);
}

/// The attachment as it really sits beside the database: sealed under the library key,
/// which is what the export has to open before it can write the file out.
fn seal_beside(directory: &std::path::Path, library: &Library, record: &Attachment, bytes: &[u8]) {
    let sealed = library.vault().seal_bytes(bytes).unwrap();
    std::fs::write(directory.join(record.stored_name()), sealed).unwrap();
}

/// `create` takes the pair, the way an import inside a transaction does.
fn attach(
    library: &mut Library,
    record: &Attachment,
) -> Result<(), devnotes_lib::error::StorageError> {
    let (db, vault) = library.split();
    attachments::create(db, vault, record)
}

fn capture(note_id: &str) -> Attachment {
    Attachment {
        id: format!("a-{note_id}"),
        note_id: note_id.to_string(),
        file_name: "capture.png".to_string(),
        mime_type: "image/png".to_string(),
        byte_size: 4,
        created_at: t0(),
    }
}

/// The point of the archive. A screenshot is a file beside the database, so an export
/// that carried only the rows handed over notes whose thumbnails would never load — and
/// nothing said so.
#[test]
fn an_attachment_travels_with_the_library() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path().to_path_buf();
    let source_files = directory.join("source");
    let target_files = directory.join("target");
    std::fs::create_dir_all(&source_files).unwrap();
    std::fs::create_dir_all(&target_files).unwrap();

    let mut source = library();
    let note_id = notes::all(&mut source, None).unwrap()[0].id.clone();
    let record = capture(&note_id);
    seal_beside(&source_files, &source, &record, b"\x89PNG");
    attach(&mut source, &record).unwrap();

    let target_path = directory
        .join("library.devnotes")
        .to_string_lossy()
        .to_string();
    let packed = exported(&mut source);
    let written = file::write(&target_path, &packed, &source_files, source.vault(), None).unwrap();
    assert_eq!(written.attachments, 1);

    let mut target = open_in_memory().unwrap();
    let (incoming, mut payload) = file::read(&target_path, None).unwrap();
    let report = merge_bundle(&mut target, incoming, &mut payload, &target_files).unwrap();

    assert_eq!(report.attachments_imported, 1);
    assert_eq!(report.attachments_missing, 0);
    let landed = attachments::list(&mut target, &note_id).unwrap();
    assert_eq!(landed.len(), 1);
    // Not `record.stored_name()`: the id is remapped on the way in, because it decides a
    // write path and came out of a file. The bytes are what has to survive.
    assert_eq!(
        std::fs::read(target_files.join(landed[0].stored_name())).unwrap(),
        b"\x89PNG"
    );
    assert_ne!(landed[0].id, record.id);
}

/// Re-importing the same archive adds nothing, attachments included: the notes are
/// skipped, so their files have nowhere to land twice.
#[test]
fn importing_the_same_archive_twice_restores_the_attachment_once() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path().to_path_buf();
    let files = directory.join("files");
    std::fs::create_dir_all(&files).unwrap();

    let mut source = library();
    let note_id = notes::all(&mut source, None).unwrap()[0].id.clone();
    let record = capture(&note_id);
    seal_beside(&files, &source, &record, b"\x89PNG");
    attach(&mut source, &record).unwrap();

    let target_path = directory
        .join("library.devnotes")
        .to_string_lossy()
        .to_string();
    file::write(
        &target_path,
        &exported(&mut source),
        &files,
        source.vault(),
        None,
    )
    .unwrap();

    let mut target = open_in_memory().unwrap();
    let (first, mut payload) = file::read(&target_path, None).unwrap();
    merge_bundle(&mut target, first, &mut payload, &files).unwrap();

    let (again, mut payload) = file::read(&target_path, None).unwrap();
    let second = merge_bundle(&mut target, again, &mut payload, &files).unwrap();

    assert_eq!(second.notes_imported, 0);
    assert_eq!(second.attachments_imported, 0);
    assert_eq!(attachments::list(&mut target, &note_id).unwrap().len(), 1);
}

/// A record whose bytes the archive does not carry is counted, never swallowed: the
/// note arrives with a preview that will stay empty, and the report is what explains it.
#[test]
fn an_attachment_the_archive_does_not_carry_is_reported() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path().to_path_buf();
    let files = directory.join("files");
    std::fs::create_dir_all(&files).unwrap();

    let mut source = library();
    let note_id = notes::all(&mut source, None).unwrap()[0].id.clone();
    // The record exists, its file never did: the export writes the row and no entry.
    attach(&mut source, &capture(&note_id)).unwrap();

    let target_path = directory
        .join("library.devnotes")
        .to_string_lossy()
        .to_string();
    let written = file::write(
        &target_path,
        &exported(&mut source),
        &files,
        source.vault(),
        None,
    )
    .unwrap();
    assert_eq!(written.attachments, 0);

    let mut target = open_in_memory().unwrap();
    let (incoming, mut payload) = file::read(&target_path, None).unwrap();
    let report = merge_bundle(&mut target, incoming, &mut payload, &files).unwrap();

    assert_eq!(report.notes_imported, 2);
    assert_eq!(report.attachments_missing, 1);
    assert_eq!(report.attachments_imported, 0);
}

mod folders_travelling {
    use super::*;

    use devnotes_lib::folders::board::{BoardFrame, BoardPoint, CardPlacement, ZonePlacement};
    use devnotes_lib::folders::store as folders;
    use devnotes_lib::folders::store::board as geometry;

    /// A library with its notes filed, and a board arranged — the state a real one is in.
    fn arranged() -> (Library, String, String) {
        let mut connection = open_in_memory().unwrap();
        let sql = spaces::create(&mut connection, "SQL").unwrap().id;
        let perf = folders::create(&mut connection, &sql, "Perf", t0()).unwrap();
        let migrations = folders::create(&mut connection, &sql, "Migrations", t0()).unwrap();

        let filed = notes::create(&mut connection, draft(&sql, "EXPLAIN lent"), t0()).unwrap();
        let elsewhere = notes::create(&mut connection, draft(&sql, "Backfill"), t0()).unwrap();
        notes::create(&mut connection, draft(&sql, "Dump nocturne"), t0()).unwrap();

        folders::file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&perf.id),
            t0(),
        )
        .unwrap();
        folders::file_many(
            &mut connection,
            std::slice::from_ref(&elsewhere.id),
            Some(&migrations.id),
            t0(),
        )
        .unwrap();

        geometry::save_layout(
            &mut connection,
            &[ZonePlacement {
                folder_id: perf.id.clone(),
                frame: BoardFrame {
                    x: 900,
                    y: 700,
                    width: 520,
                    height: 400,
                },
            }],
            &[],
        )
        .unwrap();

        (connection, sql, perf.id)
    }

    fn folder_names(connection: &mut Library, space_id: &str) -> Vec<String> {
        let mut names: Vec<String> = folders::list(connection, Some(space_id))
            .unwrap()
            .into_iter()
            .map(|folder| folder.name)
            .collect();
        names.sort();
        names
    }

    fn folder_of(connection: &mut Library, title: &str) -> Option<String> {
        notes::all(connection, None)
            .unwrap()
            .into_iter()
            .find(|note| note.title == title)
            .and_then(|note| note.folder_id)
            .and_then(|id| {
                folders::list(connection, None)
                    .unwrap()
                    .into_iter()
                    .find(|folder| folder.id == id)
                    .map(|folder| folder.name)
            })
    }

    #[test]
    fn a_library_comes_back_arranged_on_another_machine() {
        let (mut source, _, _) = arranged();
        let bundle = round_tripped(&exported(&mut source));

        let mut target = open_in_memory().unwrap();
        let report = merge(&mut target, bundle);

        assert_eq!(report.notes_imported, 3);
        assert_eq!(report.spaces_created, 1);
        assert_eq!(report.folders_created, 2);

        let space = spaces::list(&mut target).unwrap()[0].id.clone();
        assert_eq!(folder_names(&mut target, &space), ["Migrations", "Perf"]);
        assert_eq!(
            folder_of(&mut target, "EXPLAIN lent").as_deref(),
            Some("Perf")
        );
        assert_eq!(
            folder_of(&mut target, "Backfill").as_deref(),
            Some("Migrations")
        );
        assert_eq!(folder_of(&mut target, "Dump nocturne"), None);
    }

    /// Exporting one folder's worth of notes must not recreate the whole tree.
    #[test]
    fn only_the_folders_actually_cited_travel() {
        let (mut source, space_id, perf_id) = arranged();
        let filed: Vec<_> = notes::all(&mut source, None)
            .unwrap()
            .into_iter()
            .filter(|note| note.folder_id.as_deref() == Some(perf_id.as_str()))
            .collect();

        let bundle = collect(&mut source, filed).unwrap();

        assert_eq!(bundle.folders.len(), 1);
        assert_eq!(bundle.folders[0].name, "Perf");
        assert_eq!(bundle.spaces.len(), 1);
        assert_eq!(bundle.spaces[0].id, space_id);
    }

    /// A board received from elsewhere must not land on top of the one you arranged.
    /// Keeping the geometry off the `Folder` model is what makes this free.
    #[test]
    fn no_coordinate_ever_leaves_the_machine() {
        let (mut source, _, _) = arranged();

        let json = serde_json::to_string(&exported(&mut source)).unwrap();

        for spelled in [
            "\"x\"",
            "\"y\"",
            "\"w\"",
            "\"h\"",
            "\"frame\"",
            "\"position\"",
        ] {
            assert!(!json.contains(spelled), "the export spells {spelled}");
        }
    }

    #[test]
    fn a_received_folder_is_laid_out_by_the_machine_that_receives_it() {
        let (mut source, _, _) = arranged();
        let bundle = round_tripped(&exported(&mut source));

        let mut target = open_in_memory().unwrap();
        merge(&mut target, bundle);

        let space = spaces::list(&mut target).unwrap()[0].id.clone();
        let placed = target
            .transaction(|connection, _vault| geometry::frames(connection, &space))
            .unwrap();

        // Nothing was written: the board lays it out on its first read, here as anywhere.
        assert!(placed.is_empty());
    }

    /// The rule spaces already follow, and for the same reason: merge, never replace.
    #[test]
    fn a_folder_of_the_same_name_is_reused_rather_than_duplicated() {
        let (mut source, _, _) = arranged();
        let bundle = round_tripped(&exported(&mut source));

        let mut target = open_in_memory().unwrap();
        let space = spaces::create(&mut target, "SQL").unwrap().id;
        folders::create(&mut target, &space, "perf", t0()).unwrap();

        let report = merge(&mut target, bundle);

        assert_eq!(report.spaces_created, 0);
        assert_eq!(report.folders_created, 1);
        assert_eq!(folder_names(&mut target, &space), ["Migrations", "perf"]);
        // Matched case-insensitively, so the note lands in the folder already there.
        assert_eq!(
            folder_of(&mut target, "EXPLAIN lent").as_deref(),
            Some("perf")
        );
    }

    /// Two spaces may both hold a "Perf": the match is per space, never per library.
    #[test]
    fn a_folder_is_matched_inside_its_own_space() {
        let (mut source, _, _) = arranged();
        let bundle = round_tripped(&exported(&mut source));

        let mut target = open_in_memory().unwrap();
        let elsewhere = spaces::create(&mut target, "Veille").unwrap().id;
        folders::create(&mut target, &elsewhere, "Perf", t0()).unwrap();

        let report = merge(&mut target, bundle);

        assert_eq!(report.folders_created, 2);
        assert_eq!(folder_names(&mut target, &elsewhere), ["Perf"]);
    }

    #[test]
    fn importing_the_same_file_twice_creates_no_folder_the_second_time() {
        let (mut source, _, _) = arranged();
        let bundle = exported(&mut source);

        let mut target = open_in_memory().unwrap();
        merge(&mut target, round_tripped(&bundle));
        let second = merge(&mut target, round_tripped(&bundle));

        assert_eq!(second.folders_created, 0);
        assert_eq!(second.notes_imported, 0);
        assert_eq!(second.notes_skipped, 3);
    }

    /// `#[serde(default)]`, exactly as `kind` and `items` carry it: no `FORMAT_VERSION`
    /// bump, because such a file still parses.
    #[test]
    fn an_export_written_before_folders_still_reads() {
        let (mut source, _, _) = arranged();
        let mut json: serde_json::Value = serde_json::to_value(exported(&mut source)).unwrap();
        json.as_object_mut().unwrap().remove("folders");
        for note in json["notes"].as_array_mut().unwrap() {
            note.as_object_mut().unwrap().remove("folderId");
        }

        let read = model::read_bundle(&json.to_string()).unwrap();
        let mut target = open_in_memory().unwrap();
        let report = merge(&mut target, read);

        assert_eq!(report.notes_imported, 3);
        assert_eq!(report.folders_created, 0);
        assert_eq!(folder_of(&mut target, "EXPLAIN lent"), None);
    }

    /// The id is the *sending* library's: a dangling one would be refused by the foreign
    /// key, losing the whole import over a note that is merely unfiled.
    #[test]
    fn a_note_naming_a_folder_the_file_left_out_arrives_unfiled() {
        let (mut source, _, _) = arranged();
        let mut json: serde_json::Value = serde_json::to_value(exported(&mut source)).unwrap();
        json["folders"] = serde_json::Value::Array(Vec::new());

        let read = model::read_bundle(&json.to_string()).unwrap();
        let mut target = open_in_memory().unwrap();
        let report = merge(&mut target, read);

        assert_eq!(report.notes_imported, 3);
        assert_eq!(report.folders_created, 0);
        assert_eq!(folder_of(&mut target, "EXPLAIN lent"), None);
    }

    #[test]
    fn the_export_report_counts_the_folders_it_wrote() {
        let (mut source, _, _) = arranged();

        assert_eq!(exported(&mut source).folders.len(), 2);
    }

    /// A position is a local gesture, and it is never asked to travel.
    #[test]
    fn a_card_position_stays_on_the_machine_that_chose_it() {
        let (mut source, space_id, _) = arranged();
        let loose = notes::all(&mut source, None)
            .unwrap()
            .into_iter()
            .find(|note| note.title == "Dump nocturne")
            .unwrap();
        geometry::save_layout(
            &mut source,
            &[],
            &[CardPlacement {
                note_id: loose.id.clone(),
                position: BoardPoint { x: 640, y: 480 },
            }],
        )
        .unwrap();

        let bundle = round_tripped(&exported(&mut source));
        let mut target = open_in_memory().unwrap();
        merge(&mut target, bundle);

        let space = spaces::list(&mut target).unwrap()[0].id.clone();
        let placed = target
            .transaction(|connection, _vault| geometry::positions(connection, &space))
            .unwrap();
        assert!(placed.is_empty());

        // And the sending machine keeps its own, untouched.
        let kept = source
            .transaction(|connection, _vault| geometry::positions(connection, &space_id))
            .unwrap();
        assert_eq!(kept.get(&loose.id), Some(&BoardPoint { x: 640, y: 480 }));
    }
}

/// A deliberate omission: revisions would inflate the bundle by a factor of the cap, and a
/// note arriving elsewhere without its history is the accepted cost.
#[test]
fn a_bundle_carries_no_history_of_the_bodies_it_holds() {
    let mut connection = open_in_memory().unwrap();
    let space_id = spaces::create(&mut connection, "Personal").unwrap().id;
    let id = notes::create(&mut connection, draft(&space_id, "Requête"), t0())
        .unwrap()
        .id;
    notes::update(
        &mut connection,
        &id,
        &devnotes_lib::notes::model::NotePatch {
            content: Some("select 2".to_string()),
            ..Default::default()
        },
        t0(),
    )
    .unwrap();

    let bundle = exported(&mut connection);
    let written = serde_json::to_string(&bundle).unwrap();

    // The current body travels; the one it replaced does not.
    assert!(written.contains("select 2"));
    assert!(!written.contains("select 1"));
}
