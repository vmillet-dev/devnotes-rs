use std::collections::BTreeMap;

use devnotes_lib::db::Library;
use diesel::prelude::*;

use chrono::{DateTime, Utc};

use devnotes_lib::db::iso8601;
use devnotes_lib::db::open_in_memory;
use devnotes_lib::db::schema::{
    note_items, note_placeholders, note_revisions, note_tags, notes as notes_table,
    spaces as spaces_table,
};
use devnotes_lib::error::StorageError;
use devnotes_lib::notes::checklist::{ChecklistItem, NoteKind};
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{Note, NoteDraft, NoteLifecycle, NotePatch, decorate};
use devnotes_lib::notes::store::trash::{expired_ids, list_trashed, purge, restore_many, trash};
use devnotes_lib::notes::store::{
    all, by_ids, count_notes_tagged, create, drop_tags, fetch, global_placeholder_values,
    insert_imported, move_many, replace_global_placeholder_values, restore_placements, retag, seed,
    set_placeholder_values, tag_many, tag_usage, untag_many, update,
};
use devnotes_lib::notes::view::{self, NoteFilter, NotesQuery, NotesView};
use devnotes_lib::spaces::store as spaces;

/// ⚠️ The first launch used to be six round trips — one space, one marker, four notes —
/// and a process killed between any two left a space standing with nothing in it, which
/// both of the front end’s guards then read as "already seeded".
#[test]
fn the_first_launch_writes_the_space_and_its_notes_as_one() {
    let mut library = open_in_memory().unwrap();

    let space = seed(
        &mut library,
        "Personnel",
        &[],
        vec![
            sample(
                None,
                NoteDraft {
                    title: "Bienvenue".to_string(),
                    ..draft("ignored")
                },
            ),
            sample(
                None,
                NoteDraft {
                    title: "Un snippet".to_string(),
                    ..draft("ignored")
                },
            ),
        ],
        t0(),
    )
    .unwrap();

    assert_eq!(space.name, "Personnel");
    let written = list(&mut library).unwrap();
    assert_eq!(written.len(), 2);
    // ⚠️ The drafts carry a space that does not exist when the front end composes them.
    assert!(written.iter().all(|note| note.space_id == space.id));
}

/// ⚠️ The folders belong to the same transaction. A seeding that wrote the space and the
/// notes but not the folders would be permanent: a space exists, so both of the front
/// end's guards read "already seeded" and it never runs again.
#[test]
fn the_first_launch_writes_its_folders_with_everything_else() {
    let mut library = open_in_memory().unwrap();

    let space = seed(
        &mut library,
        "Découverte",
        &["Snippets".to_string(), "Prise en main".to_string()],
        vec![
            sample(
                Some(0),
                NoteDraft {
                    title: "Connexion psql".to_string(),
                    ..draft("ignored")
                },
            ),
            sample(
                Some(1),
                NoteDraft {
                    title: "Bienvenue".to_string(),
                    ..draft("ignored")
                },
            ),
            sample(
                None,
                NoteDraft {
                    title: "À essayer".to_string(),
                    ..draft("ignored")
                },
            ),
        ],
        t0(),
    )
    .unwrap();

    let folders = devnotes_lib::folders::store::list(&mut library, Some(&space.id)).unwrap();
    assert_eq!(
        folders.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["Snippets", "Prise en main"]
    );
    // Assigned by rotation, so the two zones are told apart on the board at a glance.
    assert_ne!(folders[0].colour, folders[1].colour);

    let written = list(&mut library).unwrap();
    let filed = |title: &str| {
        written
            .iter()
            .find(|note| note.title == title)
            .and_then(|note| note.folder_id.clone())
    };
    assert_eq!(filed("Connexion psql"), Some(folders[0].id.clone()));
    assert_eq!(filed("Bienvenue"), Some(folders[1].id.clone()));
    // ⚠️ One left loose on purpose: "no folder" is a legitimate state, and the first
    // launch shows it rather than describing it.
    assert_eq!(filed("À essayer"), None);
}

