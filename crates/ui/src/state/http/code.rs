//! « Generate code »: the request written for another tool, from what a send would write. Rust
//! writes it; here is what is asked of it, and the question that stands before a secret in clear.

use engine::http::CodeRequestOf;
use engine::http::codegen::{CodeOptions, CodeTarget, GeneratedCode};
use engine::http::model::{HttpMethod, RequestDocument, RequestKind};
use i18n::keys;

use super::tabs::{Place, Tab};
use crate::api;
use crate::backend::Backend;
use crate::services::errors::ErrorNotifier;
use crate::state::Revision;
use crate::state::query::QueryCell;

/// The switches that change at a click; « Include secrets » asks first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    Resolve,
    Multiline,
    Follow,
}

/// What the dialog asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// On the tab it was opened from.
    Open(String),
    Close,
    Target(CodeTarget),
    Toggle(Switch),
    /// Switched on, it waits for a yes; off, it never asks.
    ToggleSecrets,
    CancelSecrets,
    IncludeSecrets,
}

#[derive(Clone, PartialEq)]
struct Question {
    target: CodeTarget,
    options: CodeOptions,
    kind: RequestKind,
    method: HttpMethod,
    document: RequestDocument,
    place: Option<Place>,
    environment: Option<String>,
    revision: Revision,
}

fn fresh_options() -> CodeOptions {
    CodeOptions {
        resolve_variables: true,
        include_secrets: false,
        multiline: true,
        follow_redirects: false,
    }
}

#[derive(Default)]
pub struct CodeDialog {
    /// The tab it is open on.
    key: Option<String>,
    /// The target last chosen, for the session: the dialog opens again on it.
    target: CodeTarget,
    options: CodeOptions,
    asking_secrets: bool,
    code: QueryCell<Question, GeneratedCode>,
}

impl CodeDialog {
    pub fn ask(&mut self, asked: Asked) {
        match asked {
            Asked::Open(key) => {
                self.key = Some(key);
                self.options = fresh_options();
                self.asking_secrets = false;
            }
            Asked::Close => self.key = None,
            Asked::Target(target) => self.target = target,
            Asked::Toggle(Switch::Resolve) => {
                self.options.resolve_variables = !self.options.resolve_variables;
            }
            Asked::Toggle(Switch::Multiline) => self.options.multiline = !self.options.multiline,
            Asked::Toggle(Switch::Follow) => {
                self.options.follow_redirects = !self.options.follow_redirects;
            }
            Asked::ToggleSecrets if self.options.include_secrets => {
                self.options.include_secrets = false;
            }
            Asked::ToggleSecrets => self.asking_secrets = true,
            Asked::CancelSecrets => self.asking_secrets = false,
            Asked::IncludeSecrets => {
                self.asking_secrets = false;
                self.options.include_secrets = true;
            }
        }
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn target(&self) -> CodeTarget {
        self.target
    }

    pub fn options(&self) -> CodeOptions {
        self.options
    }

    pub fn asking_secrets(&self) -> bool {
        self.asking_secrets
    }

    pub fn code(&self) -> Option<&GeneratedCode> {
        self.code.value()
    }

    /// Once a frame while it is open.
    pub fn update(
        &mut self,
        backend: &Backend,
        tab: &Tab,
        (environment, revision): (Option<String>, Revision),
        errors: &mut ErrorNotifier,
    ) {
        self.code.want(Question {
            target: self.target,
            options: self.options,
            kind: tab.draft.kind,
            method: tab.draft.method,
            document: tab.draft.document.clone(),
            place: tab.place.clone(),
            environment,
            revision,
        });
        let answered = self.code.update(|question| {
            let place = question.place;
            api::http::generate_code(
                backend,
                CodeRequestOf {
                    target: question.target,
                    options: question.options,
                    kind: question.kind,
                    method: question.method,
                    document: question.document,
                    collection_id: place.as_ref().map(|place| place.collection_id.clone()),
                    folder_id: place.and_then(|place| place.folder_id),
                    environment_id: question.environment,
                },
            )
        });
        if answered && let Some(error) = self.code.error() {
            errors.report(keys::http::FAILED, error);
        }
    }
}
