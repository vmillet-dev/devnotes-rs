//! Asks before the window goes with requests that are not saved in it: a request typed in a tab
//! and never saved, or a saved one with changes, is gone after a restart. Nothing is asked when no
//! tab is modified: leaving stays instant.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    Cancel,
    Discard,
    SaveAll,
}

/// What the question stands in front of: its words, and what follows the answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Reason {
    #[default]
    Quit,
    Switch,
    Restore,
}

#[derive(Default)]
pub struct Leave {
    reason: Reason,
    asking: bool,
    /// « Save all » under way, and the tabs it has written: one still modified after its write
    /// failed or was refused a place, and the question stays out.
    saving: Option<BTreeSet<String>>,
    /// Answered: the window may go, and nothing is asked again.
    released: bool,
    /// The window is told to close once.
    closes: bool,
}

impl Leave {
    pub fn is_asking(&self) -> bool {
        self.asking
    }

    pub fn reason(&self) -> Reason {
        self.reason
    }

    /// Whether the area's state is to go on while another area is shown.
    pub fn is_under_way(&self) -> bool {
        self.asking || self.closes
    }

    pub fn is_saving(&self) -> bool {
        self.saving.is_some()
    }

    pub fn is_released(&self) -> bool {
        self.released
    }

    pub fn ask(&mut self, reason: Reason) {
        self.reason = reason;
        self.asking = true;
        self.saving = None;
    }

    pub fn cancel(&mut self) {
        self.asking = false;
        self.saving = None;
    }

    pub fn save_all(&mut self) {
        self.saving = Some(BTreeSet::new());
    }

    pub fn release(&mut self) {
        self.asking = false;
        self.saving = None;
        self.released = true;
        self.closes = true;
    }

    /// The next tab « Save all » writes, once; `false` as the answer stops it there.
    pub fn written(&mut self, key: &str) -> bool {
        let Some(written) = &mut self.saving else {
            return false;
        };
        if written.insert(key.to_owned()) {
            return true;
        }
        self.saving = None;
        false
    }

    pub fn take_close(&mut self) -> bool {
        self.reason == Reason::Quit && std::mem::take(&mut self.closes)
    }

    /// The answer let a switch or a restore through. Asked again next time: the window stays.
    pub fn take_switch(&mut self) -> bool {
        let passed = self.reason != Reason::Quit && std::mem::take(&mut self.closes);
        self.released &= !passed;
        passed
    }
}
