//! The environments' commands: a name checked before the lock, a name held once, a set gone
//! said by its code, and each variable's fault by its rank.

use devnotes_lib::error::ErrorCode;
use devnotes_lib::folders::model::FolderColour;
use devnotes_lib::http::environments::{EnvironmentPatch, Variable, VariableFault, VariableKind};
use devnotes_lib::http::{
    check_http_variables, create_http_environment, delete_http_environment,
    duplicate_http_environment, http_environments, save_http_environment, save_http_globals,
};

use super::{Session, code};

fn variable(name: &str, value: &str, kind: VariableKind) -> Variable {
    Variable {
        name: name.to_string(),
        value: value.to_string(),
        kind,
        ..Variable::default()
    }
}

#[test]
fn environments_are_made_written_copied_and_deleted_through_their_commands() {
    let session = Session::open();
    let local = session
        .call(|app| create_http_environment("  Local  ".to_string(), app))
        .unwrap();
    assert_eq!(local.name, "Local");

    let patch = EnvironmentPatch {
        name: Some("Dev".to_string()),
        colour: Some(FolderColour::Green),
        protected: Some(true),
        variables: Some(vec![variable("token", "t0k3n", VariableKind::Secret)]),
    };
    let saved = session
        .call(|app| save_http_environment(local.id.clone(), patch, app))
        .unwrap();
    assert_eq!(
        (saved.name.as_str(), saved.colour, saved.protected),
        ("Dev", FolderColour::Green, true)
    );

    let globals = vec![variable("tenant", "acme", VariableKind::Text)];
    let kept = session
        .call(|app| save_http_globals(globals.clone(), app))
        .unwrap();
    assert_eq!(kept, globals);

    let copy = session
        .call(|app| duplicate_http_environment(local.id.clone(), "Dev (copie)".to_string(), app))
        .unwrap();
    assert_eq!(copy.variables, saved.variables);

    session
        .call(|app| delete_http_environment(local.id.clone(), app))
        .unwrap();
    let all = session.call(http_environments).unwrap();
    assert_eq!(all.globals, globals);
    assert_eq!(
        all.environments
            .iter()
            .map(|environment| environment.name.as_str())
            .collect::<Vec<_>>(),
        ["Dev (copie)"]
    );
}

#[test]
fn a_blank_name_a_name_held_twice_and_a_set_gone_each_answer_their_code() {
    let session = Session::open();
    assert_eq!(
        code(session.call(|app| create_http_environment("   ".to_string(), app))),
        ErrorCode::InvalidInput
    );
    let local = session
        .call(|app| create_http_environment("Local".to_string(), app))
        .unwrap();
    assert_eq!(
        code(session.call(|app| create_http_environment("LOCAL".to_string(), app))),
        ErrorCode::DuplicateEnvironmentName
    );
    let renaming = EnvironmentPatch {
        name: Some(" ".to_string()),
        ..EnvironmentPatch::default()
    };
    assert_eq!(
        code(session.call(|app| save_http_environment(local.id.clone(), renaming, app))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(session.call(|app| duplicate_http_environment(local.id.clone(), String::new(), app))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(session.call(|app| delete_http_environment("gone".to_string(), app))),
        ErrorCode::HttpEnvironmentNotFound
    );
    assert_eq!(
        code(session.call(|app| {
            save_http_environment("gone".to_string(), EnvironmentPatch::default(), app)
        })),
        ErrorCode::HttpEnvironmentNotFound
    );
}

#[test]
fn the_variables_are_judged_row_by_row() {
    let session = Session::open();
    let faults = session
        .call(|_| {
            check_http_variables(vec![
                variable("baseUrl", "https://exemple.fr", VariableKind::Text),
                variable("base url", "", VariableKind::Text),
                variable("pageSize", "fifty", VariableKind::Number),
            ])
        })
        .unwrap();
    assert_eq!(
        faults,
        [None, Some(VariableFault::Name), Some(VariableFault::Number)]
    );
}

#[test]
fn nothing_answers_before_the_library_is_unlocked() {
    let session = Session::locked();
    assert_eq!(code(session.call(http_environments)), ErrorCode::Locked);
}
