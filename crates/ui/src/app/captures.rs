//! The window in each state the title bar, the gate and the banners can be in, both themes, drawn
//! on a software adapter (WARP, lavapipe) and written to `DEVNOTES_CAPTURES` when it is set, for
//! a look by eye beside the reference's captures. Nothing here compares pixels.

use std::path::Path;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::DevNotesApp;
use super::tests::startup;
use crate::services::time::fake::FakeClock;
use crate::testing::PASSPHRASE;
use engine::http::model::{HttpMethod, RequestKind};

const SIZE: egui::Vec2 = egui::vec2(1100.0, 720.0);

fn launch(profile: &Path) -> Harness<'static, DevNotesApp> {
    launch_at(profile, SIZE)
}

fn launch_at(profile: &Path, size: egui::Vec2) -> Harness<'static, DevNotesApp> {
    // Loaded before the first frame, as the window has them a moment after: the captures show
    // the colours.
    text::highlight::warm();
    let startup = startup(profile);
    let mut harness = Harness::builder()
        .with_size(size)
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    harness.state_mut().state.clock = Box::new(FakeClock::at("2026-11-03T09:30:00Z"));
    // The window finds the arrows in the system's fonts, which a bench has not.
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    settle(&mut harness);
    harness
}

fn settle(harness: &mut Harness<'_, DevNotesApp>) {
    super::tests::settle(harness);
}

fn click(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    settle(harness);
}

fn type_into(harness: &mut Harness<'_, DevNotesApp>, author_id: &str, words: &str) {
    click(harness, author_id);
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    settle(harness);
}

/// Draws the window as it stands, then again in the other theme, and gives the first one back.
fn capture(harness: &mut Harness<'_, DevNotesApp>, name: &str) {
    shoot(harness, name, true);
}

/// As `capture`, the pointer still pressed where it is: what a drag shows on its way.
fn capture_holding(harness: &mut Harness<'_, DevNotesApp>, name: &str) {
    shoot(harness, name, false);
}

fn shoot(harness: &mut Harness<'_, DevNotesApp>, name: &str, pointer_gone: bool) {
    let directory = std::env::var_os("DEVNOTES_CAPTURES");
    for _ in 0..2 {
        let theme = match harness.ctx.theme() {
            egui::Theme::Dark => "dark",
            egui::Theme::Light => "light",
        };
        // kittest draws its pointer, and where a click left it.
        if pointer_gone {
            harness.event(egui::Event::PointerGone);
        }
        settle(harness);
        let image = harness
            .render()
            .expect("a software adapter draws the window");
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
            image
                .save(Path::new(directory).join(format!("{name}-{theme}.png")))
                .unwrap();
        }
        // By its action, not its button: a click would close what the capture shows.
        let other = match harness.ctx.theme() {
            egui::Theme::Dark => crate::theme::Appearance::Light,
            egui::Theme::Light => crate::theme::Appearance::Dark,
        };
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let action = crate::action::Action::App(crate::action::AppAction::SetAppearance(other));
        app.state.apply(action, &ctx, &app.backend);
        settle(harness);
    }
}

#[test]
fn the_gate_in_each_of_its_states() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    capture(&mut harness, "gate-create");

    type_into(&mut harness, "vault-passphrase", "too short");
    type_into(&mut harness, "vault-confirmation", "too shorts");
    capture(&mut harness, "gate-create-refused");
    drop(harness);

    // A later launch over the library the first one created.
    let mut creating = launch(profile.path());
    type_into(&mut creating, "vault-passphrase", PASSPHRASE);
    type_into(&mut creating, "vault-confirmation", PASSPHRASE);
    click(&mut creating, "vault-submit");
    settle(&mut creating);
    crate::testing::close_library(&creating.state().backend);
    drop(creating);

    let mut harness = launch(profile.path());
    capture(&mut harness, "gate-unlock");

    type_into(&mut harness, "vault-passphrase", "not the phrase");
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "gate-unlock-wrong");

    click(&mut harness, "vault-forgotten");
    capture(&mut harness, "gate-forgotten");
}

#[test]
fn the_title_bar_and_the_banners_over_an_open_library() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "titlebar-notes");

    click(&mut harness, "about-open");
    capture(&mut harness, "about-menu");
    click(&mut harness, "about-about");
    settle(&mut harness);
    capture(&mut harness, "about-card");
    click(&mut harness, "about-close");
    click(&mut harness, "about-open");
    click(&mut harness, "about-getting-started");
    settle(&mut harness);
    capture(&mut harness, "getting-started");
    click(&mut harness, "guide-close");
    click(&mut harness, "about-open");
    click(&mut harness, "about-shortcuts");
    settle(&mut harness);
    capture(&mut harness, "shortcut-sheet");
    click(&mut harness, "sheet-close");
    click(&mut harness, "about-open");
    click(&mut harness, "about-whats-new");
    settle(&mut harness);
    capture(&mut harness, "whats-new");
    click(&mut harness, "news-close");
    settle(&mut harness);

    click(&mut harness, "file-menu");
    capture(&mut harness, "file-menu");
    click(&mut harness, "file-quit");
    capture(&mut harness, "file-menu-quit-asks");
    harness.state_mut().state.dialog = std::sync::Arc::new(platform::dialog::Fixed(
        profile.path().join("library.devnotes"),
    ));
    click(&mut harness, "file-option-exportAll");
    settle(&mut harness);
    capture(&mut harness, "passphrase-prompt");
    click(&mut harness, "passphrase-prompt-cancel");
    settle(&mut harness);
    click(&mut harness, "file-menu");
    click(&mut harness, "file-libraries");
    settle(&mut harness);
    capture(&mut harness, "libraries");
    click(&mut harness, "libraries-close");
    settle(&mut harness);
    click(&mut harness, "file-menu");
    click(&mut harness, "file-preferences");
    settle(&mut harness);
    capture(&mut harness, "preferences-general");
    click(&mut harness, "settings-page-shortcuts");
    settle(&mut harness);
    capture(&mut harness, "preferences-shortcuts");
    click(&mut harness, "settings-page-security");
    settle(&mut harness);
    capture(&mut harness, "preferences-security");
    click(&mut harness, "setting-change-passphrase");
    settle(&mut harness);
    capture(&mut harness, "change-passphrase");
    click(&mut harness, "change-passphrase-cancel");
    settle(&mut harness);
    click(&mut harness, "settings-page-backups");
    settle(&mut harness);
    capture(&mut harness, "preferences-backups");
    click(&mut harness, "settings-page-variables");
    click(&mut harness, "variable-add");
    settle(&mut harness);
    capture(&mut harness, "preferences-variables");
    click(&mut harness, "settings-cancel");
    settle(&mut harness);

    click(&mut harness, "area-option-http");
    let state = &mut harness.state_mut().state;
    state.errors.report(
        i18n::keys::errors::ATTACHMENTS_LOAD_FAILED,
        &engine::error::AppError::from(engine::error::StorageError::Locked),
    );
    state
        .status
        .report(i18n::Message::new(i18n::keys::settings::COPIED));
    settle(&mut harness);
    capture(&mut harness, "banners-http");
}

