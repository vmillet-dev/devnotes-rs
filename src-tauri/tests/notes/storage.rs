use super::*;

#[test]
fn tags_are_normalized_on_write() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let created = create(
        &mut connection,
        tagged(&space_id, &["  #urgent ", "URGENT", "", " # ", "later"]),
        t0(),
    )
    .unwrap();

    assert_eq!(created.tags, ["later", "urgent"]);
    assert_eq!(list(&mut connection).unwrap()[0].tags, ["later", "urgent"]);
}

#[test]
fn a_normalized_write_returns_what_a_read_would_return() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let created = create(&mut connection, tagged(&space_id, &["zeta", "alpha"]), t0()).unwrap();

    assert_eq!(created.tags, list(&mut connection).unwrap()[0].tags);
}

#[test]
fn deleting_a_space_takes_its_notes_with_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    diesel::delete(spaces_table::table.find(&space_id))
        .execute(connection.db())
        .unwrap();

    assert!(list(&mut connection).unwrap().is_empty());
}

#[test]
fn a_stored_date_that_is_out_of_format_is_reported_rather_than_guessed() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    diesel::update(devnotes_lib::db::schema::notes::table.find(&created.id))
        .set(devnotes_lib::db::schema::notes::created_at.eq("pas une date"))
        .execute(connection.db())
        .unwrap();

    let error = list(&mut connection).unwrap_err();

    assert!(matches!(
        error,
        StorageError::CorruptRow {
            field: "createdAt",
            ..
        }
    ));
}

#[test]
fn a_stored_date_always_carries_its_milliseconds() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let round_second = at("2026-07-25T09:00:00Z");

    let created = create(&mut connection, draft(&space_id), round_second).unwrap();

    let stored: String = devnotes_lib::db::schema::notes::table
        .find(&created.id)
        .select(devnotes_lib::db::schema::notes::updated_at)
        .first(connection.db())
        .unwrap();

    assert_eq!(stored, "2026-07-25T09:00:00.000Z");
}

/// The point of the whole thing, and the only test that reads the file rather than the
/// API: a note written through the store must not be findable by grepping the database.
#[test]
fn a_note_is_not_readable_in_the_file_it_was_written_to() {
    use devnotes_lib::db;

    let scratch = tempfile::tempdir().unwrap();

    let directory = scratch.path().to_path_buf();
    let path = directory.join("sealed.sqlite3");

    let secret = "psql -h prod.internal -U admin -W hunter2";
    {
        let mut library = db::open(&path, db::test_vault().unwrap()).unwrap();
        let space = spaces::create(&mut library, "Secrets").unwrap().id;
        let mut seeded = draft(&space);
        seeded.title = "AWS prod credentials".to_string();
        seeded.content = secret.to_string();
        seeded.tags = vec!["prod".to_string()];
        create(&mut library, seeded, t0()).unwrap();

        // A variable for the whole corpus is where a host name or a token ends up, so it
        // is sealed like anything else a reader would want.
        let mut globals = std::collections::BTreeMap::new();
        globals.insert("host".to_string(), "prod.internal".to_string());
        devnotes_lib::notes::store::replace_global_placeholder_values(&mut library, &globals)
            .unwrap();
    }

    let raw = std::fs::read(&path).unwrap();
    let haystack = String::from_utf8_lossy(&raw);

    assert!(!haystack.contains(secret), "the body is in the clear");
    assert!(
        !haystack.contains("AWS prod credentials"),
        "the title is in the clear"
    );
    assert!(
        !haystack.contains("Secrets"),
        "the space name is in the clear"
    );
    assert!(
        !haystack.contains("prod.internal"),
        "a global variable's value is in the clear"
    );
    // Tags and the names of variables are deliberately not sealed: the facet, the
    // filter and the lookup all touch them in SQL. This asserts the decision rather than
    // an accident.
    assert!(
        haystack.contains("prod"),
        "a tag is expected to stay readable"
    );
    assert!(
        haystack.contains("host"),
        "a variable's name is expected to stay readable"
    );
}
