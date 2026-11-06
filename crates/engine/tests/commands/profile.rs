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
        .call(list_libraries)
        .libraries
        .into_iter()
        .map(|library| library.name)
        .collect()
}

#[test]
fn a_library_is_added_renamed_opened_and_another_deleted() {
    let session = Session::open();
    let work = session
        .call(|e| create_library(e, "Work".to_string()))
        .unwrap();
    let old = session
        .call(|e| create_library(e, "Old".to_string()))
        .unwrap();
    assert!(names(&session).contains(&"Work".to_string()));

    let id = work.id.clone();
    session
        .call(|e| rename_library(e, id, "Archive".to_string()))
        .unwrap();
    assert!(names(&session).contains(&"Archive".to_string()));

    session.call(|e| open_library(e, work.id)).unwrap();
    assert_eq!(session.call(vault_state).unwrap(), VaultState::Absent);

    session.call(|e| delete_library(e, old.id)).unwrap();
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

    let outcome = session.call(|e| generate_identity(e.paths(), &request));

    assert!(matches!(outcome, IdentityOutcome::Unreadable { .. }));
}

#[test]
fn a_qr_code_is_drawn_as_square_rgba_for_the_shell_to_copy() {
    use engine::tools::qr::{QrContent, QrCorrection, QrRequest};

    let request = |text: &str| QrRequest {
        content: QrContent::Text {
            text: text.to_string(),
        },
        correction: QrCorrection::Medium,
        margin: 2,
    };

    let drawn = engine::tools::qr::rgba(&request("devnotes"));
    let nothing = engine::tools::qr::rgba(&request(""));

    let (side, rgba) = drawn.unwrap();
    assert_eq!(rgba.len(), (side * side * 4) as usize);
    assert!(nothing.is_none());
}
