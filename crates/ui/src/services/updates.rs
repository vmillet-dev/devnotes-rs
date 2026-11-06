//! Offering a new version. Nothing installs without an explicit gesture: a silent update would
//! restart the application mid-keystroke. A failed check at launch stays silent — offline it would
//! fail every morning — while one asked from the menu says so.
//!
//! The updater itself is the platform phase's: until then `NoUpdater` has nothing to offer, and
//! the menu offers no check.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use engine::app_info::METADATA;
use engine::changelog::model::ChangelogRelease;
use i18n::{Message, keys};

use super::errors::ErrorNotifier;
use super::settings::Settings;
use crate::api;
use crate::backend::{Backend, Pending};

/// A version the updater found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

/// What the platform answers. ⚠️ Each blocks until it is done: never on the window's thread.
pub trait Updater {
    /// Whether anything can be checked at all.
    fn is_available(&self) -> bool;
    /// `None` when the application is up to date.
    fn check(&self) -> Result<Option<AvailableUpdate>, String>;
    /// `progress` in percent, when the size of the download is known.
    fn install(&self, progress: &dyn Fn(u32)) -> Result<(), String>;
    fn relaunch(&self);
}

/// What the application has until the platform phase.
pub struct NoUpdater;

impl Updater for NoUpdater {
    fn is_available(&self) -> bool {
        false
    }

    fn check(&self) -> Result<Option<AvailableUpdate>, String> {
        Ok(None)
    }

    fn install(&self, _progress: &dyn Fn(u32)) -> Result<(), String> {
        Err("no updater".to_owned())
    }

    fn relaunch(&self) {}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    Available,
    Installing,
    Installed,
}

/// Apart from `Status`: the prompt must not depend on a check that led nowhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CheckState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Failed,
}

type Checked = Result<Option<AvailableUpdate>, String>;

/// No percent yet: the server announced no size.
const UNKNOWN: u32 = u32::MAX;

pub struct UpdateState {
    updater: Arc<dyn Updater + Send + Sync>,
    update: Option<AvailableUpdate>,
    notes: Option<ChangelogRelease>,
    status: Status,
    check: CheckState,
    asked: Option<(bool, Pending<Checked>)>,
    installing: Option<Pending<Result<(), String>>>,
    percent: Arc<AtomicU32>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self::with(Arc::new(NoUpdater))
    }
}

impl UpdateState {
    pub fn with(updater: Arc<dyn Updater + Send + Sync>) -> Self {
        Self {
            updater,
            update: None,
            notes: None,
            status: Status::Idle,
            check: CheckState::Idle,
            asked: None,
            installing: None,
            percent: Arc::new(AtomicU32::new(UNKNOWN)),
        }
    }

    pub fn can_check(&self) -> bool {
        self.updater.is_available()
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn check_state(&self) -> CheckState {
        self.check
    }

    /// The offer on screen, which is the status and not a version: « Later » leaves the update
    /// known, for the dot, and the prompt closed.
    pub fn offered(&self) -> Option<&AvailableUpdate> {
        self.update.as_ref().filter(|_| self.status != Status::Idle)
    }

    pub fn notes(&self) -> Option<&ChangelogRelease> {
        self.notes.as_ref()
    }

    /// Something exists and is not installed: outlives the prompt it opened.
    pub fn has_pending(&self) -> bool {
        self.update.is_some() && self.status != Status::Installed
    }

    pub fn progress(&self) -> Option<u32> {
        let percent = self.percent.load(Ordering::Relaxed);
        (percent != UNKNOWN).then_some(percent)
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.status, Status::Installing | Status::Installed)
    }

    /// `silent` at launch; asked from the menu it speaks, « nothing » included.
    pub fn check(&mut self, backend: &Backend, silent: bool) {
        if !self.can_check() || self.status != Status::Idle || self.asked.is_some() {
            return;
        }
        self.check = CheckState::Checking;
        let updater = Arc::clone(&self.updater);
        let pending = backend.compute(move |_| Ok(updater.check()));
        self.asked = Some((silent, pending));
    }

