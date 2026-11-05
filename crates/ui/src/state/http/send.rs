//! The sends, one a tab: Rust sends; this keeps what each tab shows of it. A send leaves on the
//! network queue and never holds the db queue; the time it started is kept here and not in the
//! pane that counts from it, so leaving the tab and coming back does not restart the count.

use std::collections::HashMap;
use std::sync::Arc;

use engine::error::{AppError, ErrorCode};
use engine::http::send::{SendRequest, SentResponse};

use super::tabs::Tab;
use crate::api;
use crate::backend::{Backend, Pending};

/// Where a tab's send stands.
pub enum Phase {
    Sending { started: f64 },
    Answered(Arc<SentResponse>),
    Failed(AppError),
    Cancelled,
}

/// `id` is the name Rust keeps the body under, fixed at the send: a draft saved meanwhile
/// changes its tab's key, not that.
pub struct Sent {
    pub id: String,
    pub phase: Phase,
}

#[derive(Default)]
pub struct Sends {
    by_tab: HashMap<String, Sent>,
    in_flight: Vec<(String, Pending<SentResponse>)>,
    count: u32,
}

impl Sends {
    pub fn of(&self, key: &str) -> Option<&Sent> {
        self.by_tab.get(key)
    }

    /// The sends whose answers are still shown.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_tab.values().map(|sent| sent.id.as_str())
    }

    /// The tab a send belongs to.
    pub fn key_of(&self, id: &str) -> Option<&str> {
        let held = self.by_tab.iter().find(|(_, sent)| sent.id == id)?;
        Some(held.0.as_str())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.by_tab.keys().map(String::as_str)
    }

    pub fn is_sending(&self, key: &str) -> bool {
        matches!(
            self.of(key),
            Some(Sent {
                phase: Phase::Sending { .. },
                ..
            })
        )
    }

    /// Answers arrived this frame, under whatever tab now holds their send.
    /// `true` when a send ended.
    pub fn update(&mut self) -> bool {
        let mut settled = Vec::new();
        self.in_flight.retain_mut(|(id, pending)| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            let phase = match answer {
                Ok(response) => Phase::Answered(Arc::new(response)),
                Err(error) if error.code == ErrorCode::HttpCancelled => Phase::Cancelled,
                Err(error) => {
                    log::warn!("A send failed: {}", error.detail);
                    Phase::Failed(error)
                }
            };
            settled.push((id.clone(), phase));
            false
        });
        let ended = !settled.is_empty();
        for (id, phase) in settled {
            if let Some(sent) = self.by_tab.values_mut().find(|sent| sent.id == id) {
                sent.phase = phase;
            }
        }
        ended
    }

    /// The tab's draft as it is, from where it sits; a send already under way is let be.
    pub fn send(
        &mut self,
        backend: &Backend,
        tab: &Tab,
        (now, environment_id): (f64, Option<String>),
        confirmed: bool,
    ) {
        if self.is_sending(&tab.key) {
            return;
        }
        self.count += 1;
        let id = format!("{}#{}", tab.key, self.count);
        let request = SendRequest {
            id: id.clone(),
            kind: tab.draft.kind,
            method: tab.draft.method,
            document: tab.draft.document.clone(),
            collection_id: tab.place.as_ref().map(|place| place.collection_id.clone()),
            folder_id: tab.place.as_ref().and_then(|place| place.folder_id.clone()),
            name: tab.draft.name.clone(),
            request_id: tab.request_id.clone(),
            environment_id,
            confirmed,
        };
        let pending = api::http::send(backend, request);
        let previous = self.by_tab.insert(
            tab.key.clone(),
            Sent {
                id: id.clone(),
                phase: Phase::Sending { started: now },
            },
        );
        if let Some(previous) = previous {
            api::http::forget_response(backend, previous.id);
        }
        self.in_flight.push((id, pending));
    }

    pub fn cancel(&self, backend: &Backend, key: &str) {
        if let Some(sent) = self.of(key).filter(|_| self.is_sending(key)) {
            api::http::cancel(backend, sent.id.clone());
        }
    }

    /// A tab closed, or its pane cleared: what it sent, and what Rust kept of the answer, go.
    pub fn forget(&mut self, backend: &Backend, key: &str) {
        self.cancel(backend, key);
        if let Some(sent) = self.by_tab.remove(key) {
            api::http::forget_response(backend, sent.id);
        }
    }

    /// A draft saved for the first time: its tab is known by the request's id from then on.
    pub fn rekey(&mut self, from: &str, to: &str) {
        if let Some(sent) = self.by_tab.remove(from) {
            self.by_tab.insert(to.to_owned(), sent);
        }
    }
}
