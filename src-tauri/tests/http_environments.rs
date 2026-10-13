//! The environments and the globals as data: sealed sets of variables, ordered, named once,
//! copied with their secrets and deleted alone.

use devnotes_lib::db::{Library, open_in_memory};
use devnotes_lib::error::StorageError;
use devnotes_lib::folders::model::FolderColour;
use devnotes_lib::http::environments::{EnvironmentPatch, Variable, VariableKind, store};

mod common;
use common::{t0, t1};

fn variable(name: &str, value: &str, kind: VariableKind) -> Variable {
    Variable {
        name: name.to_string(),
        value: value.to_string(),
        kind,
        ..Variable::default()
    }
}

fn names(library: &mut Library) -> Vec<String> {
    store::list(library)
        .unwrap()
        .environments
        .into_iter()
        .map(|environment| environment.name)
        .collect()
}

#[test]
fn a_new_library_has_no_environment_and_no_global() {
    let mut library = open_in_memory().unwrap();

    let all = store::list(&mut library).unwrap();
    assert!(all.globals.is_empty());
    assert!(all.environments.is_empty());
}

#[test]
fn environments_are_listed_in_the_order_they_were_made_each_in_the_next_colour() {
    let mut library = open_in_memory().unwrap();
    let local = store::create(&mut library, "Local", t0()).unwrap();
    let staging = store::create(&mut library, "Staging", t1()).unwrap();

    assert_eq!(names(&mut library), ["Local", "Staging"]);
    assert_eq!(local.colour, FolderColour::nth(0));
    assert_eq!(staging.colour, FolderColour::nth(1));
    assert!(!staging.protected);
    assert!(staging.variables.is_empty());
}

#[test]
fn a_name_is_held_once_whatever_its_case() {
    let mut library = open_in_memory().unwrap();
    let local = store::create(&mut library, "Local", t0()).unwrap();
    store::create(&mut library, "Staging", t0()).unwrap();

    assert!(matches!(
        store::create(&mut library, "LOCAL", t0()),
        Err(StorageError::DuplicateEnvironmentName(_))
    ));
    let renaming = |name: &str| EnvironmentPatch {
        name: Some(name.to_string()),
        ..EnvironmentPatch::default()
    };
    assert!(matches!(
        store::save(
            &mut library,
            &local.id,
            &renaming("staging"),
            Some("staging")
        ),
        Err(StorageError::DuplicateEnvironmentName(_))
    ));
    // Its own name in another case is no collision.
    let renamed = store::save(&mut library, &local.id, &renaming("LOCAL"), Some("LOCAL")).unwrap();
    assert_eq!(renamed.name, "LOCAL");
}

#[test]
fn a_patch_writes_what_it_names_and_keeps_the_rest() {
    let mut library = open_in_memory().unwrap();
    let production = store::create(&mut library, "Production", t0()).unwrap();
    let variables = vec![
        variable("baseUrl", "https://api.exemple.fr", VariableKind::Text),
        variable("clientSecret", "s3cr3t", VariableKind::Secret),
    ];

    store::save(
        &mut library,
        &production.id,
        &EnvironmentPatch {
            variables: Some(variables.clone()),
            ..EnvironmentPatch::default()
        },
        None,
    )
    .unwrap();
    let saved = store::save(
        &mut library,
        &production.id,
        &EnvironmentPatch {
            protected: Some(true),
            colour: Some(FolderColour::Red),
            ..EnvironmentPatch::default()
        },
        None,
    )
    .unwrap();

    assert!(saved.protected);
    assert_eq!(saved.colour, FolderColour::Red);
    assert_eq!(saved.variables, variables);
    assert_eq!(store::get(&mut library, &production.id).unwrap(), saved);
}

#[test]
fn the_globals_are_one_set_written_whole() {
    let mut library = open_in_memory().unwrap();
    store::create(&mut library, "Local", t0()).unwrap();

    store::save_globals(
        &mut library,
        &[variable("tenant", "acme", VariableKind::Text)],
        t0(),
    )
    .unwrap();
    store::save_globals(
        &mut library,
        &[variable("tenant", "globex", VariableKind::Text)],
        t1(),
    )
    .unwrap();

    let all = store::list(&mut library).unwrap();
    assert_eq!(
        all.globals,
        [variable("tenant", "globex", VariableKind::Text)]
    );
    // Not an environment: it is neither listed nor found as one.
    assert_eq!(names(&mut library), ["Local"]);
}

#[test]
fn a_copy_sits_right_after_its_original_with_its_secrets() {
    let mut library = open_in_memory().unwrap();
    let local = store::create(&mut library, "Local", t0()).unwrap();
    store::create(&mut library, "Production", t0()).unwrap();
    let secret = vec![variable("token", "t0k3n", VariableKind::Secret)];
    store::save(
        &mut library,
        &local.id,
        &EnvironmentPatch {
            variables: Some(secret.clone()),
            protected: Some(true),
            ..EnvironmentPatch::default()
        },
        None,
    )
    .unwrap();

    let copy = store::duplicate(&mut library, &local.id, "Local (copie)", t1()).unwrap();

    assert_eq!(
        names(&mut library),
        ["Local", "Local (copie)", "Production"]
    );
    assert_eq!(copy.variables, secret);
    assert!(copy.protected);
    assert!(matches!(
        store::duplicate(&mut library, &local.id, "local", t1()),
        Err(StorageError::DuplicateEnvironmentName(_))
    ));
}

#[test]
fn a_deletion_takes_that_environment_alone() {
    let mut library = open_in_memory().unwrap();
    let local = store::create(&mut library, "Local", t0()).unwrap();
    store::create(&mut library, "Staging", t0()).unwrap();
    store::save_globals(
        &mut library,
        &[variable("a", "1", VariableKind::Text)],
        t0(),
    )
    .unwrap();

    store::delete(&mut library, &local.id).unwrap();

    assert_eq!(names(&mut library), ["Staging"]);
    assert_eq!(store::list(&mut library).unwrap().globals.len(), 1);
    assert!(matches!(
        store::delete(&mut library, &local.id),
        Err(StorageError::HttpEnvironmentNotFound(_))
    ));
    assert!(matches!(
        store::get(&mut library, &local.id),
        Err(StorageError::HttpEnvironmentNotFound(_))
    ));
}
