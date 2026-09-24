use super::*;

#[test]
fn moving_a_selection_leaves_the_notes_already_there_untouched() {
    let mut connection = open_in_memory().unwrap();
    let source = space(&mut connection, "Personal");
    let target = space(&mut connection, "Boulot");
    let moved = create(&mut connection, draft(&source), t0()).unwrap();
    let settled = create(&mut connection, draft(&target), t0()).unwrap();

    let previous = move_many(
        &mut connection,
        &[moved.id.clone(), settled.id.clone()],
        &target,
        t1(),
    )
    .unwrap();

    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].note_id, moved.id);
    assert_eq!(previous[0].space_id, source);
    let listed = list(&mut connection).unwrap();
    assert!(listed.iter().all(|note| note.space_id == target));
    let untouched = listed.iter().find(|note| note.id == settled.id).unwrap();
    assert_eq!(untouched.updated_at, t0());
}

#[test]
fn moving_to_an_unknown_space_is_refused_for_the_whole_batch() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();

    let error = move_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        "ghost",
        t1(),
    )
    .unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
    assert_eq!(list(&mut connection).unwrap()[0].space_id, space_id);
}

#[test]
fn tagging_a_selection_adds_without_replacing() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();

    let added = tag_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        &["urgent".to_string()],
        t1(),
    )
    .unwrap();

    assert_eq!(added.len(), 1);
    assert_eq!(added[0].note_id, note.id);
    assert_eq!(added[0].tag, "urgent");
    assert_eq!(
        list(&mut connection).unwrap()[0].tags,
        ["api", "auth", "urgent"]
    );
}

/// The pair list is what the undo strips again, so a tag the note already carried
/// must not appear in it — and nothing changed, so `updated_at` stays put.
#[test]
fn tagging_twice_does_not_duplicate_the_tag_nor_claim_to_have_added_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();

    let added = tag_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        &["Auth".to_string()],
        t1(),
    )
    .unwrap();

    assert!(added.is_empty(), "{added:?}");
    let listed = list(&mut connection).unwrap();
    assert_eq!(listed[0].tags, ["api", "auth"]);
    assert_eq!(listed[0].updated_at, t0());
}

/// The whole point of answering pairs: undoing a batch that skipped a note must not take
/// the tag off that note.
#[test]
fn undoing_a_tagging_leaves_the_tag_on_the_note_that_already_carried_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let carried = create(&mut connection, draft(&space_id), t0()).unwrap();
    let plain = create(
        &mut connection,
        NoteDraft {
            tags: Vec::new(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();

    let added = tag_many(
        &mut connection,
        &[carried.id.clone(), plain.id.clone()],
        &["auth".to_string()],
        t1(),
    )
    .unwrap();
    untag_many(&mut connection, &added).unwrap();

    let listed = list(&mut connection).unwrap();
    let kept = listed.iter().find(|note| note.id == carried.id).unwrap();
    let stripped = listed.iter().find(|note| note.id == plain.id).unwrap();

    assert_eq!(kept.tags, ["api", "auth"]);
    assert!(stripped.tags.is_empty(), "{:?}", stripped.tags);
}

#[test]
fn a_move_goes_back_to_the_space_each_note_left() {
    let mut connection = open_in_memory().unwrap();
    let personal = space(&mut connection, "Personal");
    let work = space(&mut connection, "Boulot");
    let target = space(&mut connection, "Archive");
    let from_personal = create(&mut connection, draft(&personal), t0()).unwrap();
    let from_work = create(&mut connection, draft(&work), t0()).unwrap();

    let previous = move_many(
        &mut connection,
        &[from_personal.id.clone(), from_work.id.clone()],
        &target,
        t1(),
    )
    .unwrap();
    let restored = restore_placements(&mut connection, &previous).unwrap();

    assert_eq!(restored, 2);
    let listed = list(&mut connection).unwrap();
    let one = listed
        .iter()
        .find(|note| note.id == from_personal.id)
        .unwrap();
    let other = listed.iter().find(|note| note.id == from_work.id).unwrap();

    assert_eq!(one.space_id, personal);
    assert_eq!(other.space_id, work);
}

/// Like restoring from the trash: undoing is not editing, and the canvas sorts on it.
#[test]
fn putting_a_move_back_does_not_refresh_updated_at() {
    let mut connection = open_in_memory().unwrap();
    let source = space(&mut connection, "Personal");
    let target = space(&mut connection, "Boulot");
    let note = create(&mut connection, draft(&source), t0()).unwrap();

    let previous = move_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        &target,
        t1(),
    )
    .unwrap();
    restore_placements(&mut connection, &previous).unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].updated_at, t1());
}

/// A space dropped between the move and the undo takes its notes with it; the rest of
/// the batch still goes home.
#[test]
fn a_space_gone_since_the_move_does_not_take_the_rest_of_the_undo_with_it() {
    let mut connection = open_in_memory().unwrap();
    let doomed = space(&mut connection, "Personal");
    let kept = space(&mut connection, "Boulot");
    let target = space(&mut connection, "Archive");
    let orphan = create(&mut connection, draft(&doomed), t0()).unwrap();
    let survivor = create(&mut connection, draft(&kept), t0()).unwrap();

    let previous = move_many(
        &mut connection,
        &[orphan.id.clone(), survivor.id.clone()],
        &target,
        t1(),
    )
    .unwrap();
    spaces::delete(&mut connection, &doomed, &target).unwrap();

    let restored = restore_placements(&mut connection, &previous).unwrap();

    assert_eq!(restored, 1);
    let listed = list(&mut connection).unwrap();
    assert_eq!(
        listed
            .iter()
            .find(|note| note.id == survivor.id)
            .unwrap()
            .space_id,
        kept
    );
}
