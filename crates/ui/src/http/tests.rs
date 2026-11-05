//! The HTTP area over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::model::{
    HttpCollection, HttpFolder, HttpMethod, HttpRequest, HttpRequestDraft, RequestDocument,
    RequestKind,
};

use crate::backend::Backend;
use crate::notes::bench::{Bench, area_of, names, remembered, settle};
use crate::state::Area;
use crate::state::http::COLLAPSED_KEY;
use crate::testing;

pub(crate) fn collection(backend: &Backend, name: &str) -> HttpCollection {
    let name = name.to_owned();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::http::create_http_collection(engine, name)
    })
    .unwrap()
}

pub(crate) fn folder(backend: &Backend, collection: &HttpCollection, name: &str) -> HttpFolder {
    let (id, name) = (collection.id.clone(), name.to_owned());
    engine::db::run_here(backend.engine(), move |engine| {
        engine::http::create_http_folder(engine, id, None, name)
    })
    .unwrap()
}

pub(crate) fn request(
    backend: &Backend,
    (collection, folder): (&HttpCollection, Option<&HttpFolder>),
    name: &str,
    (kind, method): (RequestKind, HttpMethod),
) -> HttpRequest {
    let draft = HttpRequestDraft {
        collection_id: collection.id.clone(),
        folder_id: folder.map(|folder| folder.id.clone()),
        name: name.to_owned(),
        kind,
        method,
        document: RequestDocument {
            url: format!("https://api.example.com/{name}"),
            ..RequestDocument::default()
        },
    };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::http::create_http_request(engine, draft)
    })
    .unwrap()
}

/// `Paiements[Auth[Login] Socket]`, the HTTP area shown.
pub(crate) fn paiements() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Paiements");
    let auth = folder(&backend, &api, "Auth");
    let http = (RequestKind::Http, HttpMethod::Post);
    request(&backend, (&api, Some(&auth)), "Login", http);
    let socket = (RequestKind::Websocket, HttpMethod::Get);
    request(&backend, (&api, None), "Socket", socket);
    area_of(Area::Http, backend, profile)
}

fn click_named(harness: &mut Harness<'_, Bench>, author_id: &str, name: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .click();
    settle(harness);
}

#[test]
fn the_rail_lays_the_tree_flat_with_each_request_after_its_method() {
    let harness = paiements();

    assert_eq!(
        names(&harness, "http-node-name"),
        ["Paiements", "Auth", "POST Login", "WS Socket"]
    );
    assert_eq!(
        harness
            .query_all_by(|node| node.author_id() == Some("http-node-twisty"))
            .count(),
        2
    );
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-rail-empty"))
            .is_none()
    );
}

#[test]
fn a_folder_folds_on_its_twisty_and_on_its_name_and_is_remembered() {
    let mut harness = paiements();

    click_named(&mut harness, "http-node-twisty", "Collapse Auth");

    assert_eq!(
        names(&harness, "http-node-name"),
        ["Paiements", "Auth", "WS Socket"]
    );
    let folded = harness.get_by(|node| {
        node.author_id() == Some("http-node-name") && node.label().as_deref() == Some("Auth")
    });
    assert_eq!(folded.accesskit_node().data().is_expanded(), Some(false));
    let auth = engine::db::run_here(harness.state().backend.engine(), engine::http::http_tree)
        .unwrap()
        .collections[0]
        .children
        .iter()
        .find_map(|node| match node {
            engine::http::model::HttpNode::Folder { folder, .. } => Some(folder.id.clone()),
            engine::http::model::HttpNode::Request { .. } => None,
        })
        .unwrap();
    settle(&mut harness);
    assert_eq!(
        remembered(&harness, COLLAPSED_KEY),
        Some(format!(r#"["{auth}"]"#))
    );

    click_named(&mut harness, "http-node-name", "Paiements");
    assert_eq!(names(&harness, "http-node-name"), ["Paiements"]);
}

#[test]
fn an_empty_library_says_it_has_no_collection() {
    let (backend, profile) = testing::unlocked();
    let harness = area_of(Area::Http, backend, profile);

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-rail-empty"))
            .is_some()
    );
    assert_eq!(names(&harness, "http-node-name"), Vec::<String>::new());
}

#[test]
fn ctrl_b_puts_the_rail_away_and_brings_it_back() {
    let mut harness = paiements();

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::B);
    settle(&mut harness);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-node-name"))
            .is_none()
    );

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::B);
    settle(&mut harness);
    assert_eq!(names(&harness, "http-node-name").len(), 4);
}

