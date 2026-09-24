use super::*;

#[test]
fn a_created_note_is_read_back_whole() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let created = create(&mut connection, draft(&space_id), t0()).unwrap();
    let listed = list(&mut connection).unwrap();

    assert_eq!(listed.len(), 1);
    let note = &listed[0];
    assert_eq!(note.id, created.id);
    assert_eq!(note.space_id, space_id);
    assert_eq!(note.title, "Titre");
    assert_eq!(note.content, "Contenu");
    assert_eq!(note.source, "API Gateway / Auth");
    assert!(!note.pinned);
    assert_eq!(note.tags, ["api", "auth"]);
}

#[test]
fn creation_stamps_both_dates_identically() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    assert_eq!(created.created_at, t0());
    assert_eq!(created.updated_at, t0());
}

#[test]
fn an_expiring_lifecycle_survives_a_round_trip() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let expiring = NoteDraft {
        lifecycle: NoteLifecycle::Expires {
            at: at("2026-08-01T00:00:00.000Z"),
        },
        ..draft(&space_id)
    };

    create(&mut connection, expiring, t0()).unwrap();

    let deadline = at("2026-08-01T00:00:00.000Z");
    let listed = list(&mut connection).unwrap();
    assert!(matches!(
        &listed[0].lifecycle,
        NoteLifecycle::Expires { at } if *at == deadline
    ));
}

#[test]
fn dropping_an_expiry_clears_the_stored_date() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let expiring = NoteDraft {
        lifecycle: NoteLifecycle::Expires {
            at: at("2026-08-01T00:00:00.000Z"),
        },
        ..draft(&space_id)
    };
    let created = create(&mut connection, expiring, t0()).unwrap();

    let patch = NotePatch {
        lifecycle: Some(NoteLifecycle::Permanent),
        ..NotePatch::default()
    };
    update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert!(matches!(
        list(&mut connection).unwrap()[0].lifecycle,
        NoteLifecycle::Permanent
    ));
}

/// A newer build writes a language this one cannot name, and this one reads it as the
/// fallback: an edit must not write that fallback back.
#[test]
fn an_edit_keeps_a_language_this_build_cannot_read() {
    let mut connection = open_in_memory().unwrap();
    let space = spaces::create(&mut connection, "Perso").unwrap().id;
    let created = create(&mut connection, draft(&space), t0()).unwrap();

    // Written the way a newer build would have written it.
    diesel::update(notes_table::table.find(&created.id))
        .set(notes_table::language.eq("rust"))
        .execute(connection.db())
        .unwrap();

    // The read degrades, which is deliberate and unchanged.
    assert_eq!(list(&mut connection).unwrap()[0].language, Language::Txt);

    let patch = NotePatch {
        title: Some("Nouveau titre".to_string()),
        ..NotePatch::default()
    };
    update(&mut connection, &created.id, &patch, t1()).unwrap();

    let stored: String = notes_table::table
        .find(&created.id)
        .select(notes_table::language)
        .first(connection.db())
        .unwrap();
    assert_eq!(
        stored, "rust",
        "the edit overwrote a language it could not read"
    );
}

/// A column the patch never named is left alone, which is what the guarantee above
/// rests on — and it costs nine columns less per title change.
#[test]
fn an_edit_writes_only_what_it_moved() {
    let mut connection = open_in_memory().unwrap();
    let space = spaces::create(&mut connection, "Perso").unwrap().id;
    let created = create(&mut connection, draft(&space), t0()).unwrap();

    // A value no patch will touch, and that this build would happily rewrite.
    diesel::update(notes_table::table.find(&created.id))
        .set(notes_table::kind.eq("runbook"))
        .execute(connection.db())
        .unwrap();

    let patch = NotePatch {
        pinned: Some(true),
        ..NotePatch::default()
    };
    update(&mut connection, &created.id, &patch, t1()).unwrap();

    let (kind, pinned): (String, bool) = notes_table::table
        .find(&created.id)
        .select((notes_table::kind, notes_table::pinned))
        .first(connection.db())
        .unwrap();
    assert_eq!(kind, "runbook");
    assert!(pinned);
}

#[test]
fn creating_in_an_unknown_space_is_refused() {
    let mut connection = open_in_memory().unwrap();

    let error = create(&mut connection, draft("unknown"), t0()).unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
    assert!(list(&mut connection).unwrap().is_empty());
}

#[test]
fn notes_are_listed_most_recently_updated_first() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");

    let older = create(&mut connection, draft(&space_id), t0()).unwrap();
    let newer = create(&mut connection, draft(&space_id), t1()).unwrap();

    let ids: Vec<String> = list(&mut connection)
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();

    assert_eq!(ids, [newer.id, older.id]);
}