    pub fn install(&mut self, backend: &Backend) {
        if self.status != Status::Available {
            return;
        }
        self.status = Status::Installing;
        self.percent.store(UNKNOWN, Ordering::Relaxed);
        let updater = Arc::clone(&self.updater);
        let percent = Arc::clone(&self.percent);
        self.installing = Some(
            backend
                .compute(move |_| Ok(updater.install(&|at| percent.store(at, Ordering::Relaxed)))),
        );
    }

    /// « Later ». A version is what gets written down, never a boolean: remembering « no » would
    /// silence the release after it too. `true` when the settings changed.
    pub fn dismiss(&mut self, skip: bool, settings: &mut Settings) -> bool {
        if self.is_busy() {
            return false;
        }
        self.status = Status::Idle;
        let Some(update) = self.update.as_ref().filter(|_| skip) else {
            return false;
        };
        settings.skipped_update.clone_from(&update.version);
        true
    }

    pub fn update(&mut self, settings: &Settings, errors: &mut ErrorNotifier) {
        self.adopt_check(settings, errors);
        let Some(answer) = self.installing.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.installing = None;
        if matches!(answer, Ok(Ok(()))) {
            self.status = Status::Installed;
            self.updater.relaunch();
            return;
        }
        // Back to the offer: it can be tried again from the prompt.
        self.status = Status::Available;
        errors.notify(Message::new(keys::errors::UPDATE_FAILED).with("app", METADATA.name));
    }

    fn adopt_check(&mut self, settings: &Settings, errors: &mut ErrorNotifier) {
        let Some((silent, pending)) = &mut self.asked else {
            return;
        };
        let silent = *silent;
        let Some(answer) = pending.poll() else {
            return;
        };
        self.asked = None;
        match answer.unwrap_or_else(|_| Err("the check stopped".to_owned())) {
            Ok(Some(update)) => {
                let hushed = silent && is_silenced(&update, settings);
                self.status = if hushed {
                    Status::Idle
                } else {
                    Status::Available
                };
                self.notes = update.notes.as_deref().map(|notes| ChangelogRelease {
                    version: update.version.clone(),
                    date: None,
                    sections: api::help::release_notes(notes),
                });
                self.update = Some(update);
                self.check = CheckState::Idle;
            }
            Ok(None) => self.check = CheckState::UpToDate,
            Err(detail) => {
                self.check = CheckState::Failed;
                log::warn!("The update check failed: {detail}");
                if !silent {
                    errors.notify(Message::new(keys::errors::UPDATE_CHECK_FAILED));
                }
            }
        }
    }
}

