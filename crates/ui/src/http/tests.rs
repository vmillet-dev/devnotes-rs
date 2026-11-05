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
    assert_eq!(harness.state_mut().state.http.take_created().len(), 1);
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
