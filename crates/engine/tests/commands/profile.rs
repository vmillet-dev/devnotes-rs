//! What reaches the profile the engine was given: the registry of libraries, the gate's state,
//! the recovery commands and the shipped lists.

use engine::error::ErrorCode;
use engine::libraries::{
    create_library, delete_library, list_libraries, open_library, rename_library,
};
use engine::recovery::{archive_locked_library, set_aside_damaged_library};
use engine::tools::generate_identity;
use engine::tools::identity::{Country, IdentityOutcome, IdentityRequest};
use engine::vault::{VaultState, vault_state};

use super::{Session, code};

fn names(session: &Session) -> Vec<String> {
    session
        .call(|app| async move { list_libraries(app) })
        .unwrap()
        .libraries
        .into_iter()
        .map(|library| library.name)
        .collect()
}

#[test]
fn a_library_is_added_renamed_opened_and_another_deleted() {
    let session = Session::open();
    let work = session
        .call(|app| async move { create_library("Work".to_string(), app) })
        .unwrap();
    let old = session
        .call(|app| async move { create_library("Old".to_string(), app) })
        .unwrap();
    assert!(names(&session).contains(&"Work".to_string()));

    let id = work.id.clone();
    session
        .call(|app| async move { rename_library(id, "Archive".to_string(), app) })
        .unwrap();
    assert!(names(&session).contains(&"Archive".to_string()));

    session.call(|app| open_library(work.id, app)).unwrap();
    assert_eq!(session.call(vault_state).unwrap(), VaultState::Absent);

    session.call(|app| delete_library(old.id, app)).unwrap();
    assert!(!names(&session).contains(&"Old".to_string()));
}

#[test]
fn the_gate_says_unlocked_while_a_library_is_open() {
    let session = Session::open();

    assert_eq!(session.call(vault_state).unwrap(), VaultState::Unlocked);
}

#[test]
fn an_open_library_is_never_set_aside() {
    let session = Session::open();

    assert!(matches!(
        code(session.call(set_aside_damaged_library)),
        ErrorCode::LibraryOpen
    ));
    assert!(matches!(
        code(session.call(archive_locked_library)),
        ErrorCode::LibraryOpen
    ));
}

/// The lists are read from the resources the engine was given: here, a folder without them.
#[test]
fn the_identity_tool_answers_that_its_lists_are_missing_rather_than_failing() {
    let session = Session::open();
    let request = IdentityRequest {
        country: Country::Fr,
        sex: None,
        age_min: 18,
        age_max: 80,
        seed: Some("devnotes".to_string()),
    };

    let outcome = session.call(|app| generate_identity(app, request)).unwrap();

    assert!(matches!(outcome, IdentityOutcome::Unreadable { .. }));
}
