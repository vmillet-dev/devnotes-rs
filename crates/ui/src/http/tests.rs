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
