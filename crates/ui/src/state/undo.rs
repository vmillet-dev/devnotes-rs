//! The last write that can be put back, and the bar offering it. The banner is what the timer
//! hides, the record what `Ctrl+Z` reads: only `dismiss` gives up for good.

use i18n::keys;

use super::Revision;
use crate::api::{self, undo::Reversible};
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

/// Seconds. The note is not lost after that: it stays in the trash for 30 days.
pub const UNDO_WINDOW: f64 = 8.0;

#[derive(Default)]
pub struct UndoState {
    last: Option<Reversible>,
    /// When the banner goes, on egui's clock.
    banner_until: Option<f64>,
    reverting: Vec<Pending<u32>>,
}

impl UndoState {
    /// Nothing moved is nothing to offer: a bar saying « 0 notes » is noise, not an undo.
    pub fn record(&mut self, action: Reversible, now: f64) {
        if action.count() == 0 {
            return;
        }
        self.last = Some(action);
        self.banner_until = Some(now + UNDO_WINDOW);
    }

    /// What the bar offers at `now`; `None` once its time is up, though `Ctrl+Z` still can.
    pub fn banner(&self, now: f64) -> Option<&Reversible> {
        self.banner_until
            .filter(|until| now < *until)
            .and(self.last.as_ref())
    }

    /// Seconds until the banner goes: the window is redrawn then, without an input.
    pub fn banner_left(&self, now: f64) -> Option<f64> {
        self.banner(now)?;
        self.banner_until.map(|until| until - now)
    }

    /// `false` when there was nothing to put back.
    pub fn revert(&mut self, backend: &Backend) -> bool {
        let Some(action) = self.last.take() else {
            return false;
        };
        self.banner_until = None;
        self.reverting.push(api::undo::revert(backend, action));
        true
    }

    pub fn dismiss(&mut self) {
        self.last = None;
        self.banner_until = None;
    }

    pub fn update(&mut self, revision: &mut Revision, errors: &mut ErrorNotifier) {
        self.reverting.retain_mut(|pending| match pending.poll() {
            None => true,
            Some(Ok(_)) => {
                revision.bump();
                false
            }
            Some(Err(error)) => {
                errors.report(keys::errors::UNDO_FAILED, &error);
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pinned() -> Reversible {
        Reversible::Pin {
            ids: vec!["a".into()],
            pinned: true,
        }
    }

    #[test]
    fn the_banner_goes_with_its_timer_but_the_record_stays() {
        let (backend, _profile) = crate::testing::unlocked();
        let mut undo = UndoState::default();
        undo.record(pinned(), 10.0);

        assert!(undo.banner(17.9).is_some());
        assert!(undo.banner(18.0).is_none());
        assert!(undo.banner_left(18.0).is_none());
        assert!(undo.revert(&backend), "Ctrl+Z still reads the record");
        assert!(!undo.revert(&backend), "and puts it back once");
    }

    #[test]
    fn nothing_touched_is_nothing_to_offer() {
        let mut undo = UndoState::default();
        undo.record(Reversible::Move(Vec::new()), 0.0);
        undo.record(
            Reversible::Deletion {
                ids: vec!["a".into()],
                count: 0,
            },
            0.0,
        );

        assert!(undo.banner(0.0).is_none());
    }

    #[test]
    fn a_dismissed_offer_is_given_up_for_good() {
        let (backend, _profile) = crate::testing::unlocked();
        let mut undo = UndoState::default();
        undo.record(pinned(), 0.0);
        undo.dismiss();

        assert!(undo.banner(0.0).is_none());
        assert!(!undo.revert(&backend));
    }
}