/// ⚠️ Every seeded row used to share one instant, so both orders fell back to a random
/// UUID: the zones and the sample cards came out arranged differently on each install.
#[test]
fn the_samples_are_read_back_in_the_order_they_were_declared() {
    let mut library = open_in_memory().unwrap();

    let space = seed(
        &mut library,
        "Découverte",
        &["Un".to_string(), "Deux".to_string(), "Trois".to_string()],
        vec![
            sample(
                None,
                NoteDraft {
                    title: "Premier".to_string(),
                    ..draft("ignored")
                },
            ),
            sample(
                None,
                NoteDraft {
                    title: "Deuxième".to_string(),
                    ..draft("ignored")
                },
            ),
            sample(
                None,
                NoteDraft {
                    title: "Troisième".to_string(),
                    ..draft("ignored")
                },
            ),
        ],
        t0(),
    )
    .unwrap();

    let folders = devnotes_lib::folders::store::list(&mut library, Some(&space.id)).unwrap();
    assert_eq!(
        folders.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["Un", "Deux", "Trois"]
    );

    // `fetch` is what the canvas asks, ordered as the canvas orders it.
    assert_eq!(
        list(&mut library)
            .unwrap()
            .iter()
            .map(|note| note.title.as_str())
            .collect::<Vec<_>>(),
        ["Premier", "Deuxième", "Troisième"]
    );
}

/// A folder index nothing answers leaves the note loose rather than failing the launch.
#[test]
fn a_sample_naming_a_folder_that_is_not_there_is_simply_left_loose() {
    let mut library = open_in_memory().unwrap();

    seed(
        &mut library,
        "Découverte",
        &["Snippets".to_string()],
        vec![sample(
            Some(7),
            NoteDraft {
                title: "Égarée".to_string(),
                ..draft("ignored")
            },
        )],
        t0(),
    )
    .unwrap();

    assert_eq!(list(&mut library).unwrap()[0].folder_id, None);
}

/// A refused seeding leaves nothing behind — not even the space it had started with.
#[test]
fn a_seeding_that_fails_leaves_no_half_library() {
    let mut library = open_in_memory().unwrap();
    spaces::create(&mut library, "Personnel").unwrap();
    let before = spaces::list(&mut library).unwrap().len();

    // The name is taken, so the space this seeding opens with is refused.
    let refused = seed(
        &mut library,
        "personnel",
        &["Snippets".to_string()],
        vec![sample(None, draft("ignored"))],
        t0(),
    );

    assert!(matches!(refused, Err(StorageError::DuplicateSpaceName(_))));
    assert_eq!(spaces::list(&mut library).unwrap().len(), before);
    assert!(list(&mut library).unwrap().is_empty());
    assert!(
        devnotes_lib::folders::store::list(&mut library, None)
            .unwrap()
            .is_empty()
    );
}

fn sample(folder: Option<u32>, draft: NoteDraft) -> devnotes_lib::notes::model::SampleNote {
    devnotes_lib::notes::model::SampleNote { folder, draft }
}

/// No such shortcut exists in production code: it would invite re-filtering on the
/// front end.
fn list(connection: &mut Library) -> Result<Vec<Note>, StorageError> {
    fetch(
        connection,
        &NotesQuery {
            space_id: None,
            folder_id: None,
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            now: t0(),
            tz_offset_minutes: 0,
            pinned_first: true,
        },
    )
    .map(|(notes, _)| notes)
}

fn at(iso: &str) -> DateTime<Utc> {
    iso8601::parse(iso).expect("tests write valid instants")
}

fn t0() -> DateTime<Utc> {
    at("2026-07-25T09:00:00.000Z")
}

fn t1() -> DateTime<Utc> {
    at("2026-07-25T10:00:00.000Z")
}

fn query(connection: &mut Library, request: &NotesQuery) -> Result<NotesView, StorageError> {
    let (notes, facets) = fetch(connection, request)?;
    Ok(view::build(notes, facets, request))
}

fn space(connection: &mut Library, name: &str) -> String {
    spaces::create(connection, name).unwrap().id
}

fn draft(space_id: &str) -> NoteDraft {
    NoteDraft {
        space_id: space_id.to_string(),
        folder_id: None,
        title: "Titre".to_string(),
        language: Language::Txt,
        content: "Contenu".to_string(),
        source: "API Gateway / Auth".to_string(),
        tags: vec!["auth".to_string(), "api".to_string()],
        pinned: false,
        lifecycle: NoteLifecycle::Permanent,
        kind: NoteKind::Snippet,
        items: Vec::new(),
    }
}

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