/// 2,000 requests in folders of fifty.
fn thousands(backend: &Backend) {
    let api = collection(backend, "Corpus");
    for at in 0..40 {
        let folder = folder(backend, &api, &format!("folder {at}"));
        for index in 0..50 {
            let name = format!("request {at}-{index}");
            request(
                backend,
                (&api, Some(&folder)),
                &name,
                (RequestKind::Http, HttpMethod::Get),
            );
        }
    }
}

#[test]
fn only_the_rows_in_view_of_two_thousand_requests_are_drawn() {
    let (backend, profile) = testing::unlocked();
    thousands(&backend);
    let harness = area_of(Area::Http, backend, profile);

    let drawn = harness
        .query_all_by(|node| node.author_id() == Some("http-node-name"))
        .count();
    assert!(drawn > 10 && drawn < 60, "{drawn} rows drawn");
    assert_eq!(harness.state().state.http.rows().len(), 2_041);
}

/// The budget of § 6.2, measured by hand: `cargo test -p ui --release http_rail_budget --
/// --ignored --nocapture`. A frame is egui's pass, without painting.
#[test]
#[ignore = "a measure, not a check"]
fn http_rail_budget() {
    let (backend, profile) = testing::unlocked();
    thousands(&backend);
    let started = std::time::Instant::now();
    let tree = engine::db::run_here(backend.engine(), engine::http::http_tree).unwrap();
    let rows = crate::state::http::tree::rail_rows(&tree, &std::collections::BTreeSet::new());
    println!(
        "2,000 requests: read and laid out in {:?} ({} rows)",
        started.elapsed(),
        rows.len()
    );
    let mut harness = area_of(Area::Http, backend, profile);
    let started = std::time::Instant::now();
    click_named(&mut harness, "http-node-name", "folder 0");
    println!("a fold: {:?}", started.elapsed());
    let centre = harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name") && node.label().as_deref() == Some("Corpus")
        })
        .rect()
        .center();
    harness.event(egui::Event::PointerMoved(centre));
    let mut frames = Vec::new();
    for _ in 0..200 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -80.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        let started = std::time::Instant::now();
        harness.step();
        frames.push(started.elapsed());
    }
    frames.sort();
    println!(
        "scrolling p50 {:?}, p95 {:?}, p99 {:?}",
        frames[100], frames[190], frames[198]
    );
}

fn click_id(harness: &mut Harness<'_, Bench>, author_id: &str) {
    crate::notes::bench::click_id(harness, author_id);
}

/// A row's `⋯`, then an entry of its menu, a sub-menu's through its parent.
fn choose(harness: &mut Harness<'_, Bench>, row: &str, entries: &[&str]) {
    click_named(harness, "http-node-menu", &format!("Options for {row}"));
    for entry in entries {
        click_id(harness, entry);
    }
}

/// What a label says: egui puts a label's words in its value.
fn text(harness: &Harness<'_, Bench>, author_id: &str) -> String {
    let node = harness.get_by(|node| node.author_id() == Some(author_id));
    let node = node.accesskit_node();
    node.label().or(node.value()).unwrap_or_default()
}

fn said(harness: &Harness<'_, Bench>) -> String {
    text(harness, "http-rail-moved")
}

fn focus_row(harness: &mut Harness<'_, Bench>, name: &str) {
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name") && node.label().as_deref() == Some(name)
        })
        .focus();
    settle(harness);
}