#[test]
fn the_update_prompt() {
    use crate::services::updates::UpdateState;
    use crate::services::updates::fake::FakeUpdater;
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    harness.state_mut().state.updates = UpdateState::with(FakeUpdater::offering("0.26.0"));

    click(&mut harness, "about-open");
    capture(&mut harness, "about-menu-check");
    click(&mut harness, "about-check-updates");
    settle(&mut harness);
    capture(&mut harness, "update-prompt");

    click(&mut harness, "update-later");
    settle(&mut harness);
    capture(&mut harness, "update-dot");
}

/// A library holding `API Paiements`, the HTTP area shown.
fn http_area(profile: &Path) -> Harness<'static, DevNotesApp> {
    let mut harness = launch(profile);
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = harness.state().backend();
    let api = crate::http::tests::collection(backend, "API Paiements");
    let auth = crate::http::tests::folder(backend, &api, "Auth");
    let methods = [
        ("Login", HttpMethod::Post),
        ("Session", HttpMethod::Get),
        ("Logout", HttpMethod::Delete),
    ];
    for (name, method) in methods {
        crate::http::tests::request(
            backend,
            (&api, Some(&auth)),
            name,
            (RequestKind::Http, method),
        );
    }
    crate::http::tests::request(
        backend,
        (&api, None),
        "Factures",
        (RequestKind::Graphql, HttpMethod::Post),
    );
    crate::http::tests::request(
        backend,
        (&api, None),
        "Flux",
        (RequestKind::Websocket, HttpMethod::Get),
    );
    // The tree is read when the area is first shown.
    click(&mut harness, "area-option-http");
    harness
}

#[test]
fn the_http_request_editor() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state.http.tabs_mut().edit(&key, |draft| {
        use crate::state::http::tabs::DraftEdit;
        draft.apply(DraftEdit::Url(
            "{{base}}/sessions?user={{user}}&remember=true".to_owned(),
        ));
        let header = |key: &str, value: &str| engine::http::model::KeyValue {
            enabled: true,
            key: key.to_owned(),
            value: value.to_owned(),
            description: String::new(),
        };
        draft.apply(DraftEdit::Headers(vec![
            header("Accept", "application/json"),
            header("X Trace", "on"),
        ]));
    });
    settle(&mut harness);
    capture(&mut harness, "http-request-params");
    click(&mut harness, "http-section-headers");
    capture(&mut harness, "http-request-headers");
    click(&mut harness, "http-section-auth");
    capture(&mut harness, "http-request-auth");
    click(&mut harness, "http-section-settings");
    capture(&mut harness, "http-request-settings");
    let app = harness.state_mut();
    app.state.http.tabs_mut().edit(&key, |draft| {
        use engine::http::environments::{FilledBy, Variable, VariableKind};
        draft.apply(crate::state::http::tabs::DraftEdit::Variables(vec![
            Variable {
                name: "user".to_owned(),
                value: "ada".to_owned(),
                ..Variable::default()
            },
            Variable {
                name: "token".to_owned(),
                value: "eyJ.a.b".to_owned(),
                kind: VariableKind::Secret,
                filled_by: Some(FilledBy {
                    request_id: None,
                    name: "Login".to_owned(),
                }),
                ..Variable::default()
            },
        ]));
    });
    click(&mut harness, "http-section-variables");
    capture(&mut harness, "http-request-variables");
    click(&mut harness, "http-environment-picker");
    capture(&mut harness, "http-environment-picker");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "http-section-scripts");
    settle(&mut harness);
    capture(&mut harness, "http-request-scripts");
    click(&mut harness, "http-section-body");
    let app = harness.state_mut();
    app.state.http.tabs_mut().edit(&key, |draft| {
        draft.apply(crate::state::http::tabs::DraftEdit::Body(
            engine::http::model::RequestBody::Json {
                text: "{\n  \"login\": \"ada\",\n  \"remember\": true\n}".to_owned(),
            },
        ));
    });
    settle(&mut harness);
    capture(&mut harness, "http-request-body");
    click(&mut harness, "http-body-kind-4");
    type_into(&mut harness, "http-part-key", "justificatif");
    capture(&mut harness, "http-request-multipart");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node
                    .label()
                    .is_some_and(|label| label.ends_with("Factures"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state.http.tabs_mut().edit(&key, |draft| {
        draft.apply(crate::state::http::tabs::DraftEdit::Graphql(
            engine::http::graphql::GraphqlDocument {
                query: "query Facture($id: ID!) {\n  facture(id: $id) {\n    total\n".to_owned(),
                ..Default::default()
            },
        ));
    });
    settle(&mut harness);
    capture(&mut harness, "http-request-graphql");
}

#[test]
fn the_http_environments() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    click(&mut harness, "http-environments-open");
    settle(&mut harness);
    capture(&mut harness, "http-environments-globals");
    click(&mut harness, "http-environment-create-open");
    type_into(&mut harness, "http-environment-create-input", "Staging");
    click(&mut harness, "http-environment-create-submit");
    settle(&mut harness);
    click(&mut harness, "http-environment-protected");
    settle(&mut harness);
    capture(&mut harness, "http-environments-set");
}

