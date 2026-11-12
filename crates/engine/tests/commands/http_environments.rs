//! The environments' commands: a name checked before the lock, a name held once, a set gone
//! said by its code, and each variable's fault by its rank.

use engine::error::ErrorCode;
use engine::folders::model::FolderColour;
use engine::http::environments::{EnvironmentPatch, Variable, VariableFault, VariableKind};
use engine::http::{
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
        .call(|e| create_http_environment(e, "  Local  ".to_string()))
        .unwrap();
    assert_eq!(local.name, "Local");

    let patch = EnvironmentPatch {
        name: Some("Dev".to_string()),
        colour: Some(FolderColour::Green),
        protected: Some(true),
        variables: Some(vec![variable("token", "t0k3n", VariableKind::Secret)]),
    };
    let saved = session
        .call(|e| save_http_environment(e, local.id.clone(), patch))
        .unwrap();
    assert_eq!(
        (saved.name.as_str(), saved.colour, saved.protected),
        ("Dev", FolderColour::Green, true)
    );

    let globals = vec![variable("tenant", "acme", VariableKind::Text)];
    let kept = session
        .call(|e| save_http_globals(e, globals.clone()))
        .unwrap();
    assert_eq!(kept, globals);

    let copy = session
        .call(|e| duplicate_http_environment(e, local.id.clone(), "Dev (copie)".to_string()))
        .unwrap();
    assert_eq!(copy.variables, saved.variables);

    session
        .call(|e| delete_http_environment(e, local.id.clone()))
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
        code(session.call(|e| create_http_environment(e, "   ".to_string()))),
        ErrorCode::InvalidInput
    );
    let local = session
        .call(|e| create_http_environment(e, "Local".to_string()))
        .unwrap();
    assert_eq!(
        code(session.call(|e| create_http_environment(e, "LOCAL".to_string()))),
        ErrorCode::DuplicateEnvironmentName
    );
    let renaming = EnvironmentPatch {
        name: Some(" ".to_string()),
        ..EnvironmentPatch::default()
    };
    assert_eq!(
        code(session.call(|e| save_http_environment(e, local.id.clone(), renaming))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(session.call(|e| duplicate_http_environment(e, local.id.clone(), String::new()))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(session.call(|e| delete_http_environment(e, "gone".to_string()))),
        ErrorCode::HttpEnvironmentNotFound
    );
    assert_eq!(
        code(session.call(|e| save_http_environment(
            e,
            "gone".to_string(),
            EnvironmentPatch::default()
        ))),
        ErrorCode::HttpEnvironmentNotFound
    );
}

#[test]
fn the_variables_are_judged_row_by_row() {
    let session = Session::open();
    let faults = session
        .wait(|_| {
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