/// ⚠️ The scenario in full: a newer build writes a language this one cannot name, this
/// one reads it as the fallback — and writing every column back used to make that
/// fallback permanent, in the database, with no error anywhere along the way.
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
fn pasting_into_a_freshly_created_note_settles_its_language() {
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

fn item(text: &str, done: bool) -> ChecklistItem {
    ChecklistItem {
        text: text.to_string(),
        done,
    }
}

fn checklist(space_id: &str, items: Vec<ChecklistItem>) -> NoteDraft {
    NoteDraft {
        kind: NoteKind::Checklist,
        content: String::new(),
        items,
        ..draft(space_id)
    }
}

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

/// This section's reference snippet: two fields, one of them with a default.
fn templated(space_id: &str) -> NoteDraft {
    NoteDraft {
        content: "psql -h {{host}} -p {{port=5432}}".to_string(),
        ..draft(space_id)
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

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

    let globals = values(&[("host", "db.internal")]);
    let mut display = decorate(created, t1());
    devnotes_lib::notes::model::apply_global_defaults(&mut display, &globals);

    let field = display
        .placeholders
        .iter()
        .find(|placeholder| placeholder.name == "host")
        .unwrap();
    assert_eq!(field.default_value, "db.internal");
    assert!(field.value.is_empty());
}

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

/// ⚠️ The pair list is what the undo strips again, so a tag the note already carried
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

/// ⚠️ Like restoring from the trash: undoing is not editing, and the canvas sorts on it.
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

/// ⚠️ Distinct, not summed: a note carrying two of the tags is one note, and a
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

/// A trashed note is not part of any blast radius: it is not in the corpus any more.
#[test]
fn the_blast_radius_leaves_out_what_is_in_the_trash() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &note.id, t1()).unwrap();

    assert_eq!(
        count_notes_tagged(&mut connection, &["auth".to_string()]).unwrap(),
        1
    );
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

/// Neutral query: tests override one field at a time, so each states exactly what
/// it exercises.
fn all_notes() -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search: String::new(),
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: Vec::new(),
        now: t1(),
        tz_offset_minutes: 0,
        pinned_first: true,
    }
}

fn matched_ids(view: &NotesView) -> Vec<String> {
    view.sections
        .iter()
        .flat_map(|section| section.notes.iter().map(|note| note.id.clone()))
        .collect()
}

fn tagged(space_id: &str, tags: &[&str]) -> NoteDraft {
    NoteDraft {
        tags: tags.iter().copied().map(String::from).collect(),
        ..draft(space_id)
    }
}

fn written_in(space_id: &str, language: Language) -> NoteDraft {
    NoteDraft {
        language,
        ..draft(space_id)
    }
}

#[test]
fn a_query_without_criteria_returns_every_note() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(view.matched, 1);
    assert!(!view.is_filtering);
}