#[test]
fn the_first_collection_is_offered_from_the_empty_rail() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Http, backend, profile);

    click_id(&mut harness, "http-rail-empty-create");
    crate::notes::bench::type_and_enter(&mut harness, "http-create-input", "API");

    assert_eq!(names(&harness, "http-node-name"), ["API"]);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-rail-empty"))
            .is_none()
    );
}

#[test]
fn a_folder_and_a_request_are_made_from_a_row_s_menu() {
    let mut harness = paiements();

    choose(&mut harness, "Paiements", &["http-node-new-folder"]);
    crate::notes::bench::type_and_enter(&mut harness, "http-create-input", "Factures");
    choose(
        &mut harness,
        "Factures",
        &["http-node-new", "http-node-new-graphql"],
    );
    crate::notes::bench::type_and_enter(&mut harness, "http-create-input", "Lister");

    assert_eq!(
        names(&harness, "http-node-name"),
        [
            "Paiements",
            "Auth",
            "POST Login",
            "WS Socket",
            "Factures",
            "GQL Lister"
        ]
    );
    assert_eq!(harness.state().state.http.tabs().tabs().len(), 1);
}

#[test]
fn a_name_left_is_kept_and_escape_writes_nothing() {
    let mut harness = paiements();

    focus_row(&mut harness, "WS Socket");
    harness.key_press(egui::Key::F2);
    settle(&mut harness);
    harness
        .get_by(|node| node.author_id() == Some("http-rename-input"))
        .type_text(" live");
    settle(&mut harness);
    click_id(&mut harness, "http-workspace-empty");
    crate::notes::bench::until(&mut harness, "the rename is read back", |harness| {
        names(harness, "http-node-name").contains(&"WS Socket live".to_owned())
    });

    choose(&mut harness, "Auth", &["http-node-rename"]);
    harness
        .get_by(|node| node.author_id() == Some("http-rename-input"))
        .type_text("ignored");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(names(&harness, "http-node-name").contains(&"Auth".to_owned()));
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-rename-input"))
            .is_none()
    );
}

#[test]
fn a_deletion_says_what_goes_with_it_and_cancel_keeps_it() {
    let mut harness = paiements();

    choose(&mut harness, "Paiements", &["http-node-delete"]);
    assert_eq!(
        text(&harness, "http-delete-count"),
        "1 folder and 2 requests go with it."
    );
    click_id(&mut harness, "http-delete-cancel");
    assert_eq!(names(&harness, "http-node-name").len(), 4);

    focus_row(&mut harness, "POST Login");
    harness.key_press(egui::Key::Delete);
    settle(&mut harness);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-delete-count"))
            .is_none()
    );
    click_id(&mut harness, "http-delete-submit");
    assert_eq!(
        names(&harness, "http-node-name"),
        ["Paiements", "Auth", "WS Socket"]
    );
}

#[test]
fn a_duplicate_is_named_a_copy() {
    let mut harness = paiements();

    choose(&mut harness, "Socket", &["http-node-duplicate"]);

    assert!(names(&harness, "http-node-name").contains(&"WS Socket (copy)".to_owned()));
}

#[test]
fn a_move_with_the_keys_is_made_shown_and_said() {
    let mut harness = paiements();

    focus_row(&mut harness, "WS Socket");
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowRight);
    settle(&mut harness);
    crate::notes::bench::until(&mut harness, "the move is said", |harness| {
        said(harness) == "Socket is now in Auth."
    });
    assert_eq!(
        names(&harness, "http-node-name"),
        ["Paiements", "Auth", "POST Login", "WS Socket"]
    );
    assert!(
        harness
            .get_by(|node| node.author_id() == Some("http-node-name")
                && node.label().as_deref() == Some("WS Socket"))
            .accesskit_node()
            .is_focused()
    );

    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(said(&harness), "Socket is already last.");
}

