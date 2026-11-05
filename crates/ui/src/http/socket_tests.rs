//! A WebSocket's tab: its composer over a real core, its log fed by a fake source of events.

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use engine::http::model::{HttpMethod, RequestKind};
use engine::http::websocket::{SocketEvent, WebsocketEvent};

use super::tests::{active, click_id, click_named, collection, request, said_by};
use crate::notes::bench::{Bench, area_of, settle, until};
use crate::state::Area;
use crate::state::http::sockets::{LOG_KEPT, Phase};
use crate::testing;

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

/// A socket's tab, in front.
fn socket() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Flux");
    let kind = (RequestKind::Websocket, HttpMethod::Get);
    request(&backend, (&api, None), "Prix", kind);
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(egui::vec2(1_100.0, 900.0));
    settle(&mut harness);
    click_named(&mut harness, "http-node-name", "WS Prix");
    until(&mut harness, "the composer is open", |harness| {
        has(harness, "http-socket-message")
    });
    harness
}

fn type_message(harness: &mut Harness<'_, Bench>, words: &str) {
    click_id(harness, "http-socket-message");
    harness
        .get_by(|node| node.author_id() == Some("http-socket-message"))
        .type_text(words);
    settle(harness);
}

#[test]
fn a_message_is_kept_named_after_its_first_line_and_loaded_back() {
    let mut harness = socket();
    type_message(&mut harness, "ping");
    click_id(&mut harness, "http-socket-keep");

    let kept = active(&harness).document.websocket.messages;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "ping");

    click_id(&mut harness, "http-socket-saved-remove");
    assert!(active(&harness).document.websocket.messages.is_empty());
}

#[test]
fn ctrl_enter_on_a_closed_socket_says_so_and_connects_nothing() {
    let mut harness = socket();
    type_message(&mut harness, "ping");

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    settle(&mut harness);

    assert!(has(&harness, "http-socket-not-open"));
    let key = active_key(&harness);
    assert!(harness.state().state.http.sockets().of(&key).is_none());
}

fn active_key(harness: &Harness<'_, Bench>) -> String {
    let tabs = harness.state().state.http.tabs();
    tabs.active().unwrap().key.clone()
}

#[test]
fn an_address_a_socket_cannot_open_says_so_and_offers_its_twin() {
    let mut harness = socket();
    click_id(&mut harness, "http-socket-connect");
    until(&mut harness, "the refusal is said", |harness| {
        has(harness, "http-socket-twin")
    });
    assert!(has(&harness, "http-socket-failure"));
    assert_eq!(
        said_by(&harness, "http-socket-phase").as_deref(),
        Some("Disconnected")
    );

    click_id(&mut harness, "http-socket-twin");
    assert!(active(&harness).document.url.starts_with("wss://"));
}

/// An event under the id the tab's socket was connected with.
fn event(id: &str, event: SocketEvent) -> WebsocketEvent {
    WebsocketEvent {
        id: id.to_owned(),
        at: Utc::now(),
        event,
    }
}

/// A connection left under way, its id known from its first event.
fn connected(harness: &mut Harness<'_, Bench>) -> String {
    let key = active_key(harness);
    let url = "ws://127.0.0.1:9/".to_owned();
    let tabs = harness.state_mut().state.http.tabs_mut();
    tabs.edit(&key, |draft| {
        draft.apply(crate::state::http::tabs::DraftEdit::Url(url));
    });
    settle(harness);
    click_id(harness, "http-socket-connect");
    let id = format!("{key}#1");
    let opened = SocketEvent::Opened {
        url: "ws://local".to_owned(),
        protocol: Some("chat".to_owned()),
    };
    harness
        .state_mut()
        .state
        .http
        .receive_socket(event(&id, opened));
    settle(harness);
    id
}

#[test]
fn what_went_each_way_is_logged_and_filtered() {
    let mut harness = socket();
    let id = connected(&mut harness);
    assert_eq!(
        said_by(&harness, "http-socket-phase").as_deref(),
        Some("Connected")
    );

    let http = &mut harness.state_mut().state.http;
    http.receive_socket(event(
        &id,
        SocketEvent::Sent {
            text: "ping".to_owned(),
            size: 4,
        },
    ));
    http.receive_socket(event(
        &id,
        SocketEvent::Received {
            text: "pong".to_owned(),
            size: 4,
            binary: false,
            cut: false,
        },
    ));
    settle(&mut harness);
    let rows = |harness: &Harness<'_, Bench>| {
        harness
            .query_all_by(|node| node.author_id() == Some("http-socket-entry"))
            .count()
    };
    assert_eq!(rows(&harness), 3);

    click_id(&mut harness, "http-socket-filter");
    harness
        .get_by(|node| node.author_id() == Some("http-socket-filter"))
        .type_text("pong");
    settle(&mut harness);
    assert_eq!(rows(&harness), 1);
}

#[test]
fn closing_a_live_socket_s_tab_asks_first() {
    let mut harness = socket();
    connected(&mut harness);
    let key = active_key(&harness);

    harness.state_mut().state.http.ask_close(&key);
    settle(&mut harness);
    assert!(has(&harness, "http-close-connected"));
}

#[test]
fn a_log_keeps_the_last_entries_and_an_old_socket_s_events_are_dropped() {
    let mut harness = socket();
    let id = connected(&mut harness);
    let key = active_key(&harness);
    let http = &mut harness.state_mut().state.http;
    for _ in 0..LOG_KEPT + 3 {
        http.receive_socket(event(
            &id,
            SocketEvent::Sent {
                text: "x".to_owned(),
                size: 1,
            },
        ));
    }
    let closed = SocketEvent::Closed {
        code: None,
        reason: String::new(),
    };
    http.receive_socket(event("replaced#9", closed));

    let state = http.sockets().of(&key).unwrap();
    assert_eq!(state.log.len(), LOG_KEPT);
    assert_eq!(state.phase, Phase::Open);
}