fn is_silenced(update: &AvailableUpdate, settings: &Settings) -> bool {
    !settings.update_notifications || settings.skipped_update == update.version
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::Mutex;

    use super::*;

    /// Answers what a test decided, and remembers what was asked.
    #[derive(Default)]
    pub(crate) struct FakeUpdater {
        pub found: Mutex<Option<AvailableUpdate>>,
        pub fails: Mutex<bool>,
        pub relaunched: Mutex<bool>,
    }

    impl FakeUpdater {
        pub(crate) fn offering(version: &str) -> Arc<Self> {
            let fake = Self::default();
            *fake.found.lock().unwrap() = Some(AvailableUpdate {
                version: version.to_owned(),
                current_version: "0.25.0".to_owned(),
                notes: Some("### Added\n\n- **Faster** search".to_owned()),
            });
            Arc::new(fake)
        }
    }

    impl Updater for FakeUpdater {
        fn is_available(&self) -> bool {
            true
        }

        fn check(&self) -> Result<Option<AvailableUpdate>, String> {
            if *self.fails.lock().unwrap() {
                return Err("offline".to_owned());
            }
            Ok(self.found.lock().unwrap().clone())
        }

        fn install(&self, progress: &dyn Fn(u32)) -> Result<(), String> {
            progress(50);
            if *self.fails.lock().unwrap() {
                return Err("the download stopped".to_owned());
            }
            Ok(())
        }

        fn relaunch(&self) {
            *self.relaunched.lock().unwrap() = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeUpdater;
    use super::*;
    use crate::testing;

    struct Bench {
        backend: Backend,
        _profile: tempfile::TempDir,
        updates: UpdateState,
        settings: Settings,
        errors: ErrorNotifier,
    }

    fn bench(fake: Arc<FakeUpdater>) -> Bench {
        let (backend, profile) = testing::backend();
        Bench {
            backend,
            _profile: profile,
            updates: UpdateState::with(fake),
            settings: Settings::default(),
            errors: ErrorNotifier::default(),
        }
    }

    impl Bench {
        fn settle(&mut self) {
            self.backend.settle();
            self.updates.update(&self.settings, &mut self.errors);
        }

        fn check(&mut self, silent: bool) {
            self.updates.check(&self.backend, silent);
            self.settle();
        }
    }

    #[test]
    fn with_no_updater_nothing_is_checked() {
        let (backend, _profile) = testing::backend();
        let mut updates = UpdateState::default();

        updates.check(&backend, false);

        assert!(!updates.can_check());
        assert_eq!(updates.check_state(), CheckState::Idle);
    }

    #[test]
    fn a_new_version_is_offered_with_its_notes() {
        let mut bench = bench(FakeUpdater::offering("0.26.0"));

        bench.check(true);

        assert_eq!(bench.updates.offered().unwrap().version, "0.26.0");
        assert_eq!(bench.updates.notes().unwrap().sections.len(), 1);
        assert!(bench.updates.has_pending());
    }

    #[test]
    fn nothing_new_is_said_when_it_was_asked() {
        let mut bench = bench(Arc::new(FakeUpdater::default()));

        bench.check(false);

        assert_eq!(bench.updates.check_state(), CheckState::UpToDate);
        assert!(bench.updates.offered().is_none());
    }

    #[test]
    fn a_skipped_version_is_remembered_and_stays_quiet_at_launch_only() {
        let mut bench = bench(FakeUpdater::offering("0.26.0"));
        bench.check(true);

        assert!(bench.updates.dismiss(true, &mut bench.settings));
        assert_eq!(bench.settings.skipped_update, "0.26.0");
        assert!(bench.updates.offered().is_none());
        assert!(bench.updates.has_pending(), "the dot outlives the prompt");

        bench.check(true);
        assert!(bench.updates.offered().is_none(), "silent at launch");

        bench.check(false);
        assert!(bench.updates.offered().is_some(), "asked for, it answers");
    }

    #[test]
    fn a_later_version_than_the_one_skipped_is_offered() {
        let mut bench = bench(FakeUpdater::offering("0.27.0"));
        bench.settings.skipped_update = "0.26.0".to_owned();

        bench.check(true);

        assert!(bench.updates.offered().is_some());
    }

    #[test]
    fn notifications_off_keep_the_launch_quiet_and_the_update_known() {
        let mut bench = bench(FakeUpdater::offering("0.26.0"));
        bench.settings.update_notifications = false;

        bench.check(true);

        assert!(bench.updates.offered().is_none());
        assert!(bench.updates.has_pending());
    }

    #[test]
    fn a_failed_check_is_said_only_when_it_was_asked() {
        let fake = Arc::new(FakeUpdater::default());
        *fake.fails.lock().unwrap() = true;
        let mut bench = bench(fake);

        bench.check(true);
        assert_eq!(bench.updates.check_state(), CheckState::Failed);
        assert!(bench.errors.notice().is_none());

        bench.check(false);
        assert!(bench.errors.notice().is_some());
    }

    #[test]
    fn an_install_goes_to_the_end_and_relaunches() {
        let fake = FakeUpdater::offering("0.26.0");
        let mut bench = bench(Arc::clone(&fake));
        bench.check(true);

        bench.updates.install(&bench.backend);
        assert!(bench.updates.is_busy());
        assert!(
            !bench.updates.dismiss(true, &mut bench.settings),
            "too late"
        );
        bench.settle();

        assert_eq!(bench.updates.status(), Status::Installed);
        assert_eq!(bench.updates.progress(), Some(50));
        assert!(*fake.relaunched.lock().unwrap());
        assert!(!bench.updates.has_pending());
    }

    #[test]
    fn a_failed_install_comes_back_to_the_offer() {
        let fake = FakeUpdater::offering("0.26.0");
        let mut bench = bench(Arc::clone(&fake));
        bench.check(true);
        *fake.fails.lock().unwrap() = true;

        bench.updates.install(&bench.backend);
        bench.settle();

        assert_eq!(bench.updates.status(), Status::Available);
        assert!(bench.errors.notice().is_some());
        assert!(!*fake.relaunched.lock().unwrap());
    }
}