#[test]
fn a_row_dragged_mid_folder_lands_inside_it() {
    let mut harness = paiements();
    let rect = |harness: &Harness<'_, Bench>, name: &str| {
        harness
            .get_by(|node| {
                node.author_id() == Some("http-node-name") && node.label().as_deref() == Some(name)
            })
            .rect()
    };
    let from = rect(&harness, "WS Socket").center();
    let to = rect(&harness, "Auth").center();

    harness.event(egui::Event::PointerMoved(from));
    harness.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
    for step in 1..=10_u8 {
        let at = from + (to - from) * (f32::from(step) / 10.0);
        harness.event(egui::Event::PointerMoved(at));
        harness.step();
    }
    harness.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut harness);

    crate::notes::bench::until(&mut harness, "the row lands in the folder", |harness| {
        names(harness, "http-node-name") == ["Paiements", "Auth", "POST Login", "WS Socket"]
    });
}

#[test]
fn a_row_clicked_keeps_the_keys_through_two_moves() {
    let mut harness = paiements();

    click_named(&mut harness, "http-node-name", "WS Socket");
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
    crate::notes::bench::until(&mut harness, "the socket is first", |harness| {
        names(harness, "http-node-name") == ["Paiements", "WS Socket", "Auth", "POST Login"]
    });
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    crate::notes::bench::until(&mut harness, "the socket is back last", |harness| {
        names(harness, "http-node-name") == ["Paiements", "Auth", "POST Login", "WS Socket"]
    });
    let focused = harness
        .query_by(|node| node.is_focused() && node.author_id() == Some("http-node-name"))
        .map(|node| node.accesskit_node().label());
    assert_eq!(focused, Some(Some("WS Socket".to_owned())));
}

fn tab_names(harness: &Harness<'_, Bench>) -> Vec<String> {
    names(harness, "http-tab-open")
}

#[test]
fn a_request_opens_in_its_tab_and_its_row_is_lit() {
    let mut harness = paiements();

    click_named(&mut harness, "http-node-name", "POST Login");
    crate::notes::bench::until(&mut harness, "the tab opens", |harness| {
        tab_names(harness) == ["POST Login"]
    });
    let lit = harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().as_deref() == Some("POST Login")
        })
        .accesskit_node()
        .toggled();
    assert_eq!(lit, Some(egui::accesskit::Toggled::True));
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-workspace-empty"))
            .is_none()
    );

    click_named(&mut harness, "http-node-name", "WS Socket");
    click_named(&mut harness, "http-node-name", "POST Login");
    assert_eq!(tab_names(&harness), ["POST Login", "WS Socket"]);
    assert_eq!(
        harness
            .state()
            .state
            .http
            .tabs()
            .active()
            .map(|tab| tab.draft.name.as_str()),
        Some("Login")
    );
}

#[test]
fn a_new_request_is_begun_from_the_empty_workspace_and_the_tabs() {
    let mut harness = paiements();

    click_id(&mut harness, "http-workspace-new-graphql");
    click_id(&mut harness, "http-tab-new");

    assert_eq!(
        tab_names(&harness),
        ["GQL New request, modified", "GET New request, modified"]
    );
    click_id(&mut harness, "http-tab-new-kind");
    click_id(&mut harness, "http-new-kind-websocket");
    assert_eq!(tab_names(&harness).len(), 3);
    assert!(tab_names(&harness)[2].starts_with("WS"));
}

#[test]
fn a_modified_tab_asks_before_closing_and_a_clean_one_does_not() {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "POST Login");
    click_named(&mut harness, "http-node-name", "WS Socket");
    crate::notes::bench::until(&mut harness, "two tabs", |harness| {
        tab_names(harness).len() == 2
    });

    click_named(&mut harness, "http-tab-close", "Close Socket");
    assert_eq!(tab_names(&harness), ["POST Login"]);

    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active_key()
        .unwrap()
        .to_owned();
    harness
        .state_mut()
        .state
        .http
        .tabs_mut()
        .edit(&key, |draft| draft.method = HttpMethod::Put);
    settle(&mut harness);
    click_named(&mut harness, "http-tab-close", "Close Login");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-close-modified"))
            .is_some()
    );
    click_id(&mut harness, "http-close-cancel");
    assert_eq!(tab_names(&harness), ["PUT Login, modified"]);

    click_named(&mut harness, "http-tab-close", "Close Login");
    click_id(&mut harness, "http-close-discard");
    assert!(tab_names(&harness).is_empty());
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-workspace-empty"))
            .is_some()
    );
}

