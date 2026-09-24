use super::*;

#[test]
fn a_checklist_is_read_back_in_the_order_it_was_written() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let created = create(
        &mut connection,
        checklist(
            &space_id,
            vec![item("Relire", true), item("Déployer", false)],
        ),
        t0(),
    )
    .unwrap();

    assert_eq!(created.kind, NoteKind::Checklist);
    assert_eq!(
        created.items,
        [item("Relire", true), item("Déployer", false)]
    );
    assert_eq!(list(&mut connection).unwrap()[0].items, created.items);
}

#[test]
fn patching_the_items_replaces_the_whole_list() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(
        &mut connection,
        checklist(
            &space_id,
            vec![item("a", false), item("b", false), item("c", false)],
        ),
        t0(),
    )
    .unwrap();

    let updated = update(
        &mut connection,
        &created.id,
        &NotePatch {
            items: Some(vec![item("c", true), item("a", false)]),
            ..NotePatch::default()
        },
        t1(),
    )
    .unwrap();

    assert_eq!(updated.items, [item("c", true), item("a", false)]);
    assert_eq!(list(&mut connection).unwrap()[0].items, updated.items);
}

#[test]
fn emptying_the_list_leaves_no_row_behind() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(
        &mut connection,
        checklist(&space_id, vec![item("a", false)]),
        t0(),
    )
    .unwrap();

    update(
        &mut connection,
        &created.id,
        &NotePatch {
            items: Some(Vec::new()),
            ..NotePatch::default()
        },
        t1(),
    )
    .unwrap();

    assert!(list(&mut connection).unwrap()[0].items.is_empty());
    assert_eq!(
        note_items::table
            .count()
            .get_result::<i64>(connection.db())
            .unwrap(),
        0
    );
}

#[test]
fn a_patch_that_says_nothing_about_the_items_leaves_them_stored() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(
        &mut connection,
        checklist(&space_id, vec![item("a", true)]),
        t0(),
    )
    .unwrap();

    let updated = update(
        &mut connection,
        &created.id,
        &NotePatch {
            title: Some("Sprint".to_string()),
            ..NotePatch::default()
        },
        t1(),
    )
    .unwrap();

    assert_eq!(updated.items, [item("a", true)]);
}

#[test]
fn purging_removes_the_note_and_its_items_for_good() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(
        &mut connection,
        checklist(&space_id, vec![item("a", false)]),
        t0(),
    )
    .unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    purge(&mut connection, std::slice::from_ref(&created.id)).unwrap();

    assert_eq!(
        note_items::table
            .count()
            .get_result::<i64>(connection.db())
            .unwrap(),
        0
    );
}