#[test]
fn the_http_socket() {
    use engine::http::websocket::{SocketEvent, WebsocketEvent};
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Flux"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-socket-connect");
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    let at = chrono::Utc::now();
    for event in [
        SocketEvent::Opened {
            url: "wss://api.example.com/Flux".to_owned(),
            protocol: Some("chat".to_owned()),
        },
        SocketEvent::Sent {
            text: "{\"subscribe\":\"prix\"}".to_owned(),
            size: 21,
        },
        SocketEvent::Received {
            text: "{\"prix\":42}".to_owned(),
            size: 10,
            binary: false,
            cut: false,
        },
    ] {
        app.state.http.receive_socket(WebsocketEvent {
            id: format!("{key}#1"),
            at,
            event,
        });
    }
    settle(&mut harness);
    capture(&mut harness, "http-socket");
}

#[test]
fn the_http_dialogs() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-menu")
                && node
                    .label()
                    .is_some_and(|label| label.ends_with("API Paiements"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-node-settings");
    settle(&mut harness);
    capture(&mut harness, "http-settings-dialog");
    click(&mut harness, "http-settings-cancel");
    settle(&mut harness);
    click(&mut harness, "http-cookies-open");
    settle(&mut harness);
    capture(&mut harness, "http-cookies-empty");
}

#[test]
fn the_http_code() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-code-open");
    settle(&mut harness);
    capture(&mut harness, "http-code-dialog");
    click(&mut harness, "http-code-secrets");
    settle(&mut harness);
    capture(&mut harness, "http-code-secrets");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "http-tab-new");
    settle(&mut harness);
    click(&mut harness, "http-save");
    settle(&mut harness);
    capture(&mut harness, "http-save-dialog");
}

#[test]
fn the_http_runner() {
    use engine::http::runner::{RunEvent, RunResult, RunStep};
    use engine::http::scripts::model::TestResult;

    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = harness.state().backend();
    let local = crate::http::tests::collection(backend, "Local");
    for name in ["Login", "Commandes"] {
        crate::http::tests::request(
            backend,
            (&local, None),
            name,
            (RequestKind::Http, HttpMethod::Get),
        );
    }
    click(&mut harness, "area-option-http");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-menu")
                && node.label().is_some_and(|label| label.ends_with("Local"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-node-run");
    settle(&mut harness);
    capture(&mut harness, "http-runner-ready");
    click(&mut harness, "http-runner-start-empty");
    settle(&mut harness);
    let result = |name: &str, status, passed| RunResult {
        iteration: 0,
        request_id: name.to_owned(),
        name: name.to_owned(),
        path: Vec::new(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        status: Some(status),
        reason: String::new(),
        millis: 12,
        tests: vec![TestResult {
            name: "found".to_owned(),
            passed,
            expected: (!passed).then(|| "200".to_owned()),
            actual: (!passed).then(|| status.to_string()),
            code: (!passed).then(|| "expect(res.status).toBe(200);".to_owned()),
            ..TestResult::default()
        }],
        error: None,
        stop: None,
        skipped: false,
        response: None,
    };
    let steps = [
        RunStep::Ended {
            result: Box::new(result("Login", 200, true)),
        },
        RunStep::Ended {
            result: Box::new(result("Commandes", 404, false)),
        },
        RunStep::Finished {
            cancelled: false,
            millis: 1_200,
            capped: false,
        },
    ];
    for step in steps {
        let id = "1".to_owned();
        let http = &mut harness.state_mut().state.http;
        http.receive_run(RunEvent { id, step });
    }
    settle(&mut harness);
    capture(&mut harness, "http-runner-report");
}

#[test]
fn the_http_response() {
    let port = crate::testing::serve();
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = harness.state().backend();
    let local = crate::http::tests::collection(backend, "Local");
    let ping = crate::http::tests::request(
        backend,
        (&local, None),
        "Ping",
        (RequestKind::Http, HttpMethod::Get),
    );
    click(&mut harness, "area-option-http");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Ping"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    app.state.http.tabs_mut().edit(&ping.id, |draft| {
        draft.apply(crate::state::http::tabs::DraftEdit::Url(format!(
            "http://127.0.0.1:{port}/json"
        )));
    });
    settle(&mut harness);
    click(&mut harness, "http-send");
    capture(&mut harness, "http-response-body");
    click(&mut harness, "http-response-view-3");
    capture(&mut harness, "http-response-tree");
    click(&mut harness, "http-response-section-timeline");
    capture(&mut harness, "http-response-timeline");
    click(&mut harness, "http-response-section-body");
    click(&mut harness, "http-response-save-note");
    settle(&mut harness);
    capture(&mut harness, "http-save-as-note");
    click(&mut harness, "save-as-note-cancel");
    settle(&mut harness);
    let http = &mut harness.state_mut().state.http;
    let id = http.sends().of(&ping.id).unwrap().id.clone();
    for text in ["token read", "Bearer {{token}} kept"] {
        http.receive_console(engine::http::console::ConsoleEvent {
            id: id.clone(),
            at: chrono::Utc::now(),
            request: "Ping".to_owned(),
            iteration: None,
            line: engine::http::console::ConsoleLine::Log {
                level: engine::http::scripts::model::LogLevel::default(),
                text: text.to_owned(),
                by: None,
            },
        });
    }
    settle(&mut harness);
    capture(&mut harness, "http-console-folded");
    click(&mut harness, "http-console-unfold");
    capture(&mut harness, "http-console");
    click(&mut harness, "http-console-fold");
    click(&mut harness, "http-history-open");
    settle(&mut harness);
    click(&mut harness, "http-history-item");
    settle(&mut harness);
    capture(&mut harness, "http-history");
    assert!(
        !harness
            .state_mut()
            .state
            .http
            .ask_leave(crate::state::http::leave::Reason::Quit)
    );
    settle(&mut harness);
    capture(&mut harness, "http-leave");
}

#[test]
fn the_http_area() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    capture(&mut harness, "http-rail");

    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-menu")
                && node.label().is_some_and(|label| label.ends_with("Auth"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-node-new");
    capture_holding(&mut harness, "http-node-menu");
    click(&mut harness, "http-node-new-http");
    capture(&mut harness, "http-create-request");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-menu")
                && node
                    .label()
                    .is_some_and(|label| label.ends_with("API Paiements"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-node-delete");
    capture(&mut harness, "http-delete-confirm");
    click(&mut harness, "http-delete-cancel");

    for name in ["Login", "Flux"] {
        harness
            .get_by(|node| {
                node.author_id() == Some("http-node-name")
                    && node.label().is_some_and(|label| label.ends_with(name))
            })
            .click();
        settle(&mut harness);
    }
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state
        .http
        .tabs_mut()
        .edit(&key, |draft| draft.name = "Flux live".to_owned());
    settle(&mut harness);
    capture(&mut harness, "http-tabs");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-tab-close")
                && node
                    .label()
                    .is_some_and(|label| label.ends_with("Flux live"))
        })
        .click();
    settle(&mut harness);
    capture(&mut harness, "http-close-dialog");
}

#[test]
fn the_tools_area() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    click(&mut harness, "area-option-tools");
    capture(&mut harness, "tools-home");
    click(&mut harness, "tools-entry-star-hash");
    click(&mut harness, "tools-entry-convert");
    capture(&mut harness, "tool-frame");
    click(&mut harness, "tool-back");
    capture(&mut harness, "tools-recents");
    type_into(&mut harness, "tools-search", "cors");
    for _ in 0..10 {
        harness.step();
    }
    settle(&mut harness);
    capture(&mut harness, "tools-references");
}

/// Each tool on its example, once it has answered.
#[test]
fn the_tools() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    click(&mut harness, "area-option-tools");
    for tool in &crate::tools::catalogue::TOOLS {
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let open = crate::action::Action::Tools(crate::tools::Action::Open(tool.id));
        app.state.apply(open, &ctx, &app.backend);
        settle(&mut harness);
        if harness
            .query_by(|node| node.author_id() == Some("tool-sample"))
            .is_some()
        {
            click(&mut harness, "tool-sample");
        }
        // Past the debounce: the bench's clock moves as frames do.
        for _ in 0..30 {
            harness.step();
            settle(&mut harness);
        }
        capture(&mut harness, &format!("tool-{}", tool.id));
    }
}

