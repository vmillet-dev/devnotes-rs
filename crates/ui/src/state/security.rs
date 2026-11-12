//! What the Security and Variables pages ask of the core: the passphrase changed over the open
//! library, and the variables, which are corpus data and not a preference.

use std::collections::BTreeMap;

use engine::error::ErrorCode;
use engine::vault::PassphraseChange;
use i18n::{Message, keys};
use zeroize::Zeroizing;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;

/// One row of the Variables page: a pair and not a map entry, since a half-filled row has to
/// exist long enough to be finished.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub value: String,
}

/// The rows to send: named as the core allows, valued, the last of a duplicate winning.
pub fn record(rows: &[Variable]) -> BTreeMap<String, String> {
    rows.iter()
        .filter(|row| !row.value.is_empty() && api::notes::is_field_name(&row.name))
        .map(|row| (row.name.clone(), row.value.clone()))
        .collect()
}

/// Names carried by more than one row, for the page to flag.
pub fn duplicates(rows: &[Variable]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut twice = std::collections::BTreeSet::new();
    for row in rows.iter().filter(|row| !row.name.is_empty()) {
        if !seen.insert(row.name.as_str()) {
            twice.insert(row.name.clone());
        }
    }
    twice.into_iter().collect()
}

#[derive(Default)]
pub struct SecurityState {
    changing: Option<Pending<PassphraseChange>>,
    refused: bool,
    /// Answered without a refusal: the dialog closes.
    changed: bool,
    stored: Option<Vec<Variable>>,
    asked: Option<Pending<BTreeMap<String, String>>>,
    saving: Vec<Pending<BTreeMap<String, String>>>,
}

impl SecurityState {
    pub fn is_changing(&self) -> bool {
        self.changing.is_some()
    }

    pub fn refused(&self) -> bool {
        self.refused
    }

    pub fn clear_refusal(&mut self) {
        self.refused = false;
    }

    pub fn change(
        &mut self,
        backend: &Backend,
        current: Zeroizing<String>,
        next: Zeroizing<String>,
    ) {
        if !self.is_changing() {
            self.refused = false;
            self.changing = Some(api::vault::change_passphrase(backend, current, next));
        }
    }

    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    pub fn load_variables(&mut self, backend: &Backend) {
        self.stored = None;
        self.asked = Some(api::notes::global_variables(backend));
    }

    /// `None` until the answer is in.
    pub fn variables(&self) -> Option<&[Variable]> {
        self.stored.as_deref()
    }

    /// What was typed stays on screen: the core drops half-filled rows, which must last long
    /// enough to be finished.
    pub fn save_variables(&mut self, backend: &Backend, rows: Vec<Variable>) {
        let values = record(&rows);
        self.stored = Some(rows);
        self.saving
            .push(api::notes::set_global_variables(backend, values));
    }

    pub fn update(&mut self, status: &mut StatusNotifier, errors: &mut ErrorNotifier) {
        if let Some(answer) = self.changing.as_mut().and_then(Pending::poll) {
            self.changing = None;
            match answer {
                Ok(change) => {
                    status.report(revocation(change));
                    self.changed = true;
                }
                Err(error) if error.code == ErrorCode::WrongPassphrase => self.refused = true,
                Err(error) => errors.report(keys::errors::PASSPHRASE_CHANGE_FAILED, &error),
            }
        }
        if let Some(answer) = self.asked.as_mut().and_then(Pending::poll) {
            self.asked = None;
            match answer {
                Ok(stored) => {
                    let rows = stored.into_iter();
                    self.stored =
                        Some(rows.map(|(name, value)| Variable { name, value }).collect());
                }
                Err(error) => errors.report(keys::errors::VARIABLES_LOAD_FAILED, &error),
            }
        }
        self.saving.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(error)) => {
                errors.report(keys::errors::VARIABLES_SAVE_FAILED, &error);
                false
            }
        });
    }
}

/// How much of the change reached the copies kept beside the library.
fn revocation(change: PassphraseChange) -> Message {
    use keys::settings::security;
    match change.backups_left {
        0 => Message::new(security::CHANGED),
        1 => Message::new(security::CHANGED_ONE_LEFT),
        count => Message::new(security::CHANGED_SOME_LEFT).with("count", count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    fn row(name: &str, value: &str) -> Variable {
        Variable {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    fn settled(state: &mut SecurityState, backend: &Backend) -> (StatusNotifier, ErrorNotifier) {
        let (mut status, mut errors) = (StatusNotifier::default(), ErrorNotifier::default());
        backend.settle();
        state.update(&mut status, &mut errors);
        (status, errors)
    }

    #[test]
    fn a_badly_named_or_empty_row_is_not_sent_and_the_last_of_a_duplicate_wins() {
        let rows = [
            row("host", "first"),
            row("host", "last"),
            row("has space", "refused"),
            row("port", ""),
        ];

        let sent = record(&rows);

        assert_eq!(sent.len(), 1);
        assert_eq!(sent["host"], "last");
        assert_eq!(duplicates(&rows), ["host"]);
    }

    #[test]
    fn the_right_phrase_changes_it_and_says_so() {
        let (backend, _profile) = testing::unlocked();
        let mut state = SecurityState::default();
        let current = Zeroizing::new(testing::PASSPHRASE.to_owned());
        let next = Zeroizing::new("a second long passphrase".to_owned());

        state.change(&backend, current, next);
        let (status, _) = settled(&mut state, &backend);

        assert!(state.take_changed());
        assert_eq!(
            status.latest().map(|(_, said)| said.key),
            Some(keys::settings::security::CHANGED)
        );
    }

    #[test]
    fn the_wrong_phrase_is_refused_where_it_was_typed() {
        let (backend, _profile) = testing::unlocked();
        let mut state = SecurityState::default();
        let current = Zeroizing::new("not the phrase".to_owned());
        let next = Zeroizing::new("a second long passphrase".to_owned());

        state.change(&backend, current, next);
        let (status, errors) = settled(&mut state, &backend);

        assert!(state.refused());
        assert!(!state.take_changed());
        assert!(status.latest().is_none() && errors.notice().is_none());
    }

    #[test]
    fn variables_come_back_as_they_were_saved() {
        let (backend, _profile) = testing::unlocked();
        let mut state = SecurityState::default();

        state.save_variables(&backend, vec![row("host", "db.internal")]);
        settled(&mut state, &backend);
        state.load_variables(&backend);
        assert!(state.variables().is_none());
        settled(&mut state, &backend);

        assert_eq!(state.variables(), Some(&[row("host", "db.internal")][..]));
    }
}