#[test]
fn the_space_filter_excludes_the_other_spaces() {
    let mut connection = open_in_memory().unwrap();
    let here = space(&mut connection, "Personal");
    let elsewhere = space(&mut connection, "Boulot");
    let kept = create(&mut connection, draft(&here), t0()).unwrap();
    create(&mut connection, draft(&elsewhere), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            space_id: Some(here),
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(matched_ids(&view), [kept.id]);
}

#[test]
fn no_space_means_every_space_rather_than_none() {
    let mut connection = open_in_memory().unwrap();
    let here = space(&mut connection, "Personal");
    let elsewhere = space(&mut connection, "Boulot");
    create(&mut connection, draft(&here), t0()).unwrap();
    create(&mut connection, draft(&elsewhere), t0()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(view.matched, 2);
}

#[test]
fn the_pinned_filter_keeps_only_pinned_notes() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let pinned = create(
        &mut connection,
        NoteDraft {
            pinned: true,
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            filter: NoteFilter::Pinned,
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(matched_ids(&view), [pinned.id]);
}

#[test]
fn the_untriaged_filter_keeps_only_expiring_notes() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let expiring = create(
        &mut connection,
        NoteDraft {
            lifecycle: NoteLifecycle::Expires {
                at: at("2026-08-01T00:00:00.000Z"),
            },
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            filter: NoteFilter::Untriaged,
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(matched_ids(&view), [expiring.id]);
}

#[test]
fn a_quick_filter_alone_does_not_switch_to_results_mode() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            filter: NoteFilter::Pinned,
            ..all_notes()
        },
    )
    .unwrap();

    assert!(!view.is_filtering);
}

#[test]
fn the_search_matches_the_title_the_content_and_the_tags() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let by_title = create(
        &mut connection,
        NoteDraft {
            title: "Script de deploiement".to_string(),
            content: String::new(),
            tags: Vec::new(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();
    let by_content = create(
        &mut connection,
        NoteDraft {
            title: String::new(),
            content: "kubectl rollout".to_string(),
            tags: Vec::new(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();
    let by_tag = create(&mut connection, tagged(&space_id, &["urgent"]), t0()).unwrap();

    for (needle, expected) in [
        ("deploiement", &by_title),
        ("rollout", &by_content),
        ("urgent", &by_tag),
    ] {
        let view = query(
            &mut connection,
            &NotesQuery {
                search: needle.to_string(),
                ..all_notes()
            },
        )
        .unwrap();
        assert_eq!(
            matched_ids(&view),
            [expected.id.as_str()],
            "needle: {needle}"
        );
    }
}

#[test]
fn the_search_ignores_case_beyond_ascii() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(
        &mut connection,
        NoteDraft {
            title: "Étape de migration".to_string(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            search: "étape".to_string(),
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.matched, 1);
}

#[test]
fn a_blank_search_is_not_a_search() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            search: "   ".to_string(),
            ..all_notes()
        },
    )
    .unwrap();

    assert!(!view.is_filtering);
    assert_eq!(view.matched, 1);
}

#[test]
fn a_note_matches_when_it_carries_at_least_one_selected_tag() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let one = create(&mut connection, tagged(&space_id, &["urgent"]), t0()).unwrap();
    let two = create(&mut connection, tagged(&space_id, &["later"]), t0()).unwrap();
    create(&mut connection, tagged(&space_id, &["neither"]), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            tags: vec!["urgent".to_string(), "later".to_string()],
            ..all_notes()
        },
    )
    .unwrap();

    let ids = matched_ids(&view);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&one.id) && ids.contains(&two.id));
    assert!(view.is_filtering);
}

#[test]
fn a_selected_tag_is_normalized_like_a_stored_one() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, tagged(&space_id, &["urgent"]), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            tags: vec![" #urgent ".to_string()],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.matched, 1);
}

#[test]
fn a_selected_tag_matches_a_stored_one_of_a_different_case() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, tagged(&space_id, &["Urgent"]), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            tags: vec!["urgent".to_string()],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.matched, 1);
}

#[test]
fn the_rail_offers_one_facet_for_tags_differing_only_in_case() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, tagged(&space_id, &["Urgent"]), t0()).unwrap();
    create(&mut connection, tagged(&space_id, &["urgent"]), t1()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(view.available_tags.len(), 1);
}

#[test]
fn criteria_combine_rather_than_replace_each_other() {
    let mut connection = open_in_memory().unwrap();
    let here = space(&mut connection, "Personal");
    let elsewhere = space(&mut connection, "Boulot");

    let target = create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            pinned: true,
            tags: vec!["urgent".to_string()],
            ..draft(&here)
        },
        t0(),
    )
    .unwrap();
    create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            pinned: true,
            tags: vec!["later".to_string()],
            ..draft(&here)
        },
        t0(),
    )
    .unwrap();
    create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            pinned: false,
            tags: vec!["urgent".to_string()],
            ..draft(&here)
        },
        t0(),
    )
    .unwrap();
    create(
        &mut connection,
        NoteDraft {
            title: "autre".to_string(),
            pinned: true,
            tags: vec!["urgent".to_string()],
            ..draft(&here)
        },
        t0(),
    )
    .unwrap();
    create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            pinned: true,
            tags: vec!["urgent".to_string()],
            ..draft(&elsewhere)
        },
        t0(),
    )
    .unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            space_id: Some(here),
            search: "deploy".to_string(),
            filter: NoteFilter::Pinned,
            tags: vec!["urgent".to_string()],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(matched_ids(&view), [target.id]);
}

