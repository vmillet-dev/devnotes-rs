//! The vault's gate: the first screen of a launch, and of every library switch. Nothing else is
//! drawn until the library is open (plan § 3.3.8 « Déverrouillage »).

mod draw;
pub mod passphrase;
pub mod prompt;

pub use draw::show;

use engine::error::{AppError, ErrorCode};
use engine::libraries::registry::Registry;
use engine::vault::{MINIMUM_LENGTH, VaultState};
use i18n::{Key, Message, keys};
use zeroize::Zeroizing;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;

/// What the gate's controls ask, applied at the end of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Submit,
    ToggleReveal,
    AskToArchive,
    KeepTrying,
    Archive,
    SetAside,
    /// To the library `GateView::switch_to` names.
    SwitchLibrary,
}

/// How far the index of the library just opened is: the first sections show after the first
/// slice, and the views say they are partial until it is whole.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IndexFill {
    #[default]
    Empty,
    Filling {
        done: u32,
        total: u32,
    },
    Ready,
}

enum Work {
    /// Creating or unlocking the library.
    Opening(Pending<()>),
    /// Setting a library aside, with the status that says where it went.
    Moving(Pending<String>, Key),
    Switching(Pending<()>),
    /// A copy put back: the folder the replaced library went to.
    Restoring(Pending<String>),
}

/// The gate's state: what the library is, what is under way, what the last attempt said.
#[derive(Default)]
pub struct Gate {
    vault: Option<VaultState>,
    asked: Option<Pending<VaultState>>,
    registry: Option<Registry>,
    registry_asked: Option<Pending<Registry>>,
    work: Option<Work>,
    pub refused: bool,
    pub damaged: bool,
    pub confirming_archive: bool,
    pub index: IndexFill,
}

/// What only the gate's drawing sees: the two fields, « Show », the library picked.
#[derive(Default)]
pub struct GateView {
    pub passphrase: Zeroizing<String>,
    pub confirmation: Zeroizing<String>,
    pub revealed: bool,
    pub switch_to: Option<usize>,
}

impl GateView {
    /// Nothing typed is kept once it was handed over or given up.
    fn forget(&mut self) {
        self.passphrase = Zeroizing::new(String::new());
        self.confirmation = Zeroizing::new(String::new());
    }
}

impl Gate {
    /// Asks what the library is: at launch, and after anything that changes it.
    pub fn ask(&mut self, backend: &Backend) {
        self.asked = Some(api::vault::state(backend));
        self.registry_asked = Some(api::libraries::list(backend));
    }

    pub fn is_open(&self) -> bool {
        self.vault == Some(VaultState::Unlocked)
    }

    pub fn vault(&self) -> Option<VaultState> {
        self.vault
    }

    pub fn registry(&self) -> Option<&Registry> {
        self.registry.as_ref()
    }

    pub fn is_working(&self) -> bool {
        self.work.is_some()
    }

    pub fn is_creating(&self) -> bool {
        self.vault == Some(VaultState::Absent)
    }

    /// The floor holds only when a phrase is chosen, never when one is given back.
    pub fn too_short(&self, view: &GateView) -> bool {
        self.is_creating()
            && !view.passphrase.is_empty()
            && length(&view.passphrase) < MINIMUM_LENGTH
    }

    pub fn mismatched(&self, view: &GateView) -> bool {
        self.is_creating()
            && !view.confirmation.is_empty()
            && *view.confirmation != *view.passphrase
    }

    pub fn can_submit(&self, view: &GateView) -> bool {
        if self.is_working() || view.passphrase.is_empty() {
            return false;
        }
        !self.is_creating()
            || (length(&view.passphrase) >= MINIMUM_LENGTH
                && *view.confirmation == *view.passphrase)
    }