#[test]
fn the_canvas_of_an_open_library() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "canvas-samples");

    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    capture(&mut harness, "canvas-notes");

    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .hover();
    settle(&mut harness);
    click(&mut harness, "note-card-menu");
    capture(&mut harness, "card-menu");
    click(&mut harness, "note-card-menu");
    click(&mut harness, "note-card-fill");
    capture(&mut harness, "placeholder-form");
    click(&mut harness, "placeholder-cancel");

    for _ in 0..2 {
        harness.key_press(egui::Key::ArrowRight);
        harness.key_press(egui::Key::X);
    }
    settle(&mut harness);
    capture(&mut harness, "selection-bar");
    click(&mut harness, "selection-pin");
    capture(&mut harness, "undo-bar");
    click(&mut harness, "undo-restore");
    click(&mut harness, "selection-delete");
    click(&mut harness, "selection-delete");
    click(&mut harness, "trash-open");
    capture(&mut harness, "trash-panel");
    click(&mut harness, "trash-restore-all");
    click(&mut harness, "trash-close");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    click(&mut harness, "sort-trigger");
    capture(&mut harness, "header-sort-menu");
    click(&mut harness, "sort-trigger");

    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .click();
    settle(&mut harness);
    capture(&mut harness, "editor-snippet");
    click(&mut harness, "editor-fullscreen");
    capture(&mut harness, "editor-snippet-expanded");
    click(&mut harness, "editor-back-to-cards");
    click(&mut harness, "editor-fold-header");
    capture(&mut harness, "editor-header-folded");
    click(&mut harness, "editor-header-summary");

    click(&mut harness, "editor-close");
    drawing_editor(&mut harness);
    harness.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
        egui::Key::P,
    );
    settle(&mut harness);
    capture(&mut harness, "palette-recent");
    harness
        .get_by(|node| node.author_id() == Some("palette-input"))
        .type_text("psql");
    // Past the search's pause, in egui's time.
    for _ in 0..20 {
        harness.step();
    }
    settle(&mut harness);
    capture(&mut harness, "palette-search");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "new-note-kind");
    capture(&mut harness, "header-new-note-menu");
    click(&mut harness, "new-note-kind");
    click(&mut harness, "library-rail-toggle");
    click(&mut harness, "space-switcher");
    capture(&mut harness, "header-switchers");
    click(&mut harness, "space-switcher");
    click(&mut harness, "facets-toggle");
    capture(&mut harness, "facets-panel");
    click(&mut harness, "tag-manage");
    harness
        .get_all_by(|node| node.author_id() == Some("tag-item"))
        .next()
        .expect("the samples are tagged")
        .click();
    settle(&mut harness);
    capture(&mut harness, "tag-manager");
    click(&mut harness, "tag-manager-close");
    click(&mut harness, "facets-toggle");
    harness.set_size(egui::vec2(640.0, SIZE.y));
    settle(&mut harness);
    capture(&mut harness, "header-two-rows");
    harness.set_size(SIZE);
}

/// Wide enough for the panel beside two columns of cards: the snippet open, then its ⋯.
#[test]
fn the_editor_beside_the_cards() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .click();
    settle(&mut harness);
    capture(&mut harness, "editor-beside-the-cards");
    click(&mut harness, "editor-more");
    capture(&mut harness, "editor-more-menu");
}

/// Each kind expanded over the cards, with the layout of its own.
#[test]
fn the_editor_full_screen_per_kind() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let ids = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    let shown = ["snippet", "note", "tasks", "calc", "drawing"];
    for (at, (id, name)) in ids.into_iter().zip(shown).enumerate() {
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let open = crate::action::Action::Notes(crate::notes::Action::OpenNote(id));
        app.state.apply(open, &ctx, &app.backend);
        settle(&mut harness);
        if at == 0 {
            click(&mut harness, "editor-fullscreen");
        }
        capture(&mut harness, &format!("editor-fullscreen-{name}"));
        if name == "calc" {
            click(&mut harness, "editor-details");
            capture(&mut harness, "editor-fullscreen-details-hidden");
            click(&mut harness, "editor-details");
        }
        // Full screen is a preference: the next note opens expanded.
        click(&mut harness, "editor-close");
    }
}

