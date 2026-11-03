use super::*;

/// Distinct, not summed: a note carrying two of the tags is one note, and a
/// confirmation that overstates its blast radius teaches people to dismiss it.
#[test]
fn the_blast_radius_counts_a_note_carrying_two_of_the_tags_once() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let counted =
        count_notes_tagged(&mut connection, &["api".to_string(), "auth".to_string()]).unwrap();

    assert_eq!(counted, 1);
}

/// A rename reaches the trash too, so the count that announces it has to.
#[test]
fn the_blast_radius_counts_what_is_in_the_trash() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &note.id, t1()).unwrap();

    assert_eq!(
        count_notes_tagged(&mut connection, &["auth".to_string()]).unwrap(),
        2
    );
}

#[test]
fn the_count_confirmed_is_the_count_renamed() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let thrown = create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &thrown.id, t1()).unwrap();
    let tags = ["api".to_string(), "auth".to_string()];

    let confirmed = count_notes_tagged(&mut connection, &tags).unwrap();

    assert_eq!(retag(&mut connection, &tags, "backend").unwrap(), confirmed);
}

#[test]
fn the_count_confirmed_is_the_count_deleted() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let thrown = create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &thrown.id, t1()).unwrap();
    let tags = ["api".to_string(), "auth".to_string()];

    let confirmed = count_notes_tagged(&mut connection, &tags).unwrap();

    assert_eq!(drop_tags(&mut connection, &tags).unwrap(), confirmed);
}

#[test]
fn a_note_restored_after_a_rename_comes_back_under_the_new_name() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let thrown = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &thrown.id, t1()).unwrap();

    retag(&mut connection, &["auth".to_string()], "identity").unwrap();
    restore_many(&mut connection, std::slice::from_ref(&thrown.id)).unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].tags, ["api", "identity"]);
}

#[test]
fn every_tag_is_listed_with_the_number_of_notes_carrying_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();

    assert_eq!(
        tag_usage(&mut connection).unwrap(),
        [("api".to_string(), 2), ("auth".to_string(), 2)]
    );
}

#[test]
fn a_trashed_note_no_longer_counts_towards_its_tags() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let kept = create(&mut connection, draft(&space_id), t0()).unwrap();
    let thrown = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &thrown.id, t1()).unwrap();

    let usage = tag_usage(&mut connection).unwrap();

    assert_eq!(usage[0], ("api".to_string(), 1));
    assert_eq!(list(&mut connection).unwrap()[0].id, kept.id);
}

#[test]
fn renaming_a_tag_onto_an_existing_one_merges_them() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();

    let touched = retag(&mut connection, &["api".to_string()], "auth").unwrap();

    assert_eq!(touched, 1);
    assert_eq!(list(&mut connection).unwrap()[0].tags, ["auth"]);
    assert_eq!(note.tags.len(), 2);
}

#[test]
fn merging_several_tags_keeps_one_note_entry_each() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    retag(
        &mut connection,
        &["api".to_string(), "auth".to_string()],
        "backend",
    )
    .unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].tags, ["backend"]);
}

#[test]
fn correcting_the_case_of_a_tag_does_not_erase_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    retag(&mut connection, &["auth".to_string()], "Auth").unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].tags, ["api", "Auth"]);
}

#[test]
fn dropping_a_tag_leaves_the_notes_in_place() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    assert_eq!(
        drop_tags(&mut connection, &["auth".to_string()]).unwrap(),
        1
    );

    let listed = list(&mut connection).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].tags, ["api"]);
}

#[test]
fn a_global_retag_does_not_float_the_corpus_to_the_top_of_the_canvas() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    retag(&mut connection, &["auth".to_string()], "identity").unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].updated_at, t0());
}
