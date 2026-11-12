use super::*;

#[test]
fn deleting_takes_the_note_off_the_canvas_without_destroying_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    trash(&mut connection, &created.id, t1()).unwrap();

    assert!(list(&mut connection).unwrap().is_empty());
    let kept_tags = note_tags::table
        .count()
        .get_result::<i64>(connection.db())
        .unwrap();
    assert_eq!(kept_tags, 2);

    let trashed = list_trashed(&mut connection).unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].0.id, created.id);
    assert_eq!(trashed[0].1, t1());
}

#[test]
fn a_restored_note_comes_back_whole() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    assert_eq!(
        restore_many(&mut connection, std::slice::from_ref(&created.id)).unwrap(),
        1
    );

    let listed = list(&mut connection).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].tags, ["api", "auth"]);
    assert_eq!(listed[0].updated_at, t0());
}

#[test]
fn a_trashed_note_is_no_longer_editable() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    let error = update(&mut connection, &created.id, &NotePatch::default(), t1()).unwrap_err();

    assert!(matches!(error, StorageError::NoteNotFound(_)));
}

#[test]
fn purging_removes_the_note_and_its_tags_for_good() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    assert_eq!(
        purge(&mut connection, std::slice::from_ref(&created.id)).unwrap(),
        1
    );

    assert!(list_trashed(&mut connection).unwrap().is_empty());
    let orphan_tags = note_tags::table
        .count()
        .get_result::<i64>(connection.db())
        .unwrap();
    assert_eq!(orphan_tags, 0);
}

#[test]
fn a_note_still_on_the_canvas_cannot_be_purged() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    assert_eq!(
        purge(&mut connection, std::slice::from_ref(&created.id)).unwrap(),
        0
    );
    assert_eq!(list(&mut connection).unwrap().len(), 1);
}

#[test]
fn only_notes_past_the_retention_are_reported_as_expired() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let old = create(&mut connection, draft(&space_id), t0()).unwrap();
    let recent = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &old.id, t0()).unwrap();
    trash(&mut connection, &recent.id, at("2026-08-20T09:00:00.000Z")).unwrap();

    let expired = expired_ids(&mut connection, at("2026-08-25T09:00:00.000Z")).unwrap();

    assert_eq!(expired, [old.id]);
}

#[test]
fn deleting_an_unknown_note_reports_an_error() {
    let mut connection = open_in_memory().unwrap();

    let error = trash(&mut connection, "unknown", t1()).unwrap_err();

    assert!(matches!(error, StorageError::NoteNotFound(_)));
}

#[test]
fn deleting_the_same_note_twice_reports_an_error_rather_than_a_silent_ok() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    let error = trash(&mut connection, &created.id, t1()).unwrap_err();

    assert!(matches!(error, StorageError::NoteNotFound(_)));
}