#[test]
fn ctrl_s_saves_the_tab_in_front_and_the_rail_reads_it_back() {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "POST Login");
    crate::notes::bench::until(&mut harness, "the tab opens", |harness| {
        tab_names(harness).len() == 1
    });
    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active_key()
        .unwrap()
        .to_owned();
    harness
        .state_mut()
        .state
        .http
        .tabs_mut()
        .edit(&key, |draft| draft.method = HttpMethod::Delete);
    settle(&mut harness);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    crate::notes::bench::until(&mut harness, "the save is read back", |harness| {
        tab_names(harness) == ["DEL Login"]
            && names(harness, "http-node-name").contains(&"DEL Login".to_owned())
    });
}

#[test]
fn the_tabs_left_open_are_reopened_and_a_deleted_request_closes_its_tab() {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "POST Login");
    click_named(&mut harness, "http-node-name", "WS Socket");
    crate::notes::bench::until(&mut harness, "two tabs", |harness| {
        tab_names(harness).len() == 2
    });
    click_named(&mut harness, "http-tab-open", "POST Login");
    let stored = remembered(&harness, crate::state::http::tabs::TABS_KEY).unwrap();

    let mut state =
        crate::state::AppState::new(crate::services::preferences::Preferences::default(), true);
    let mut preferences = crate::services::preferences::Preferences::default();
    preferences.set(crate::state::http::tabs::TABS_KEY, &stored);
    state.http.restore(&preferences);
    harness.state_mut().state.http = state.http;
    crate::notes::bench::until(&mut harness, "the tabs are reopened", |harness| {
        tab_names(harness) == ["POST Login", "WS Socket"]
    });
    assert_eq!(
        harness
            .state()
            .state
            .http
            .tabs()
            .active()
            .map(|tab| tab.draft.name.as_str()),
        Some("Login")
    );

    choose(&mut harness, "Socket", &["http-node-delete"]);
    click_id(&mut harness, "http-delete-submit");
    crate::notes::bench::until(&mut harness, "its tab closes", |harness| {
        tab_names(harness) == ["POST Login"]
    });
}

#[test]
fn a_deletion_names_the_open_tabs_it_would_lose() {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "POST Login");
    crate::notes::bench::until(&mut harness, "the tab opens", |harness| {
        tab_names(harness).len() == 1
    });
    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active_key()
        .unwrap()
        .to_owned();
    harness
        .state_mut()
        .state
        .http
        .tabs_mut()
        .edit(&key, |draft| draft.name = "Login v2".to_owned());
    settle(&mut harness);

    choose(&mut harness, "Auth", &["http-node-delete"]);

    assert_eq!(names(&harness, "http-delete-unsaved-name").len(), 1);
    assert!(text(&harness, "http-delete-unsaved-name").contains("Login v2"));
}

/// `Login` open in its tab.
fn editing() -> Harness<'static, Bench> {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "POST Login");
    crate::notes::bench::until(&mut harness, "the editor is open", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-url"))
            .is_some()
    });
    harness
}

/// What the fields wearing `author_id` hold, top to bottom.
fn values(harness: &Harness<'_, Bench>, author_id: &str) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .map(|node| node.accesskit_node().value().unwrap_or_default())
        .collect()
}

fn active(harness: &Harness<'_, Bench>) -> crate::state::http::tabs::Draft {
    harness
        .state()
        .state
        .http
        .tabs()
        .active()
        .unwrap()
        .draft
        .clone()
}