#[test]
fn the_board_of_the_samples() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    harness
        .get_all_by(|node| node.author_id() == Some("space-option"))
        .next()
        .expect("the samples have a space")
        .click();
    settle(&mut harness);
    click(&mut harness, "view-board");
    capture(&mut harness, "board");
    harness
        .get_all_by(|node| node.author_id() == Some("board-zone-menu"))
        .next()
        .expect("the samples are filed")
        .click();
    settle(&mut harness);
    capture(&mut harness, "board-zone-menu");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    let card = harness
        .get_all_by(|node| node.author_id() == Some("board-zone-card"))
        .next()
        .expect("a filed card")
        .rect()
        .center();
    let held = drag_to(
        &mut harness,
        card,
        egui::PointerButton::Primary,
        egui::vec2(520.0, 40.0),
    );
    holding(&mut harness, "board-dragging");
    release(&mut harness, held, egui::PointerButton::Primary);
    let band = drag_to(
        &mut harness,
        card,
        egui::PointerButton::Secondary,
        egui::vec2(300.0, 260.0),
    );
    holding(&mut harness, "board-band");
    release(&mut harness, band, egui::PointerButton::Secondary);
}

/// A press at `from`, then the pointer moved by `by` in steps and held there: where it is.
fn drag_to(
    harness: &mut Harness<'_, DevNotesApp>,
    from: egui::Pos2,
    button: egui::PointerButton,
    by: egui::Vec2,
) -> egui::Pos2 {
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.step();
    harness.event(press(from, true));
    harness.step();
    for step in [0.2, 0.4, 0.6, 0.8, 1.0] {
        harness.event(egui::Event::PointerMoved(from + by * step));
        harness.step();
    }
    from + by
}

fn release(harness: &mut Harness<'_, DevNotesApp>, at: egui::Pos2, button: egui::PointerButton) {
    harness.event(egui::Event::PointerButton {
        pos: at,
        button,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(harness);
}

/// The window as it stands, the pointer still held: a gesture under way, in one theme.
fn holding(harness: &mut Harness<'_, DevNotesApp>, name: &str) {
    let image = harness
        .render()
        .expect("a software adapter draws the window");
    if let Some(directory) = std::env::var_os("DEVNOTES_CAPTURES") {
        std::fs::create_dir_all(&directory).unwrap();
        image
            .save(Path::new(&directory).join(format!("{name}.png")))
            .unwrap();
    }
}

/// The Dessin of `kinds`, as it opens and with a shape selected.
fn drawing_editor(harness: &mut Harness<'_, DevNotesApp>) {
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some("Sketch")
        })
        .click();
    settle(harness);
    capture(harness, "editor-drawing");
    let stage = harness
        .get_by(|node| node.author_id() == Some("drawing-stage"))
        .rect();
    let shape = stage.min + egui::vec2(170.0, 90.0);
    harness.event(egui::Event::PointerMoved(shape));
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: shape,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(1);
    }
    settle(harness);
    capture(harness, "editor-drawing-selected");
    click(harness, "editor-close");
}

/// One note of each kind, with marks, tags and a search's worth of words.
/// The notes of each kind, and their ids in that order.
fn kinds(backend: &crate::backend::Backend) -> Vec<String> {
    use engine::notes::checklist::ChecklistItem;
    use engine::notes::kind::NoteKind;
    use engine::notes::model::NotePatch;
    let scenes: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../golden/fabric/cases.json")).unwrap();
    let scene = scenes[0]["output"].as_str().unwrap().to_owned();
    let shapes = [
        (
            "psql prod",
            NoteKind::Snippet,
            "psql -h {{host}} -U admin\nSELECT count(*) FROM users;",
            vec!["db", "prod", "ops"],
            Vec::new(),
        ),
        (
            "Deploy checklist",
            NoteKind::Note,
            "# Release\n\nShip on Friday.\n\n- build\n- [x] tag\n- [ ] announce",
            vec!["release"],
            Vec::new(),
        ),
        (
            "Groceries",
            NoteKind::Checklist,
            "",
            Vec::new(),
            vec![("milk", true), ("bread", false), ("eggs", false)],
        ),
        (
            "Budget",
            NoteKind::Calc,
            "loyer = 800 €\ncharges = 120,5 €\ntotal",
            Vec::new(),
            Vec::new(),
        ),
        (
            "Sketch",
            NoteKind::Drawing,
            scene.as_str(),
            Vec::new(),
            Vec::new(),
        ),
        (
            "",
            NoteKind::Snippet,
            "curl -s https://example.test/health",
            Vec::new(),
            Vec::new(),
        ),
    ];
    let mut ids = Vec::new();
    for (title, kind, content, tags, items) in shapes {
        let id = crate::testing::note(backend, "seed").id.clone();
        ids.push(id.clone());
        let patch = NotePatch {
            title: Some(title.to_string()),
            kind: Some(kind),
            content: Some(content.to_string()),
            tags: Some(tags.into_iter().map(String::from).collect()),
            items: Some(
                items
                    .into_iter()
                    .map(|(text, done)| ChecklistItem {
                        text: text.to_string(),
                        done,
                    })
                    .collect(),
            ),
            pinned: Some(title == "psql prod"),
            ..NotePatch::default()
        };
        let mut written = backend.db(move |engine| engine::notes::update_note(engine, &id, &patch));
        backend.settle();
        written.poll().unwrap().unwrap();
    }
    ids
}

/// The rail of the samples at the narrowest, the default and a wide window (#1484).
#[test]
fn the_notes_rail_at_three_widths() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    for width in [640.0, 1084.0, 1920.0] {
        harness.set_size(egui::vec2(width, SIZE.y));
        settle(&mut harness);
        capture(&mut harness, &format!("notes-rail-{width}"));
    }
}

