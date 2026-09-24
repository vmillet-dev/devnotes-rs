use super::*;

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