#[test]
fn an_absent_patch_field_leaves_the_stored_value_untouched() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let patch = NotePatch {
        title: Some("Nouveau titre".to_string()),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(updated.title, "Nouveau titre");
    assert_eq!(updated.content, "Contenu");
    assert_eq!(updated.source, "API Gateway / Auth");
    assert_eq!(updated.tags, ["api", "auth"]);
    assert!(matches!(updated.lifecycle, NoteLifecycle::Permanent));
}

#[test]
fn updating_refreshes_updated_at_but_not_created_at() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let updated = update(&mut connection, &created.id, &NotePatch::default(), t1()).unwrap();

    assert_eq!(updated.created_at, t0());
    assert_eq!(updated.updated_at, t1());
}

#[test]
fn typing_into_a_freshly_created_note_keeps_it_text() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let empty = NoteDraft {
        language: Language::Txt,
        content: String::new(),
        ..draft(&space_id)
    };
    let created = create(&mut connection, empty, t0()).unwrap();

    let patch = NotePatch {
        content: Some("## Standup\n\ninterface Note { id: string } is the model".to_string()),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(updated.language, Language::Txt);
}

/// A paste of code is detected by the front end, through `detect_language`, before it writes.
#[test]
fn a_detected_paste_lands_with_its_language() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let empty = NoteDraft {
        language: Language::Txt,
        content: String::new(),
        ..draft(&space_id)
    };
    let created = create(&mut connection, empty, t0()).unwrap();

    let patch = NotePatch {
        content: Some("interface Note { id: string }".to_string()),
        language: Some(Language::Ts),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(updated.language, Language::Ts);
    assert_eq!(list(&mut connection).unwrap()[0].language, Language::Ts);
}

#[test]
fn a_later_edit_does_not_move_the_language_again() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let empty = NoteDraft {
        language: Language::Txt,
        content: String::new(),
        ..draft(&space_id)
    };
    let created = create(&mut connection, empty, t0()).unwrap();

    let first = NotePatch {
        content: Some("SELECT 1".to_string()),
        language: Some(Language::Sql),
        ..NotePatch::default()
    };
    update(&mut connection, &created.id, &first, t1()).unwrap();

    let second = NotePatch {
        content: Some("interface Note { id: string }".to_string()),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &second, t1()).unwrap();

    assert_eq!(updated.language, Language::Sql);
}

#[test]
fn patching_tags_replaces_the_whole_set() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let patch = NotePatch {
        tags: Some(vec!["sql".to_string()]),
        ..NotePatch::default()
    };
    update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(list(&mut connection).unwrap()[0].tags, ["sql"]);
}

#[test]
fn mixed_case_tags_come_back_in_the_same_order_a_reload_gives() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let patch = NotePatch {
        tags: Some(vec!["Urgent".to_string(), "auth".to_string()]),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(updated.tags, ["auth", "Urgent"]);
    assert_eq!(list(&mut connection).unwrap()[0].tags, updated.tags);
}

#[test]
fn patching_tags_to_an_empty_list_clears_them() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let patch = NotePatch {
        tags: Some(Vec::new()),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert!(updated.tags.is_empty());
    assert!(list(&mut connection).unwrap()[0].tags.is_empty());
}

#[test]
fn a_note_can_be_moved_to_another_space() {
    let mut connection = open_in_memory().unwrap();
    let origin = space(&mut connection, "Personal");
    let destination = space(&mut connection, "Boulot");
    let created = create(&mut connection, draft(&origin), t0()).unwrap();

    let patch = NotePatch {
        space_id: Some(destination.clone()),
        ..NotePatch::default()
    };
    let updated = update(&mut connection, &created.id, &patch, t1()).unwrap();

    assert_eq!(updated.space_id, destination);
}

#[test]
fn moving_a_note_to_an_unknown_space_is_refused_and_changes_nothing() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let created = create(&mut connection, draft(&space_id), t0()).unwrap();

    let patch = NotePatch {
        space_id: Some("unknown".to_string()),
        title: Some("Ne doit pas passer".to_string()),
        ..NotePatch::default()
    };
    let error = update(&mut connection, &created.id, &patch, t1()).unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
    let note = &list(&mut connection).unwrap()[0];
    assert_eq!(note.space_id, space_id);
    assert_eq!(note.title, "Titre");
}

#[test]
fn updating_an_unknown_note_reports_an_error() {
    let mut connection = open_in_memory().unwrap();

    let error = update(&mut connection, "unknown", &NotePatch::default(), t1()).unwrap_err();

    assert!(matches!(error, StorageError::NoteNotFound(_)));
}