/// The editor's header with the pointer resting on an icon (#1488): the pointer is kept.
#[test]
fn the_editor_header_under_the_pointer() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1084.0, 681.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    harness
        .ctx
        .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .click();
    settle(&mut harness);
    for icon in ["editor-fullscreen", "editor-close"] {
        harness
            .get_by(|node| node.author_id() == Some(icon))
            .hover();
        for _ in 0..10 {
            harness.step();
        }
        let image = harness
            .render()
            .expect("a software adapter draws the window");
        if let Some(directory) = std::env::var_os("DEVNOTES_CAPTURES") {
            std::fs::create_dir_all(&directory).unwrap();
            image
                .save(Path::new(&directory).join(format!("hover-{icon}.png")))
                .unwrap();
        }
    }
}

/// The unfolded header of a note that holds everything, at the panel's narrowest and in the
/// narrowest window (#1574).
#[test]
fn the_editor_header_of_a_full_note_when_narrow() {
    use engine::notes::model::{NoteLifecycle, NotePatch};
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1220.0, 681.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = &harness.state().backend;
    let id = crate::testing::note(backend, "Rotation of the staging certificate")
        .id
        .clone();
    let note = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        let space = engine::notes::get_note(engine, &note)?.space_id.clone();
        let folder = engine::folders::create_folder(
            engine,
            &engine::folders::model::FolderDraft {
                space_id: space,
                name: "Certificates and their yearly rotation".to_owned(),
                parent_id: None,
            },
        )?;
        engine::folders::file_notes(
            engine,
            std::slice::from_ref(&note),
            Some(&folder.id),
            engine::folders::board::PreviewLines::default(),
        )?;
        let at = chrono::DateTime::parse_from_rfc3339("2026-12-24T22:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let tags = [
            "infrastructure",
            "certificates",
            "staging",
            "openssl",
            "yearly-rotation-runbook",
        ];
        engine::notes::update_note(
            engine,
            &note,
            &NotePatch {
                source: Some("https://wiki.example.test/runbooks/certificates".to_owned()),
                tags: Some(tags.map(str::to_owned).to_vec()),
                lifecycle: Some(NoteLifecycle::Expires { at }),
                ..NotePatch::default()
            },
        )
    })
    .unwrap();
    let file = profile
        .path()
        .join("the-whole-chain-of-the-staging-certificate-2026.pem");
    std::fs::write(&file, b"-----BEGIN CERTIFICATE-----").unwrap();
    harness.state_mut().state.revision.bump();
    harness.state_mut().state.editor.open(id);
    settle(&mut harness);
    harness.state_mut().state.dialog = std::sync::Arc::new(platform::dialog::Fixed(file));
    click(&mut harness, "attachment-add");
    for width in [1220.0, 1180.0, 640.0] {
        harness.set_size(egui::vec2(width, 681.0));
        settle(&mut harness);
        capture(&mut harness, &format!("editor-header-full-{width}"));
    }
}

/// A Note presented (#1568, #1569): a short slide, a list, a short code, one too long and too
/// wide for the screen.
#[test]
fn the_slides_of_a_presentation() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1600.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let wide = "let answer = client.get(\"https://api.example.test/v1/certificates\").header(\"Authorization\", token).timeout(Duration::from_secs(30)).send().await?;";
    let long: String = std::iter::once(wide.to_owned())
        .chain((1..60).map(|at| format!("let line_{at} = {at};")))
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "## A short slide\n\nOne line of text.\n\n## A list\n\n- prepare the backup\n- switch the DNS\n- watch the logs\n\n## A short code\n\n```rust\nlet x = 1;\nlet y = 2;\n```\n\n## A long code\n\n```rust\n{long}\n```\n"
    );
    let id = crate::testing::note(&harness.state().backend, "Switch over")
        .id
        .clone();
    let patch = engine::notes::model::NotePatch {
        kind: Some(engine::notes::kind::NoteKind::Note),
        content: Some(body),
        ..engine::notes::model::NotePatch::default()
    };
    let engine = harness.state().backend.engine().clone();
    let note = id.clone();
    engine::db::run_here(&engine, move |engine| {
        engine::notes::update_note(engine, &note, &patch)
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    harness.state_mut().state.editor.open(id);
    settle(&mut harness);
    click(&mut harness, "editor-more");
    click(&mut harness, "editor-present");
    settle(&mut harness);
    for slide in ["short", "list", "code", "long-code"] {
        harness.key_press(egui::Key::ArrowRight);
        settle(&mut harness);
        capture(&mut harness, &format!("slide-{slide}"));
    }
}

/// The Format button and its settings (#1594), full screen then beside the cards, on a language
/// Prettier reads and on one it does not.
#[test]
fn the_format_button_and_its_settings() {
    use engine::notes::language::Language;
    for (name, language, code) in [
        ("ts", Language::Ts, "const a = {b:1}"),
        ("sql", Language::Sql, "SELECT 1;"),
    ] {
        let profile = tempfile::tempdir().unwrap();
        let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
        type_into(&mut harness, "vault-passphrase", PASSPHRASE);
        type_into(&mut harness, "vault-confirmation", PASSPHRASE);
        click(&mut harness, "vault-submit");
        settle(&mut harness);
        let id = crate::testing::note(&harness.state().backend, "Invoices")
            .id
            .clone();
        let patch = engine::notes::model::NotePatch {
            content: Some(code.to_owned()),
            language: Some(language),
            ..engine::notes::model::NotePatch::default()
        };
        let engine = harness.state().backend.engine().clone();
        let note = id.clone();
        engine::db::run_here(&engine, move |engine| {
            engine::notes::update_note(engine, &note, &patch)
        })
        .unwrap();
        harness.state_mut().state.revision.bump();
        harness.state_mut().state.editor.open(id);
        settle(&mut harness);
        click(&mut harness, "editor-format-settings");
        capture(&mut harness, &format!("format-panel-{name}"));
        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
        click(&mut harness, "editor-fullscreen");
        capture(&mut harness, &format!("format-full-{name}"));
    }
}

/// Three shapes over three thousand points, an arrow between two of them.
const WIDE_SCENE: &str = r#"{"version":1,"scene":{"objects":[
{"type":"Rect","left":200,"top":150,"width":240,"height":120,"id":"a","hue":"blue","stroke":"x","strokeWidth":2},
{"type":"Ellipse","left":1500,"top":700,"width":400,"height":240,"rx":200,"ry":120,"id":"b","hue":"green","strokeWidth":2},
{"type":"Rect","left":2800,"top":300,"width":300,"height":200,"id":"c","hue":"amber","strokeWidth":2}
]}}"#;

