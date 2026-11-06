//! The sockets, one a tab: Rust holds them, this keeps where each stands and its log. Each event
//! names the id the socket was connected under, not the tab: a draft saved meanwhile is rekeyed.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use engine::error::AppError;
use engine::http::WebsocketRequest;
use engine::http::websocket::{SocketEvent, WebsocketEvent};

use super::tabs::Tab;
use crate::api;
use crate::backend::{Backend, Pending};

/// What a log keeps: past it, the oldest go.
pub const LOG_KEPT: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone)]
pub struct SocketEntry {
    pub at: DateTime<Utc>,
    pub event: SocketEvent,
}

pub struct SocketState {
    socket_id: String,
    pub phase: Phase,
    pub protocol: Option<String>,
    /// Why it would not open, or why it broke.
    pub notice: Option<AppError>,
    pub log: VecDeque<SocketEntry>,
}

#[derive(Default)]
pub struct Sockets {
    by_tab: HashMap<String, SocketState>,
    connections: u32,
    connecting: Vec<(String, Pending<()>)>,
    /// A send and a close answer only whether a socket was there: nothing to say of it.
    asked: Vec<Pending<bool>>,
}

impl Sockets {
    pub fn of(&self, key: &str) -> Option<&SocketState> {
        self.by_tab.get(key)
    }

    /// Connected or on its way: what closing its tab would cut.
    pub fn is_live(&self, key: &str) -> bool {
        self.phase(key)
            .is_some_and(|phase| matches!(phase, Phase::Open | Phase::Connecting))
    }

    pub fn is_open(&self, key: &str) -> bool {
        self.phase(key) == Some(Phase::Open)
    }

    fn phase(&self, key: &str) -> Option<Phase> {
        self.by_tab.get(key).map(|state| state.phase)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.by_tab.keys().map(String::as_str)
    }

    /// Once a frame: a socket that would not open says why.
    pub fn update(&mut self) {
        self.asked.retain_mut(|asked| asked.poll().is_none());
        let mut refused = Vec::new();
        self.connecting.retain_mut(|(id, pending)| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            if let Err(error) = answer {
                refused.push((id.clone(), error));
            }
            false
        });
        for (id, error) in refused {
            if let Some(state) = self.by_socket(&id) {
                state.phase = Phase::Closed;
                state.notice = Some(error);
            }
        }
    }

    pub fn connect(&mut self, backend: &Backend, tab: &Tab, environment_id: Option<String>) {
        if self
            .phase(&tab.key)
            .is_some_and(|phase| phase != Phase::Closed)
        {
            return;
        }
        self.connections += 1;
        let socket_id = format!("{}#{}", tab.key, self.connections);
        let log = self
            .by_tab
            .remove(&tab.key)
            .map(|state| state.log)
            .unwrap_or_default();
        self.by_tab.insert(
            tab.key.clone(),
            SocketState {
                socket_id: socket_id.clone(),
                phase: Phase::Connecting,
                protocol: None,
                notice: None,
                log,
            },
        );
        let request = WebsocketRequest {
            id: socket_id.clone(),
            document: tab.draft.document.clone(),
            collection_id: tab.place.as_ref().map(|place| place.collection_id.clone()),
            folder_id: tab.place.as_ref().and_then(|place| place.folder_id.clone()),
            environment_id,
        };
        let pending = api::http::connect_socket(backend, request);
        self.connecting.push((socket_id, pending));
    }

    pub fn send(&mut self, backend: &Backend, key: &str, text: String) {
        let Some(state) = self.by_tab.get(key).filter(|it| it.phase == Phase::Open) else {
            return;
        };
        let id = state.socket_id.clone();
        self.asked.push(api::http::send_socket(backend, id, text));
    }

    pub fn close(&mut self, backend: &Backend, key: &str) {
        if !self.is_live(key) {
            return;
        }
        let Some(state) = self.by_tab.get_mut(key) else {
            return;
        };
        state.phase = Phase::Closing;
        let id = state.socket_id.clone();
        self.asked.push(api::http::close_socket(backend, id));
    }

    pub fn clear_log(&mut self, key: &str) {
        if let Some(state) = self.by_tab.get_mut(key) {
            state.log.clear();
        }
    }

    pub fn rekey(&mut self, from: &str, to: &str) {
        if let Some(state) = self.by_tab.remove(from) {
            self.by_tab.insert(to.to_owned(), state);
        }
    }

    /// A tab closed: its socket with it.
    pub fn forget(&mut self, backend: &Backend, key: &str) {
        if let Some(state) = self.by_tab.remove(key)
            && state.phase != Phase::Closed
        {
            self.asked
                .push(api::http::close_socket(backend, state.socket_id));
        }
    }

    fn by_socket(&mut self, id: &str) -> Option<&mut SocketState> {
        self.by_tab.values_mut().find(|state| state.socket_id == id)
    }

    /// An event for a socket replaced since is dropped.
    pub fn receive(&mut self, event: WebsocketEvent) {
        let WebsocketEvent { id, at, event } = event;
        let Some(state) = self.by_socket(&id) else {
            return;
        };
        match &event {
            SocketEvent::Opened { protocol, .. } => {
                state.phase = Phase::Open;
                state.protocol.clone_from(protocol);
                state.notice = None;
            }
            SocketEvent::Closed { .. } => state.phase = Phase::Closed,
            SocketEvent::Failed {
                code,
                params,
                detail,
            } => {
                state.phase = Phase::Closed;
                state.notice = Some(AppError {
                    code: *code,
                    params: params.clone(),
                    detail: detail.clone(),
                });
            }
            SocketEvent::Sent { .. } | SocketEvent::Received { .. } => {}
        }
        state.log.push_back(SocketEntry { at, event });
        if state.log.len() > LOG_KEPT {
            state.log.pop_front();
        }
    }
}
