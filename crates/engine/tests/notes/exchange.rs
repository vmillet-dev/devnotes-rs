use super::*;

#[test]
fn an_export_reads_every_live_note_of_a_space() {
    let mut connection = open_in_memory().unwrap();
    let personal = space(&mut connection, "Personal");
    let boulot = space(&mut connection, "Boulot");
    create(&mut connection, draft(&personal), t0()).unwrap();
    let elsewhere = create(&mut connection, draft(&boulot), t0()).unwrap();
    let thrown = create(&mut connection, draft(&personal), t0()).unwrap();
    trash(&mut connection, &thrown.id, t1()).unwrap();

    let exported = all(&mut connection, Some(&personal)).unwrap();

    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].tags, ["api", "auth"]);
    assert_eq!(all(&mut connection, None).unwrap().len(), 2);
    assert!(
        all(&mut connection, None)
            .unwrap()
            .iter()
            .any(|note| note.id == elsewhere.id)
    );
}

#[test]
fn an_imported_note_keeps_its_identifier_and_its_dates() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let mut note = create(&mut connection, draft(&space_id), t0()).unwrap();
    note.id = "imported".to_string();

    assert!(insert_imported(&mut connection, &note).unwrap());

    let found = all(&mut connection, None)
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == "imported")
        .unwrap();
    assert_eq!(found.created_at, t0());
    assert_eq!(found.tags, ["api", "auth"]);
}

#[test]
fn importing_the_same_note_twice_leaves_the_first_one_alone() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let mut note = create(&mut connection, draft(&space_id), t0()).unwrap();
    note.title = "Écrasé ?".to_string();

    assert!(!insert_imported(&mut connection, &note).unwrap());
    assert_eq!(list(&mut connection).unwrap()[0].title, "Titre");
}

#[test]
fn sharing_a_selection_reads_the_notes_it_names() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let first = create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let selected = by_ids(&mut connection, std::slice::from_ref(&first.id)).unwrap();

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, first.id);
    assert!(by_ids(&mut connection, &[]).unwrap().is_empty());
}