/// The window, a drawing of that scene open in it.
fn with_a_drawing(profile: &std::path::Path, scene: &str) -> Harness<'static, DevNotesApp> {
    let mut harness = launch_at(profile, egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let id = crate::testing::note(&harness.state().backend, "Architecture")
        .id
        .clone();
    let patch = engine::notes::model::NotePatch {
        kind: Some(engine::notes::kind::NoteKind::Drawing),
        content: Some(scene.to_owned()),
        ..engine::notes::model::NotePatch::default()
    };
    let engine = harness.state().backend.engine().clone();
    let note = id.clone();
    engine::db::run_here(&engine, move |engine| {
        engine::notes::update_note(engine, &note, &patch)
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    harness.state_mut().state.editor.open(id);
    settle(&mut harness);
    harness
}

/// A drawing larger than the panel (#1580), as it opens and closer.
#[test]
fn a_drawing_larger_than_the_panel() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = with_a_drawing(profile.path(), WIDE_SCENE);
    capture(&mut harness, "drawing-wide");
}

/// An arrow dragged to the middle of an ellipse's left side (#1582), the points shown.
#[test]
fn an_arrow_near_the_points_of_a_shape() {
    let profile = tempfile::tempdir().unwrap();
    let scene = r#"{"version":1,"scene":{"objects":[
{"type":"Rect","left":160,"top":160,"width":160,"height":90,"id":"a","hue":"blue","strokeWidth":2},
{"type":"Ellipse","left":460,"top":240,"width":160,"height":100,"rx":80,"ry":50,"id":"b","hue":"green","strokeWidth":2}
]}}"#;
    let mut harness = with_a_drawing(profile.path(), scene);
    click(&mut harness, "drawing-tool-arrow");
    let stage = harness
        .get_by(|node| node.author_id() == Some("drawing-stage"))
        .rect();
    harness.drag_at(stage.min + egui::vec2(192.0, 165.0));
    harness.run_steps(2);
    harness.hover_at(stage.min + egui::vec2(332.0, 246.0));
    harness.run_steps(2);
    capture_holding(&mut harness, "drawing-arrow-points");
}

/// A request in the narrowest window (#1487): its URL, its actions and its tabs.
#[test]
fn the_http_request_in_a_narrow_window() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state.http.tabs_mut().edit(&key, |draft| {
        use crate::state::http::tabs::DraftEdit;
        draft.apply(DraftEdit::Url(
            "{{base}}/sessions?user={{user}}&remember=true".to_owned(),
        ));
    });
    for size in [
        egui::vec2(640.0, 480.0),
        egui::vec2(720.0, 480.0),
        egui::vec2(800.0, 600.0),
    ] {
        harness.set_size(size);
        settle(&mut harness);
        capture(&mut harness, &format!("http-request-{}", size.x));
    }
}

/// Two cards ticked at three widths, then a report (#1486): where what acts on a selection
/// stands, what it leaves in view, and where a report is said.
#[test]
fn a_selection_and_a_report_at_three_widths() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1084.0, 681.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    for _ in 0..2 {
        harness.key_press(egui::Key::ArrowRight);
        harness.key_press(egui::Key::X);
    }
    settle(&mut harness);
    for width in [640.0, 1084.0, 1920.0] {
        harness.set_size(egui::vec2(width, 681.0));
        settle(&mut harness);
        capture(&mut harness, &format!("selection-{width}"));
    }
    harness.set_size(egui::vec2(640.0, 681.0));
    settle(&mut harness);
    if harness
        .query_by(|node| node.author_id() == Some("selection-more"))
        .is_some()
    {
        click(&mut harness, "selection-more");
        settle(&mut harness);
        capture(&mut harness, "selection-more-640");
        harness.key_press(egui::Key::Escape);
    }
    harness.key_press(egui::Key::Escape);
    harness.set_size(egui::vec2(1084.0, 681.0));
    settle(&mut harness);
    harness
        .state_mut()
        .state
        .status
        .report(i18n::Message::new(i18n::keys::tools::SAMPLE_LOADED));
    settle(&mut harness);
    capture(&mut harness, "status");
}

/// The samples' tree with a chain of folders four levels deep, at the default rail (#1463).
#[test]
fn the_tag_manager_with_two_hundred_tags() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let engine = harness.state().backend.engine().clone();
    let tags: Vec<String> = (0..200).map(|rank| format!("service-{rank:03}")).collect();
    for (rank, tags) in tags.chunks(10).enumerate() {
        let id = crate::testing::note(&harness.state().backend, &format!("note {rank}"))
            .id
            .clone();
        let patch = engine::notes::model::NotePatch {
            tags: Some(tags.to_vec()),
            ..engine::notes::model::NotePatch::default()
        };
        engine::db::run_here(&engine, move |engine| {
            engine::notes::update_note(engine, &id, &patch)
        })
        .unwrap();
    }
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    click(&mut harness, "facets-toggle");
    click(&mut harness, "tag-manage");
    settle(&mut harness);
    capture(&mut harness, "tag-manager-many");
}

#[test]
fn the_folders_four_levels_deep() {
    use engine::folders::model::FolderDraft;
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1084.0, 681.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = &harness.state().backend;
    engine::db::run_here(backend.engine(), |engine| {
        let folders = engine::folders::list_folders(engine, None)?;
        let snippets = folders
            .iter()
            .find(|folder| folder.name == "Snippets")
            .unwrap();
        let mut parent = snippets.id.clone();
        for name in ["Requêtes SQL", "Exploitation", "Lecture seule", "Rapports"] {
            let draft = FolderDraft {
                space_id: snippets.space_id.clone(),
                name: name.to_owned(),
                parent_id: Some(parent),
            };
            parent = engine::folders::create_folder(engine, &draft)?.id;
        }
        Ok::<_, engine::error::AppError>(())
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    capture(&mut harness, "folders-four-deep");
}

// Dropdowns of short entries and of long ones, at the width they open (#1459).
#[test]
fn the_dropdowns_at_their_width() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    click(&mut harness, "file-menu");
    capture(&mut harness, "menu-file");
    click(&mut harness, "file-menu");
    click(&mut harness, "sort-trigger");
    capture(&mut harness, "menu-sort");
    click(&mut harness, "sort-trigger");
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .click();
    settle(&mut harness);
    for menu in ["editor-kind", "editor-priority", "editor-language"] {
        click(&mut harness, menu);
        capture(&mut harness, &format!("menu-{menu}"));
        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
    }
    drop(harness);

    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "http-method");
    capture(&mut harness, "menu-http-method");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "http-section-auth");
    click(&mut harness, "http-auth-kind");
    capture(&mut harness, "menu-http-auth-kind");
}

