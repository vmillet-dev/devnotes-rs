//! The copies of the library, and the one gesture that puts one back. It replaces the whole
//! corpus, so the trigger only proposes: a second click on the button that fired it is the guard
//! a double click defeats. The restore itself is the gate's, which comes back asking for a phrase.

use engine::backup::copies::Backup;
use i18n::keys;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct BackupsState {
    copies: Vec<Backup>,
    asked: Option<Pending<Vec<Backup>>>,
    pending: Option<String>,
}

impl BackupsState {
    pub fn load(&mut self, backend: &Backend) {
        self.asked = Some(api::backups::list(backend));
    }

    pub fn is_loading(&self) -> bool {
        self.asked.is_some()
    }

    pub fn copies(&self) -> &[Backup] {
        &self.copies
    }

    /// The copy whose restore is being considered.
    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }

    /// Proposes only. A copy with no key file opens for nobody and is never offered.
    pub fn ask(&mut self, id: String) {
        let openable = self
            .copies
            .iter()
            .any(|copy| copy.id == id && copy.openable);
        if openable {
            self.pending = Some(id);
        }
    }

    pub fn dismiss(&mut self) {
        self.pending = None;
    }

    pub fn update(&mut self, errors: &mut ErrorNotifier) {
        let Some(answer) = self.asked.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.asked = None;
        match answer {
            Ok(copies) => self.copies = copies,
            Err(error) => errors.report(keys::errors::BACKUPS_LIST_FAILED, &error),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::testing;

    fn copy(id: &str, openable: bool) -> Backup {
        Backup {
            id: id.to_owned(),
            taken_at: Utc::now(),
            bytes: 1024,
            openable,
        }
    }

    #[test]
    fn a_copy_with_no_key_is_never_offered() {
        let mut state = BackupsState {
            copies: vec![copy("keyless", false), copy("whole", true)],
            ..BackupsState::default()
        };

        state.ask("keyless".to_owned());
        assert_eq!(state.pending(), None);

        state.ask("whole".to_owned());
        assert_eq!(state.pending(), Some("whole"));

        state.dismiss();
        assert_eq!(state.pending(), None);
    }

    #[test]
    fn the_copies_are_read_from_the_core() {
        let (backend, _profile) = testing::unlocked();
        let mut state = BackupsState::default();
        let mut errors = ErrorNotifier::default();

        state.load(&backend);
        assert!(state.is_loading());
        backend.settle();
        state.update(&mut errors);

        assert!(!state.is_loading());
        assert!(errors.notice().is_none());
    }
}
