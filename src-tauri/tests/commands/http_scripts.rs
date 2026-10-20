//! The scripts around a send, run where the application runs them: in the executable started
//! again as a script host. What they change leaves, what they write is kept, and what stops
//! them says where.

use std::path::PathBuf;
use std::time::Duration;

use devnotes_lib::error::ErrorCode;
use devnotes_lib::http::environments::{EnvironmentPatch, FilledBy, Variable, VariableKind};
use devnotes_lib::http::model::{
    ContainerSettings, Extraction, HttpItem, HttpItemKind, HttpMethod, RequestScripts,
};
use devnotes_lib::http::scripts::model::{FailureKind, ScriptKind, ScriptLevel};
use devnotes_lib::http::scripts::process::host_from;
use devnotes_lib::http::{
    create_http_collection, create_http_environment, http_environments, http_history,
    http_history_entry, http_settings, save_http_environment, save_http_settings,
    send_http_request,
};

use super::Session;
use super::http_send::{header_of, request, response, serve};

fn hosted() -> Session {
    host_from(PathBuf::from(env!("CARGO_BIN_EXE_devnotes")));
    Session::open()
}

fn scripts(before: &str, after: &str) -> RequestScripts {
    RequestScripts {
        before: before.to_string(),
        after: after.to_string(),
        ..RequestScripts::default()
    }
}

/// An environment holding `token`, a secret.
fn environment(session: &Session, name: &str, protected: bool) -> String {
    let id = session
        .call(|app| create_http_environment(name.to_string(), app))
        .unwrap()
        .id;
    let patch = EnvironmentPatch {
        protected: Some(protected),
        variables: Some(vec![Variable {
            name: "token".to_string(),
            value: "s3cr3t-value".to_string(),
            kind: VariableKind::Secret,
            ..Variable::default()
        }]),
        ..EnvironmentPatch::default()
    };
    session
        .call(|app| save_http_environment(id.clone(), patch, app))
        .unwrap();
    id
}

fn ok() -> Vec<u8> {
    response(
        "200 OK",
        &[("Content-Type", "application/json")],
        b"{\"id\":7}",
    )
}

