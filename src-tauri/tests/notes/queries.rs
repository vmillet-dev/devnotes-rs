use super::*;

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

fn long_body() -> String {
    (1..=40)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn only_note(view: &NotesView) -> &DisplayNote {
    let mut notes = view.sections.iter().flat_map(|section| &section.notes);
    let note = notes.next().expect("a note");
    assert!(notes.next().is_none(), "one note only");

    note
}

/// A list sends the head of a long body; the editor reads the rest with `get`.
#[test]
fn a_list_sends_the_head_of_a_long_body_and_get_the_whole_of_it() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(
        &mut connection,
        NoteDraft {
            content: long_body(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();

    let view = query(&mut connection, &all_notes()).unwrap();
    let listed = only_note(&view);
    assert!(listed.truncated);
    assert_eq!(listed.content.lines().count(), PREVIEW_LINES);

    assert_eq!(get(&mut connection, &note.id).unwrap().content, long_body());
}

/// The search reads the whole body before the cut: a hit far down still shows its line.
#[test]
fn a_hit_past_the_preview_still_shows_its_line() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    create(
        &mut connection,
        NoteDraft {
            content: long_body(),
            ..draft(&space_id)
        },
        t0(),
    )
    .unwrap();

    let view = query(
        &mut connection,
        &NotesQuery {
            search: "line 40".to_string(),
            ..all_notes()
        },
    )
    .unwrap();
    let listed = only_note(&view);

    assert!(listed.truncated);
    assert_eq!(listed.search_hit.as_ref().unwrap().excerpt, "line 40");
}

#[test]
fn get_finds_neither_a_trashed_note_nor_an_unknown_one() {
    let mut connection = open_in_memory().unwrap();
    let space_id = space(&mut connection, "Personal");
    let note = create(&mut connection, draft(&space_id), t0()).unwrap();
    trash(&mut connection, &note.id, t1()).unwrap();

    for id in [note.id.as_str(), "no-such-note"] {
        assert!(matches!(
            get(&mut connection, id),
            Err(StorageError::NoteNotFound(missing)) if missing == id
        ));
    }
}