fn type_into(harness: &mut Harness<'_, Bench>, author_id: &str, nth: usize, words: &str) {
    let node = harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .nth(nth)
        .unwrap();
    node.focus();
    settle(harness);
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .nth(nth)
        .unwrap()
        .type_text(words);
    settle(harness);
}

#[test]
fn the_url_and_the_parameters_follow_each_other() {
    let mut harness = editing();

    type_into(&mut harness, "http-url", 0, "?page=2&q=x");
    let draft = active(&harness);
    assert_eq!(
        draft.document.url,
        "https://api.example.com/Login?page=2&q=x"
    );
    assert_eq!(values(&harness, "http-kv-key"), ["page", "q", ""]);

    harness
        .query_all_by(|node| node.author_id() == Some("http-kv-enabled"))
        .next()
        .unwrap()
        .click();
    settle(&mut harness);
    assert_eq!(
        active(&harness).document.url,
        "https://api.example.com/Login?q=x"
    );
    assert!(names(&harness, "http-tab-open")[0].ends_with("modified"));
}

#[test]
fn a_header_typed_with_its_value_is_split_and_a_bad_name_is_said() {
    let mut harness = editing();
    click_id(&mut harness, "http-section-headers");
    assert!(
        values(&harness, "http-implied-value")
            .iter()
            .chain(&names(&harness, "http-implied-value"))
            .any(|value| value.starts_with("DevNotes/"))
    );

    type_into(&mut harness, "http-kv-key", 0, "Accept: application/json");
    let headers = active(&harness).document.headers;
    assert_eq!(headers[0].key, "Accept");
    assert_eq!(headers[0].value, "application/json");

    type_into(&mut harness, "http-kv-key", 1, "X Trace");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-kv-fault"))
            .is_some()
    );
}

#[test]
fn lines_pasted_in_a_name_fill_a_row_each() {
    let mut harness = editing();
    click_id(&mut harness, "http-section-headers");
    let key = harness
        .query_all_by(|node| node.author_id() == Some("http-kv-key"))
        .next()
        .unwrap();
    key.focus();
    settle(&mut harness);

    harness.event(egui::Event::Paste("Accept: */*\nX-Trace: on\n".to_owned()));
    settle(&mut harness);

    let keys: Vec<String> = active(&harness)
        .document
        .headers
        .iter()
        .map(|row| row.key.clone())
        .collect();
    assert_eq!(keys, ["Accept", "X-Trace"]);
}

#[test]
fn a_kind_is_said_where_a_method_would_be_chosen() {
    let mut harness = paiements();
    click_named(&mut harness, "http-node-name", "WS Socket");
    crate::notes::bench::until(&mut harness, "the editor is open", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-kind-badge"))
            .is_some()
    });

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-method"))
            .is_none()
    );
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-section-params"))
            .is_none()
    );
}

#[test]
fn a_variable_is_listed_with_where_it_resolves_from() {
    let mut harness = editing();

    type_into(&mut harness, "http-url", 0, "/{{token}}");
    eprintln!(
        "URL {:?} USED {:?}",
        active(&harness).document.url,
        harness.state().state.http.used().len()
    );

    crate::notes::bench::until(&mut harness, "the variable is listed", |harness| {
        harness
            .query_all_by(|node| node.author_id() == Some("http-used-variable"))
            .map(|node| {
                let node = node.accesskit_node();
                node.label().or(node.value()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            == ["{{token}}"]
    });
}

/// A request to the local server at `path`, open in its tab.
fn sending_to(path: &str) -> Harness<'static, Bench> {
    let port = testing::serve();
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Local");
    let draft = HttpRequestDraft {
        collection_id: api.id.clone(),
        folder_id: None,
        name: "Ping".to_owned(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        document: RequestDocument {
            url: format!("http://127.0.0.1:{port}{path}"),
            ..RequestDocument::default()
        },
    };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::http::create_http_request(engine, draft)
    })
    .unwrap();
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(egui::vec2(1_100.0, 900.0));
    click_named(&mut harness, "http-node-name", "GET Ping");
    crate::notes::bench::until(&mut harness, "the editor is open", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-send"))
            .is_some()
    });
    harness
}

