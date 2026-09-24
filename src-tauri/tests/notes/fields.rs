use super::*;

#[test]
fn a_filled_field_is_read_back_on_the_next_opening() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();

    set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")])).unwrap();

    let listed = list(&mut connection).unwrap();
    assert_eq!(listed[0].placeholder_values, values(&[("host", "db")]));
}

#[test]
fn filling_a_field_does_not_touch_updated_at() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();

    let saved =
        set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")])).unwrap();

    assert_eq!(saved.updated_at, t0());
    assert_eq!(list(&mut connection).unwrap()[0].updated_at, t0());
}

#[test]
fn writing_the_values_replaces_the_whole_set() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();
    set_placeholder_values(
        &mut connection,
        &created.id,
        &values(&[("host", "db"), ("port", "6543")]),
    )
    .unwrap();

    set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")])).unwrap();

    assert_eq!(
        list(&mut connection).unwrap()[0].placeholder_values,
        values(&[("host", "db")])
    );
}

#[test]
fn a_value_whose_token_left_the_content_stays_stored_but_out_of_the_fields() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();
    set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")])).unwrap();

    let renamed = update(
        &mut connection,
        &created.id,
        &NotePatch {
            content: Some("psql -h {{hostname}}".to_string()),
            ..NotePatch::default()
        },
        t1(),
    )
    .unwrap();

    assert_eq!(renamed.placeholder_values, values(&[("host", "db")]));
    let fields = decorate(renamed, t1()).placeholders;
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "hostname");
    assert!(fields[0].value.is_empty());
}

#[test]
fn filling_a_trashed_note_is_refused() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    let error = set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")]))
        .unwrap_err();

    assert!(matches!(error, StorageError::NoteNotFound(_)));
}

#[test]
fn purging_removes_the_note_and_its_values_for_good() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();
    set_placeholder_values(&mut connection, &created.id, &values(&[("host", "db")])).unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    purge(&mut connection, std::slice::from_ref(&created.id)).unwrap();

    assert_eq!(
        note_placeholders::table
            .count()
            .get_result::<i64>(connection.db())
            .unwrap(),
        0
    );
}

#[test]
fn global_variables_read_back_what_was_written() {
    let mut connection = open_in_memory().unwrap();

    replace_global_placeholder_values(&mut connection, &values(&[("host", "db.internal")]))
        .unwrap();

    assert_eq!(
        global_placeholder_values(&mut connection).unwrap(),
        values(&[("host", "db.internal")])
    );
}

#[test]
fn writing_the_set_again_drops_what_is_no_longer_sent() {
    let mut connection = open_in_memory().unwrap();
    replace_global_placeholder_values(
        &mut connection,
        &values(&[("host", "db.internal"), ("port", "5432")]),
    )
    .unwrap();

    replace_global_placeholder_values(&mut connection, &values(&[("port", "6543")])).unwrap();

    assert_eq!(
        global_placeholder_values(&mut connection).unwrap(),
        values(&[("port", "6543")])
    );
}

#[test]
fn a_global_variable_belongs_to_no_note_and_survives_a_purge() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();
    replace_global_placeholder_values(&mut connection, &values(&[("host", "db.internal")]))
        .unwrap();
    trash(&mut connection, &created.id, t1()).unwrap();

    purge(&mut connection, std::slice::from_ref(&created.id)).unwrap();

    assert_eq!(
        global_placeholder_values(&mut connection).unwrap(),
        values(&[("host", "db.internal")])
    );
}

#[test]
fn a_global_variable_shows_up_as_the_suggested_value_of_a_field() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, templated(&space_id), t0()).unwrap();

    replace_global_placeholder_values(&mut connection, &values(&[("host", "db.internal")]))
        .unwrap();
    let mut display = decorate(created, t1());
    devnotes_lib::notes::store::decorations(&mut connection)
        .unwrap()
        .apply([&mut display]);

    let field = display
        .placeholders
        .iter()
        .find(|placeholder| placeholder.name == "host")
        .unwrap();
    assert_eq!(field.default_value, "db.internal");
    assert!(field.value.is_empty());
}