#[test]
fn a_note_matches_when_it_is_written_in_one_of_the_selected_languages() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let json = create(&mut connection, written_in(&space_id, Language::Json), t0()).unwrap();
    let yml = create(&mut connection, written_in(&space_id, Language::Yml), t0()).unwrap();
    create(&mut connection, written_in(&space_id, Language::Py), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            languages: vec![Language::Json, Language::Yml],
            ..all_notes()
        },
    )
    .unwrap();

    let ids = matched_ids(&view);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&json.id) && ids.contains(&yml.id));
    assert!(view.is_filtering);
}

#[test]
fn the_language_filter_combines_with_the_other_criteria() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let target = create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            ..written_in(&space_id, Language::Yml)
        },
        t0(),
    )
    .unwrap();
    create(&mut connection, written_in(&space_id, Language::Yml), t0()).unwrap();
    create(
        &mut connection,
        NoteDraft {
            title: "deploy".to_string(),
            ..written_in(&space_id, Language::Json)
        },
        t0(),
    )
    .unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            search: "deploy".to_string(),
            languages: vec![Language::Yml],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(matched_ids(&view), [target.id]);
}

#[test]
fn available_languages_are_sorted_and_de_duplicated() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, written_in(&space_id, Language::Yml), t0()).unwrap();
    create(&mut connection, written_in(&space_id, Language::Json), t0()).unwrap();
    create(&mut connection, written_in(&space_id, Language::Json), t1()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(view.available_languages, [Language::Json, Language::Yml]);
}

#[test]
fn available_languages_are_scoped_to_the_active_space() {
    let mut connection = open_in_memory().unwrap();
    let here = space(&mut connection, "Personal");
    let elsewhere = space(&mut connection, "Boulot");
    create(&mut connection, written_in(&here, Language::Json), t0()).unwrap();
    create(&mut connection, written_in(&elsewhere, Language::Sql), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            space_id: Some(here),
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.available_languages, [Language::Json]);
}

#[test]
fn available_languages_ignore_the_current_selection() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, written_in(&space_id, Language::Json), t0()).unwrap();
    create(&mut connection, written_in(&space_id, Language::Yml), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            languages: vec![Language::Json],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.available_languages, [Language::Json, Language::Yml]);
    assert_eq!(view.matched, 1);
}

#[test]
fn available_tags_are_sorted_and_de_duplicated() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, tagged(&space_id, &["zeta", "alpha"]), t0()).unwrap();
    create(&mut connection, tagged(&space_id, &["alpha", "beta"]), t0()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(view.available_tags, ["alpha", "beta", "zeta"]);
}

#[test]
fn available_tags_are_scoped_to_the_active_space() {
    let mut connection = open_in_memory().unwrap();
    let here = space(&mut connection, "Personal");
    let elsewhere = space(&mut connection, "Boulot");
    create(&mut connection, tagged(&here, &["here-tag"]), t0()).unwrap();
    create(
        &mut connection,
        tagged(&elsewhere, &["elsewhere-tag"]),
        t0(),
    )
    .unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            space_id: Some(here),
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.available_tags, ["here-tag"]);
}

#[test]
fn available_tags_ignore_the_current_search_and_selection() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, tagged(&space_id, &["urgent"]), t0()).unwrap();
    create(&mut connection, tagged(&space_id, &["later"]), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            tags: vec!["urgent".to_string()],
            ..all_notes()
        },
    )
    .unwrap();

    assert_eq!(view.available_tags, ["later", "urgent"]);
    assert_eq!(view.matched, 1);
}

#[test]
fn a_search_matching_nothing_reports_filtering_with_zero_matches() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(&mut connection, draft(&space_id), t0()).unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            search: "no-such-thing".to_string(),
            ..all_notes()
        },
    )
    .unwrap();

    assert!(view.is_filtering);
    assert_eq!(view.matched, 0);
}

#[test]
fn the_view_orders_notes_most_recently_updated_first() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let older = create(&mut connection, draft(&space_id), t0()).unwrap();
    let newer = create(&mut connection, draft(&space_id), t1()).unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();

    assert_eq!(matched_ids(&view), [newer.id, older.id]);
}

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