fn said_by(harness: &Harness<'_, Bench>, author_id: &str) -> Option<String> {
    let node = harness.query_by(|node| node.author_id() == Some(author_id))?;
    let node = node.accesskit_node();
    node.label().or(node.value())
}

fn answered(harness: &mut Harness<'_, Bench>) {
    crate::notes::bench::until(harness, "the answer is shown", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-response-status"))
            .is_some()
    });
}

#[test]
fn a_request_is_sent_and_its_answer_read_part_by_part() {
    let mut harness = sending_to("/json");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-response-idle"))
            .is_some()
    );

    click_id(&mut harness, "http-send");
    answered(&mut harness);

    assert_eq!(
        said_by(&harness, "http-response-status").as_deref(),
        Some("200 OK")
    );
    assert!(said_by(&harness, "http-response-time").is_some_and(|time| time.ends_with("ms")));
    assert!(said_by(&harness, "http-response-pretty").is_some_and(|body| body.contains("\"Ada\"")));
    click_id(&mut harness, "http-response-view-1");
    assert!(
        said_by(&harness, "http-response-body").is_some_and(|body| body.starts_with("{\"name\""))
    );

    click_id(&mut harness, "http-response-section-cookies");
    assert_eq!(
        said_by(&harness, "http-response-cookie").as_deref(),
        Some("session")
    );
    click_id(&mut harness, "http-response-section-timeline");
    assert!(
        said_by(&harness, "http-response-sent")
            .is_some_and(|sent| sent.starts_with("GET http://127.0.0.1"))
    );
    assert!(
        said_by(&harness, "http-response-received").is_some_and(|got| got.starts_with("200 OK"))
    );

    click_id(&mut harness, "http-response-clear");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-response-idle"))
            .is_some()
    );
}

#[test]
fn a_json_answer_is_explored_as_a_tree() {
    let mut harness = sending_to("/json");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    answered(&mut harness);

    click_id(&mut harness, "http-response-view-3");

    crate::notes::bench::until(&mut harness, "the tree is drawn", |harness| {
        harness
            .query_all_by(|node| {
                node.label()
                    .is_some_and(|label| label.contains("languages"))
            })
            .count()
            > 0
    });
}

#[test]
fn a_send_under_way_counts_its_time_and_can_be_cancelled() {
    let mut harness = sending_to("/slow");

    // Not settled: settling waits for the core, and the send with it.
    harness
        .get_by(|node| node.author_id() == Some("http-send"))
        .click();
    harness.step();
    harness.step();
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-response-sending"))
            .is_some()
    );
    assert!(said_by(&harness, "http-response-elapsed").is_some_and(|time| time.ends_with(" s")));
    let http = &harness.state().state.http;
    assert!(http.sends().is_sending(http.tabs().active_key().unwrap()));
    // A cancel that arrives before the send has left cancels nothing: it reaches the server first.
    let url = active(&harness).document.url;
    let port: u16 = url["http://127.0.0.1:".len()..url.rfind('/').unwrap()]
        .parse()
        .unwrap();
    for _ in 0..10_000 {
        if testing::slow_arrived(port) {
            break;
        }
        harness.step();
    }
    assert!(testing::slow_arrived(port), "the send reached the server");

    harness
        .get_by(|node| node.author_id() == Some("http-send"))
        .click();
    harness.step();
    harness.step();
    crate::notes::bench::until(&mut harness, "the send is cancelled", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-response-cancelled"))
            .is_some()
    });
}