#[test]
fn a_script_before_changes_what_leaves_and_what_the_scripts_write_is_kept() {
    let session = hosted();
    let staging = environment(&session, "Staging", false);
    let (port, seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/{{{{path}}}}"));
    sent.environment_id = Some(staging.clone());
    sent.document.scripts = scripts(
        "pm.variables.set('path', 'orders');\n\
         pm.request.headers.add({ key: 'X-Signed', value: 'yes' });\n\
         console.log('token', env.get('token'));",
        "env.set('orderId', String(res.json().id));",
    );

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    let seen = seen.recv().unwrap();
    assert!(seen.head.starts_with("GET /orders "), "{}", seen.head);
    assert_eq!(header_of(&seen, "X-Signed").as_deref(), Some("yes"));
    assert!(answer.scripts.failure.is_none());
    assert!(answer.scripts.wrote);
    // A secret in force is never shown in clear, not even in what a script logged.
    assert_eq!(answer.scripts.logs[0].text, "token {{token}}");
    let sets = session.call(http_environments).unwrap();
    let kept = sets
        .environments
        .iter()
        .find(|environment| environment.id == staging)
        .unwrap();
    assert!(
        kept.variables
            .iter()
            .any(|variable| variable.name == "orderId" && variable.value == "7")
    );
}

#[test]
fn a_script_that_throws_before_the_send_keeps_the_request_in_and_says_whose_it_is() {
    let session = hosted();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let settings = ContainerSettings {
        scripts: scripts("const a = 1;\nthrow new Error('no token');", ""),
        ..ContainerSettings::default()
    };
    let item = HttpItem {
        kind: HttpItemKind::Collection,
        id: api.id.clone(),
    };
    session
        .call(|app| save_http_settings(item, settings, app))
        .unwrap();
    let (port, seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/"));
    sent.collection_id = Some(api.id.clone());

    let error = session
        .call(|app| send_http_request(sent, app))
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::HttpScript);
    assert_eq!(error.params["failure"], FailureKind::Error.to_string());
    assert_eq!(error.params["level"], ScriptLevel::Collection.to_string());
    assert_eq!(error.params["name"], "API");
    assert_eq!(error.params["owner"], api.id);
    assert_eq!(error.params["line"], "2");
    assert!(error.params["message"].contains("no token"));
    assert!(seen.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_script_after_the_answer_that_fails_leaves_the_answer_shown() {
    let session = hosted();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let (port, _seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/"));
    sent.collection_id = Some(api.id.clone());
    sent.document.scripts = scripts(
        "",
        "pm.collectionVariables.set('lastId', res.json().id);\nres.json().missing.field;",
    );

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    assert_eq!(answer.status, 200);
    let failure = answer.scripts.failure.unwrap();
    assert_eq!(failure.kind, FailureKind::Error);
    assert_eq!(failure.script, Some(ScriptKind::After));
    assert_eq!(failure.level, Some(ScriptLevel::Request));
    assert_eq!(failure.line, Some(2));
    // What it wrote before it failed is kept.
    let item = HttpItem {
        kind: HttpItemKind::Collection,
        id: api.id,
    };
    let held = session.call(|app| http_settings(item, app)).unwrap();
    assert_eq!(held.variables[0].name, "lastId");
    assert_eq!(held.variables[0].value, "7");
}

#[test]
fn a_protected_environment_refuses_what_the_script_made_a_change_unless_confirmed() {
    let session = hosted();
    let production = environment(&session, "Production", true);
    let (port, seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/"));
    sent.environment_id = Some(production);
    sent.document.scripts = scripts("pm.request.method = 'post';", "");

    let error = session
        .call(|app| send_http_request(sent.clone(), app))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::HttpScriptProtected);
    assert_eq!(error.params["method"], HttpMethod::Post.to_string());
    assert_eq!(error.params["environment"], "Production");
    assert!(seen.recv_timeout(Duration::from_millis(300)).is_err());

    sent.confirmed = true;
    session.call(|app| send_http_request(sent, app)).unwrap();
    assert!(seen.recv().unwrap().head.starts_with("POST / "));
}

#[test]
fn a_script_can_skip_its_request_or_name_a_method_that_does_not_exist() {
    let session = hosted();
    let mut sent = request("http://127.0.0.1:9/");
    sent.document.scripts = scripts("pm.execution.skipRequest();", "");
    let skipped = session.call(|app| send_http_request(sent.clone(), app));
    assert_eq!(skipped.unwrap_err().code, ErrorCode::HttpScriptSkipped);

    sent.document.scripts = scripts("pm.request.method = 'PURGE';", "");
    let error = session
        .call(|app| send_http_request(sent, app))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::HttpScriptMethod);
    assert_eq!(error.params["method"], "PURGE");
}

#[test]
fn with_no_environment_chosen_a_script_writes_the_globals() {
    let session = hosted();
    let (port, _seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/"));
    sent.document.scripts = scripts("env.set('tenant', 'acme');", "");

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    assert!(answer.scripts.wrote);
    let sets = session.call(http_environments).unwrap();
    assert_eq!(sets.globals[0].name, "tenant");
    assert_eq!(sets.globals[0].value, "acme");
}

#[test]
fn the_tests_run_after_the_scripts_and_say_what_was_expected_and_what_came() {
    let session = hosted();
    let staging = environment(&session, "Staging", false);
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let settings = ContainerSettings {
        scripts: RequestScripts {
            tests:
                "test('trace', () => {\n  expect(res.headers).toHaveProperty('x-request-id');\n});"
                    .to_string(),
            ..RequestScripts::default()
        },
        ..ContainerSettings::default()
    };
    let item = HttpItem {
        kind: HttpItemKind::Collection,
        id: api.id.clone(),
    };
    session
        .call(|app| save_http_settings(item, settings, app))
        .unwrap();
    let (port, _seen) = serve(1, |_, _| ok());
    let mut sent = request(&format!("http://127.0.0.1:{port}/"));
    sent.collection_id = Some(api.id.clone());
    sent.environment_id = Some(staging);
    sent.document.scripts = RequestScripts {
        before: String::new(),
        after: "pm.test('from after', () => pm.response.to.have.status(200));".to_string(),
        tests: "test('status', () => {\n  expect(res.status).toBe(200);\n});\n\
                pm.test('token', () => {\n  pm.expect(env.get('token')).to.equal('other');\n});"
            .to_string(),
    };

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    let tests = &answer.scripts.tests;
    let names: Vec<(&str, bool)> = tests
        .iter()
        .map(|test| (test.name.as_str(), test.passed))
        .collect();
    assert_eq!(
        names,
        [
            ("from after", true),
            ("status", true),
            ("token", false),
            ("trace", false)
        ]
    );
    let token = &tests[2];
    assert_eq!(token.kind, Some(ScriptKind::Tests));
    assert_eq!(token.level, Some(ScriptLevel::Request));
    assert_eq!(token.line, Some(5));
    assert_eq!(token.expected.as_deref(), Some("\"other\""));
    // The secret it compared is never shown in clear.
    assert_eq!(token.actual.as_deref(), Some("\"{{token}}\""));
    assert_eq!(
        token.code.as_deref(),
        Some("pm.expect(env.get('token')).to.equal('other');")
    );
    let trace = &tests[3];
    assert_eq!(trace.level, Some(ScriptLevel::Collection));
    assert_eq!(trace.holder.as_deref(), Some("API"));
    assert_eq!(trace.owner.as_deref(), Some(api.id.as_str()));
    assert_eq!(trace.line, Some(2));

    // Kept with the history entry.
    let day = &session.call(|app| http_history(0, app)).unwrap()[0];
    let entry = session
        .call(|app| http_history_entry(day.items[0].id.clone(), app))
        .unwrap();
    let kept = entry.record.response.unwrap().scripts.tests;
    assert_eq!(kept.len(), 4);
    assert_eq!(kept[2].actual.as_deref(), Some("\"{{token}}\""));
}

#[test]
fn an_extraction_writes_the_value_after_the_scripts_and_says_which_request_filled_it() {
    let session = hosted();
    let staging = environment(&session, "Staging", false);
    let patch = EnvironmentPatch {
        variables: Some(vec![
            Variable {
                name: "refresh".to_string(),
                value: "kept".to_string(),
                ..Variable::default()
            },
            Variable {
                name: "accessToken".to_string(),
                value: "old".to_string(),
                ..Variable::default()
            },
        ]),
        ..EnvironmentPatch::default()
    };
    session
        .call(|app| save_http_environment(staging.clone(), patch, app))
        .unwrap();
    let (port, _seen) = serve(1, |_, _| {
        response(
            "200 OK",
            &[("Content-Type", "application/json")],
            b"{\"access_token\":\"eyJ.a.b\",\"expires_in\":3600}",
        )
    });
    let mut sent = request(&format!("http://127.0.0.1:{port}/login"));
    sent.environment_id = Some(staging.clone());
    sent.request_id = Some("login-id".to_string());
    sent.name = "Login".to_string();
    sent.document.scripts = scripts(
        "",
        "env.set('accessToken', 'from-script');\nconsole.log(res.json().access_token);",
    );
    sent.document.extractions = vec![
        Extraction {
            path: "body.access_token".to_string(),
            variable: "accessToken".to_string(),
            secret: true,
        },
        Extraction {
            path: "body.refresh_token".to_string(),
            variable: "refresh".to_string(),
            secret: false,
        },
    ];

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    let found: Vec<(&str, bool)> = answer
        .scripts
        .extracted
        .iter()
        .map(|extracted| (extracted.variable.as_str(), extracted.found))
        .collect();
    assert_eq!(found, [("accessToken", true), ("refresh", false)]);
    // Made a secret by the rule, it is masked in what the script logged before it.
    assert_eq!(answer.scripts.logs[0].text, "{{accessToken}}");
    let sets = session.call(http_environments).unwrap();
    let rows = &sets
        .environments
        .iter()
        .find(|environment| environment.id == staging)
        .unwrap()
        .variables;
    let token = rows.iter().find(|row| row.name == "accessToken").unwrap();
    assert_eq!(token.value, "eyJ.a.b");
    assert_eq!(token.kind, VariableKind::Secret);
    assert_eq!(
        token.filled_by,
        Some(FilledBy {
            request_id: Some("login-id".to_string()),
            name: "Login".to_string(),
        })
    );
    let refresh = rows.iter().find(|row| row.name == "refresh").unwrap();
    assert_eq!(refresh.value, "kept");
    assert_eq!(refresh.filled_by, None);
}