    /// The answers that arrived. `true` when a library was just opened, or another chosen: the
    /// caller resets what belonged to the previous one and reads the new one's preferences.
    pub fn update(
        &mut self,
        backend: &Backend,
        status: &mut StatusNotifier,
        errors: &mut ErrorNotifier,
    ) -> bool {
        if let Some(answer) = self.asked.as_mut().and_then(Pending::poll) {
            self.asked = None;
            match answer {
                Ok(state) => self.vault = Some(state),
                Err(error) => errors.report(keys::errors::VAULT_STATE_FAILED, &error),
            }
        }
        if let Some(answer) = self.registry_asked.as_mut().and_then(Pending::poll) {
            self.registry_asked = None;
            self.registry = answer.ok();
        }
        let Some(work) = &mut self.work else {
            return false;
        };
        let mut switched = false;
        match work {
            Work::Opening(pending) => {
                if let Some(answer) = pending.poll() {
                    self.work = None;
                    switched = self.opened(answer, errors);
                }
            }
            Work::Moving(pending, report) => {
                let report = *report;
                if let Some(answer) = pending.poll() {
                    self.work = None;
                    match answer {
                        Ok(path) => {
                            status.report(Message::new(report).with("path", path));
                            // ⚠️ The mark goes with the library, or the fresh one opens with no
                            // space, where no note can be created.
                            drop(api::preferences::forget_in_closed_library(
                                backend,
                                crate::services::samples::SEEDED_KEY,
                            ));
                            self.damaged = false;
                            self.refused = false;
                            self.confirming_archive = false;
                            self.ask(backend);
                        }
                        Err(error) => errors.report(keys::errors::SET_ASIDE_FAILED, &error),
                    }
                }
            }
            Work::Switching(pending) => {
                if let Some(answer) = pending.poll() {
                    self.work = None;
                    match answer {
                        Ok(()) => {
                            *self = Self::default();
                            self.ask(backend);
                            switched = true;
                        }
                        Err(error) => errors.report(keys::errors::LIBRARY_ACTION_FAILED, &error),
                    }
                }
            }
            Work::Restoring(pending) => {
                if let Some(answer) = pending.poll() {
                    self.work = None;
                    match answer {
                        Ok(path) => {
                            status.report(Message::new(keys::backups::RESTORED).with("path", path));
                            // A copy from before the samples holds no space: asked again at the
                            // next open, which seeds nothing where there is one.
                            drop(api::preferences::forget_in_closed_library(
                                backend,
                                crate::services::samples::SEEDED_KEY,
                            ));
                            *self = Self::default();
                            self.ask(backend);
                            switched = true;
                        }
                        Err(error) => errors.report(keys::errors::BACKUP_RESTORE_FAILED, &error),
                    }
                }
            }
        }
        switched
    }

    fn opened(&mut self, answer: Result<(), AppError>, errors: &mut ErrorNotifier) -> bool {
        match answer {
            Ok(()) => {
                self.vault = Some(VaultState::Unlocked);
                self.index = IndexFill::Empty;
                return true;
            }
            Err(error) if error.code == ErrorCode::WrongPassphrase => self.refused = true,
            Err(error) if error.code == ErrorCode::LibraryDamaged => self.damaged = true,
            Err(error) => errors.report(keys::errors::UNLOCK_FAILED, &error),
        }
        false
    }

    /// Out of the open library, from its File menu: the gate comes back when this is answered.
    pub fn leave_for(&mut self, pending: Pending<()>) {
        self.work = Some(Work::Switching(pending));
    }

    /// As `leave_for`, with a copy of the library put back in its place.
    pub fn leave_for_copy(&mut self, pending: Pending<String>) {
        self.work = Some(Work::Restoring(pending));
    }

    /// What the index says while it fills behind the gate.
    pub fn index_progress(&mut self, done: u32, total: u32) {
        self.index = IndexFill::Filling { done, total };
    }

    pub fn index_ready(&mut self) {
        self.index = IndexFill::Ready;
    }

    pub fn apply(&mut self, action: Action, view: &mut GateView, backend: &Backend) {
        match action {
            Action::Submit => {
                if !self.can_submit(view) {
                    return;
                }
                let phrase = passphrase::take(&mut view.passphrase);
                view.forget();
                self.refused = false;
                let creating = self.is_creating();
                let pending = if creating {
                    api::vault::create(backend, phrase)
                } else {
                    api::vault::unlock(backend, phrase)
                };
                self.work = Some(Work::Opening(pending));
            }
            Action::ToggleReveal => view.revealed = !view.revealed,
            Action::AskToArchive => {
                view.forget();
                self.confirming_archive = true;
            }
            Action::KeepTrying => self.confirming_archive = false,
            Action::Archive => {
                self.work = Some(Work::Moving(
                    api::vault::archive(backend),
                    keys::vault::ARCHIVED,
                ));
            }
            Action::SetAside => {
                self.work = Some(Work::Moving(
                    api::vault::set_aside(backend),
                    keys::vault::SET_ASIDE,
                ));
            }
            Action::SwitchLibrary => {
                let id = view
                    .switch_to
                    .take()
                    .and_then(|at| self.registry.as_ref()?.libraries.get(at))
                    .map(|entry| entry.id.clone());
                if let Some(id) = id {
                    view.forget();
                    self.work = Some(Work::Switching(api::libraries::open(backend, id)));
                }
            }
        }
    }
}

/// In characters, as the floor counts them.
fn length(phrase: &str) -> u32 {
    u32::try_from(phrase.chars().count()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