#[test]
fn a_status_says_its_family_and_a_failure_says_why() {
    let mut harness = sending_to("/missing");
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    assert_eq!(
        said_by(&harness, "http-response-status").as_deref(),
        Some("404 Not Found")
    );

    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active_key()
        .unwrap()
        .to_owned();
    harness
        .state_mut()
        .state
        .http
        .tabs_mut()
        .edit(&key, |draft| {
            draft.apply(crate::state::http::tabs::DraftEdit::Url(
                "http://127.0.0.1:1/".to_owned(),
            ));
        });
    settle(&mut harness);
    click_id(&mut harness, "http-send");
    crate::notes::bench::until(&mut harness, "the failure is said", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-response-failure"))
            .is_some()
    });
}

#[test]
fn the_line_between_request_and_answer_is_left_automatic_on_a_double_click() {
    let mut harness = sending_to("/json");
    harness.state_mut().state.settings.http_request_share = 500;
    settle(&mut harness);

    harness
        .get_by(|node| node.author_id() == Some("http-split-handle"))
        .focus();
    settle(&mut harness);
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);

    assert_eq!(harness.state().state.settings.http_request_share, 0);
}

/// The budget of § 6.2, measured by hand: `cargo test -p ui --release http_body_budget --
/// --ignored --nocapture`. A frame is egui's pass, without painting.
#[test]
#[ignore = "a measure, not a check"]
fn http_body_budget() {
    let mut harness = sending_to("/big");
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    let size = said_by(&harness, "http-response-size").unwrap_or_default();
    let started = std::time::Instant::now();
    click_id(&mut harness, "http-response-view-1");
    click_id(&mut harness, "http-response-view-0");
    println!("{size}: laid out again in {:?}", started.elapsed());
    let centre = harness
        .get_by(|node| node.author_id() == Some("http-response-pretty"))
        .rect()
        .center();
    harness.event(egui::Event::PointerMoved(centre));
    let mut frames = Vec::new();
    for _ in 0..200 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        let started = std::time::Instant::now();
        harness.step();
        frames.push(started.elapsed());
    }
    frames.sort();
    println!(
        "scrolling p50 {:?}, p95 {:?}, p99 {:?}",
        frames[100], frames[190], frames[198]
    );
}

#[test]
fn the_headers_and_a_redirect_followed_are_read_from_the_answer() {
    let mut harness = sending_to("/redirect");
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    assert_eq!(
        said_by(&harness, "http-response-status").as_deref(),
        Some("200 OK")
    );

    click_id(&mut harness, "http-response-section-headers");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-response-headers"))
            .is_some()
    );
    click_id(&mut harness, "http-response-section-timeline");
    assert!(
        said_by(&harness, "http-response-redirect").is_some_and(|line| line.starts_with("302"))
    );
}

#[test]
fn a_body_sent_is_shown_in_the_timeline() {
    let mut harness = sending_to("/json");
    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active_key()
        .unwrap()
        .to_owned();
    harness
        .state_mut()
        .state
        .http
        .tabs_mut()
        .edit(&key, |draft| {
            draft.method = HttpMethod::Post;
            draft.document.body = engine::http::model::RequestBody::Json {
                text: r#"{"login":"ada"}"#.to_owned(),
            };
        });
    settle(&mut harness);
    click_id(&mut harness, "http-send");
    answered(&mut harness);

    click_id(&mut harness, "http-response-section-timeline");
    assert!(said_by(&harness, "http-response-sent").is_some_and(|sent| sent.starts_with("POST")));
    assert!(said_by(&harness, "http-response-sent-body").is_some_and(|body| body.contains("ada")));
}

#[test]
fn an_image_answer_is_shown_as_its_picture() {
    let mut harness = sending_to("/image");
    click_id(&mut harness, "http-send");
    answered(&mut harness);

    crate::notes::bench::until(&mut harness, "the picture is shown", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("http-response-image"))
            .is_some()
    });
}

#[test]
fn a_wide_rail_names_what_its_head_makes() {
    let mut harness = paiements();
    harness.state_mut().state.settings.rail_width = 300;
    settle(&mut harness);

    click_id(&mut harness, "http-collection-create-open");

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("http-create-input"))
            .is_some()
    );
}
