//! The library's cookie jar while its dialog is open: what answers set, by domain, to read and to
//! throw away. Rust matches and seals them; here they are listed.

use engine::http::cookies::store::CookieDomain;
use i18n::keys;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::state::Revision;
use crate::state::query::QueryCell;

/// What the dialog asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Open,
    Close,
    /// What « Empty the jar » removes, counted by Rust before the question.
    AskClear,
    CancelClear,
    Clear,
    Remove(String),
    RemoveDomain(String),
}

#[derive(Default)]
pub struct CookieJar {
    open: bool,
    read: Revision,
    domains: QueryCell<Revision, Vec<CookieDomain>>,
    counting: Option<Pending<u32>>,
    confirming: Option<u32>,
    /// Each answers how many went.
    removing: Vec<Pending<u32>>,
}

impl CookieJar {
    /// Once a frame. Answers how many cookies went, when some were removed.
    pub fn update(&mut self, backend: &Backend, errors: &mut ErrorNotifier) -> Option<u32> {
        if !self.open {
            return None;
        }
        if let Some(pending) = &mut self.counting
            && let Some(answer) = pending.poll()
        {
            self.counting = None;
            match answer {
                Ok(count) => self.confirming = Some(count),
                Err(error) => errors.report(keys::http::cookies::FAILED, &error),
            }
        }
        let mut removed = None;
        self.removing.retain_mut(|removal| {
            let Some(answer) = removal.poll() else {
                return true;
            };
            match answer {
                Ok(count) => removed = Some(removed.unwrap_or(0) + count),
                Err(error) => errors.report(keys::http::cookies::FAILED, &error),
            }
            false
        });
        if removed.is_some() {
            self.read.bump();
        }
        self.domains.want(self.read);
        if self.domains.update(|_| api::http::cookies(backend))
            && let Some(error) = self.domains.error()
        {
            errors.report(keys::http::cookies::FAILED, error);
        }
        removed
    }

    pub fn ask(&mut self, backend: &Backend, asked: Asked) {
        match asked {
            Asked::Open => {
                self.open = true;
                self.read.bump();
            }
            Asked::Close => {
                self.open = false;
                self.confirming = None;
            }
            Asked::AskClear => self.counting = Some(api::http::count_cookies(backend)),
            Asked::CancelClear => self.confirming = None,
            Asked::Clear => {
                self.confirming = None;
                self.removing.push(api::http::clear_cookies(backend));
            }
            Asked::Remove(id) => self.removing.push(api::http::delete_cookie(backend, id)),
            Asked::RemoveDomain(domain) => {
                self.removing
                    .push(api::http::delete_cookie_domain(backend, domain));
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// `None` until read.
    pub fn domains(&self) -> Option<&[CookieDomain]> {
        self.domains.value().map(Vec::as_slice)
    }

    pub fn confirming(&self) -> Option<u32> {
        self.confirming
    }
}