/// ⚠️ The point of the whole thing, and the only test that reads the file rather than the
/// API: a note written through the store must not be findable by grepping the database.
#[test]
fn a_note_is_not_readable_in_the_file_it_was_written_to() {
    use devnotes_lib::db;

    let directory = std::env::temp_dir().join(format!(
        "devnotes-sealed-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
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
    // ⚠️ Tags and the names of variables are deliberately not sealed: the facet, the
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

    std::fs::remove_dir_all(&directory).ok();
}

mod revisions {
    use super::*;
    use devnotes_lib::notes::revision::{KEEP, Revision};
    use devnotes_lib::notes::store::restore_revision;
    use devnotes_lib::notes::store::revisions::{content_of, list};

    fn body(content: &str) -> NotePatch {
        NotePatch {
            content: Some(content.to_string()),
            ..Default::default()
        }
    }

    fn snippet(connection: &mut Library) -> String {
        let space_id = space(connection, "Personal");
        create(connection, draft(&space_id), t0()).unwrap().id
    }

    fn kept(connection: &mut Library, id: &str) -> Vec<Revision> {
        let (db, vault) = connection.split();
        list(db, vault, id).unwrap()
    }

    fn opened(connection: &mut Library, note_id: &str, revision_id: &str) -> String {
        let (db, vault) = connection.split();
        content_of(db, vault, note_id, revision_id)
            .unwrap()
            .unwrap()
    }

    /// ⚠️ The whole of #39: it is the body **before** the edit that is worth keeping —
    /// the version that worked, not the one that has just stopped working.
    #[test]
    fn an_edit_keeps_the_body_it_replaced() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);

        update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

        let history = kept(&mut connection, &id);
        assert_eq!(history.len(), 1);
        assert_eq!(opened(&mut connection, &id, &history[0].id), "Contenu");
    }

    /// ⚠️ Creating a note writes nothing until the first change worth keeping, and that
    /// write is the **title**: the body arrives as a second update, replacing the empty
    /// string the row was born with. Without the guard, every note came out of its first
    /// editing session already carrying a revision of nothing.
    #[test]
    fn the_first_body_typed_replaces_nothing_worth_keeping() {
        let mut connection = open_in_memory().unwrap();
        let space_id = space(&mut connection, "Personal");
        let id = create(
            &mut connection,
            NoteDraft {
                content: String::new(),
                ..draft(&space_id)
            },
            t0(),
        )
        .unwrap()
        .id;

        update(&mut connection, &id, &body("select 1"), t1()).unwrap();

        assert!(kept(&mut connection, &id).is_empty());

        // And the one after it is kept, which is the whole point.
        update(&mut connection, &id, &body("select 2"), t1()).unwrap();
        assert_eq!(kept(&mut connection, &id).len(), 1);
    }

    /// ⚠️ The front end already computes what moved, and the back end agrees: a
    /// title-only edit stores no body.
    #[test]
    fn a_title_only_edit_keeps_nothing() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);

        update(
            &mut connection,
            &id,
            &NotePatch {
                title: Some("Un autre titre".to_string()),
                ..Default::default()
            },
            t1(),
        )
        .unwrap();

        assert!(kept(&mut connection, &id).is_empty());
    }

    /// ⚠️ The editor commits on blur *and* on every closing path, so one session writes
    /// the same body several times. Without the skip, each write would push a duplicate
    /// and rotate a genuinely different version out of the cap.
    #[test]
    fn writing_the_same_body_again_keeps_one_copy_of_it() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);

        update(&mut connection, &id, &body("une"), t1()).unwrap();
        update(&mut connection, &id, &body("deux"), t1()).unwrap();
        update(&mut connection, &id, &body("deux"), t1()).unwrap();

        // "Contenu" then "une": the third write moved nothing, and the second was
        // already the newest kept body.
        let history = kept(&mut connection, &id);
        assert_eq!(history.len(), 2);
    }

    /// ⚠️ A count and not a time window: a body runs to tens of kilobytes.
    #[test]
    fn only_the_last_few_are_kept() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);

        for step in 0..KEEP + 5 {
            update(
                &mut connection,
                &id,
                &body(&format!("version {step}")),
                t1(),
            )
            .unwrap();
        }

        assert_eq!(kept(&mut connection, &id).len(), KEEP);
    }

    #[test]
    fn a_kept_body_goes_back_onto_the_note() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);
        update(&mut connection, &id, &body("casse"), t1()).unwrap();
        let history = kept(&mut connection, &id);

        let note = restore_revision(&mut connection, &id, &history[0].id, t1()).unwrap();

        assert_eq!(note.content, "Contenu");
        assert_eq!(
            by_ids(&mut connection, &[id]).unwrap()[0].content,
            "Contenu"
        );
    }

    /// ⚠️ The canvas sorts on `updated_at`, and putting something back is not editing it
    /// — the same line `restore_many`, `restore_placements` and `untag_many` hold.
    #[test]
    fn putting_a_body_back_does_not_float_the_note_to_the_top() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);
        update(&mut connection, &id, &body("casse"), t1()).unwrap();
        let before = by_ids(&mut connection, std::slice::from_ref(&id)).unwrap()[0].updated_at;
        let history = kept(&mut connection, &id);

        restore_revision(
            &mut connection,
            &id,
            &history[0].id,
            at("2026-07-26T09:00:00.000Z"),
        )
        .unwrap();

        assert_eq!(
            by_ids(&mut connection, &[id]).unwrap()[0].updated_at,
            before
        );
    }

    /// A restore is as undoable as the edit that made it necessary.
    #[test]
    fn the_body_a_restore_replaced_is_kept_too() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);
        update(&mut connection, &id, &body("casse"), t1()).unwrap();
        let history = kept(&mut connection, &id);

        restore_revision(&mut connection, &id, &history[0].id, t1()).unwrap();

        let after = kept(&mut connection, &id);
        assert_eq!(after.len(), 2);
        assert_eq!(opened(&mut connection, &id, &after[0].id), "casse");
    }

    /// ⚠️ An id comes from the front end: one note's history must not be reachable
    /// through another note.
    #[test]
    fn a_revision_of_another_note_is_not_reachable() {
        let mut connection = open_in_memory().unwrap();
        let space_id = space(&mut connection, "Personal");
        let mine = create(&mut connection, draft(&space_id), t0()).unwrap().id;
        let theirs = create(&mut connection, draft(&space_id), t0()).unwrap().id;
        update(&mut connection, &theirs, &body("le leur"), t1()).unwrap();
        let history = kept(&mut connection, &theirs);

        let refused = restore_revision(&mut connection, &mine, &history[0].id, t1());

        assert!(matches!(refused, Err(StorageError::NoteNotFound(_))));
    }

    /// ⚠️ Said out loud rather than shipped silently: a checklist's items live in
    /// `note_items`, a second table to snapshot and a two-step restore. Half the note
    /// kinds get nothing from this first version.
    #[test]
    fn a_checklist_keeps_nothing_yet() {
        let mut connection = open_in_memory().unwrap();
        let space_id = space(&mut connection, "Personal");
        let id = create(
            &mut connection,
            NoteDraft {
                kind: NoteKind::Checklist,
                content: String::new(),
                items: vec![ChecklistItem {
                    text: "un item".to_string(),
                    done: false,
                }],
                ..draft(&space_id)
            },
            t0(),
        )
        .unwrap()
        .id;

        update(&mut connection, &id, &body("un corps"), t1()).unwrap();

        assert!(kept(&mut connection, &id).is_empty());
    }

    /// ⚠️ A history of every body in plaintext beside a sealed library would undo the
    /// encryption entirely.
    #[test]
    fn a_kept_body_is_not_readable_in_the_column_it_sits_in() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);
        update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

        let stored: Vec<String> = note_revisions::table
            .select(note_revisions::content)
            .load(connection.db())
            .unwrap();

        assert_eq!(stored.len(), 1);
        assert!(!stored[0].contains("Contenu"));
    }

    /// ⚠️ Deleting a note takes its history with it: `ON DELETE CASCADE`, which is inert
    /// without `PRAGMA foreign_keys` — set per connection in `db::configure`.
    #[test]
    fn purging_a_note_takes_its_history_with_it() {
        let mut connection = open_in_memory().unwrap();
        let id = snippet(&mut connection);
        update(&mut connection, &id, &body("psql -h prod"), t1()).unwrap();

        trash(&mut connection, &id, t1()).unwrap();
        purge(&mut connection, std::slice::from_ref(&id)).unwrap();

        let left: i64 = note_revisions::table
            .count()
            .get_result(connection.db())
            .unwrap();
        assert_eq!(left, 0);
    }
}