/// A long Note holding a code block, open beside the cards and scrolled down (#1464).
#[test]
fn a_long_note_with_code_scrolled_in_the_panel() {
    use engine::notes::kind::NoteKind;
    use engine::notes::language::Language;
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let backend = &harness.state().backend;
    let space = crate::testing::note(backend, "seed").space_id.clone();
    let code = (0..60)
        .map(|n| format!("SELECT * FROM orders WHERE id = {n};"))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("# Requêtes\n\nCe qu'on lance le lundi.\n\n```sql\n{code}\n```\n");
    engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Note,
            language: Language::Txt,
            content,
            ..crate::testing::snippet(space, "Requêtes du lundi".to_owned())
        };
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("Requêtes du lundi")
        })
        .click();
    settle(&mut harness);
    let body = harness
        .get_by(|node| node.author_id() == Some("editor-rich"))
        .rect();
    harness.event(egui::Event::PointerMoved(body.center()));
    for _ in 0..6 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
    }
    settle(&mut harness);
    capture(&mut harness, "note-code-scrolled");
}

/// The request's variables under its URL, one pointed at: a value, a secret, one set nowhere
/// (#1457).
#[test]
fn a_variable_pointed_at() {
    use engine::http::environments::{Variable, VariableKind};
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    let global = |name: &str, value: &str, kind| Variable {
        enabled: true,
        name: name.to_owned(),
        value: value.to_owned(),
        kind,
        filled_by: None,
    };
    let globals = [
        global("base", "https://api.example.test", VariableKind::Text),
        global("token", "s3cr3t-t0k3n", VariableKind::Secret),
    ];
    engine::db::run_here(harness.state().backend.engine(), |engine| {
        engine::http::save_http_globals(engine, &globals)
    })
    .unwrap();
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state.http.tabs_mut().edit(&key, |draft| {
        use crate::state::http::tabs::DraftEdit;
        draft.apply(DraftEdit::Url(
            "{{base}}/sessions/{{token}}?user={{user}}".to_owned(),
        ));
    });
    settle(&mut harness);
    for (at, name) in ["base", "token", "user"].into_iter().enumerate() {
        let chip = harness
            .get_all_by(|node| node.author_id() == Some("http-used-variable"))
            .nth(at)
            .unwrap()
            .rect();
        harness.event(egui::Event::PointerMoved(chip.center()));
        for _ in 0..60 {
            harness.step();
        }
        let image = harness
            .render()
            .expect("a software adapter draws the window");
        if let Some(directory) = std::env::var_os("DEVNOTES_CAPTURES") {
            std::fs::create_dir_all(&directory).unwrap();
            let file = format!("variable-{name}.png");
            image.save(Path::new(&directory).join(file)).unwrap();
        }
    }
}

/// Four tabs, the first held between the last two, then let go there (#1506).
#[test]
fn an_http_tab_dragged_along_the_strip() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = http_area(profile.path());
    for name in ["Login", "Session", "Factures", "Flux"] {
        harness
            .get_by(|node| {
                node.author_id() == Some("http-node-name")
                    && node.label().is_some_and(|label| label.ends_with(name))
            })
            .click();
        settle(&mut harness);
    }
    let tab = |harness: &Harness<'_, DevNotesApp>, name: &str| {
        harness
            .get_by(|node| {
                node.author_id() == Some("http-tab-open")
                    && node.label().is_some_and(|label| label.ends_with(name))
            })
            .rect()
    };
    let from = tab(&harness, "Login").center();
    let to = tab(&harness, "Flux").left_center() + egui::vec2(4.0, 0.0);
    let press = |pressed| egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
    for step in 1..=10_u8 {
        harness.event(egui::Event::PointerMoved(
            from + (to - from) * (f32::from(step) / 10.0),
        ));
        harness.step();
    }
    capture_holding(&mut harness, "http-tab-dragging");
    harness.event(press(false));
    harness.event(egui::Event::PointerMoved(egui::pos2(700.0, 500.0)));
    settle(&mut harness);
    capture(&mut harness, "http-tab-moved");
}

/// The task lists' cards, a list longer than its card among them (#1461).
#[test]
fn the_task_lists_and_their_progress() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch_at(profile.path(), egui::vec2(1440.0, 900.0));
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let _ = kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    capture(&mut harness, "task-lists");
}

/// The samples' « Snippets » folder dropped onto another space (#1503).
#[test]
fn a_folder_dropped_onto_another_space() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    let draft = engine::spaces::model::SpaceDraft {
        name: "Archives".into(),
    };
    engine::db::run_here(harness.state().backend().engine(), move |engine| {
        engine::spaces::create_space(engine, &draft)
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    let at = |harness: &Harness<'_, DevNotesApp>, author_id: &str, name: &str| {
        harness
            .get_by(|node| {
                node.author_id() == Some(author_id) && node.label().as_deref() == Some(name)
            })
            .rect()
            .center()
    };
    let from = at(&harness, "folder-option", "Snippets");
    let to = at(&harness, "space-option", "Archives");
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.step();
    harness.event(press(from, true));
    harness.step();
    for step in [0.25, 0.5, 0.75, 1.0] {
        harness.event(egui::Event::PointerMoved(from + (to - from) * step));
        harness.step();
    }
    harness.event(press(to, false));
    settle(&mut harness);
    capture(&mut harness, "folder-to-space");
}
