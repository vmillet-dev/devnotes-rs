//! Import, export and share, one at a time. Every operation reports, including when it changed
//! nothing: that case alone cannot be told from a failure without a report. The reports go under
//! the title bar, since the menu closes on the click.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use engine::error::{AppError, ErrorCode};
use engine::transfer::model::{ExportReport, ExportScope, ImportReport};
use i18n::{Key, Message, keys};
use platform::dialog::FileDialog;
use zeroize::Zeroizing;

use super::attachments::Reports;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::gate::prompt::{Answer, Purpose, Request};

type Dialog = Arc<dyn FileDialog + Send + Sync>;

enum Stage {
    Idle,
    /// Counted before any dialog: an export of nothing is said before a place is asked.
    Counting {
        scope: ExportScope,
        count: Pending<u32>,
    },
    Placing {
        scope: ExportScope,
        place: Pending<Option<PathBuf>>,
    },
    Protecting {
        scope: ExportScope,
        path: String,
    },
    Writing {
        scope: ExportScope,
        path: String,
        report: Pending<ExportReport>,
    },
    Picking(Pending<Option<PathBuf>>),
    Probing {
        path: String,
        protected: Pending<bool>,
    },
    Unlocking {
        path: String,
        refused: bool,
    },
    Reading {
        path: String,
        protected: bool,
        report: Pending<ImportReport>,
    },
    Sharing {
        notes: u32,
        markdown: Pending<String>,
    },
}

pub struct TransferState {
    stage: Stage,
}

impl Default for TransferState {
    fn default() -> Self {
        Self { stage: Stage::Idle }
    }
}

impl TransferState {
    pub fn is_busy(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }

    pub fn import(&mut self, backend: &Backend, dialog: &Dialog) {
        if !self.is_busy() {
            self.stage = Stage::Picking(api::transfer::pick_bundle(backend, Arc::clone(dialog)));
        }
    }

    pub fn export(&mut self, backend: &Backend, scope: ExportScope) {
        if !self.is_busy() {
            let count = api::transfer::count(backend, scope.clone());
            self.stage = Stage::Counting { scope, count };
        }
    }

    /// Sharing stops at the clipboard: nothing is sent anywhere.
    pub fn share(&mut self, backend: &Backend, ids: Vec<String>) {
        if !self.is_busy() {
            let notes = u32::try_from(ids.len()).unwrap_or(u32::MAX);
            let markdown = api::transfer::share(backend, ids);
            self.stage = Stage::Sharing { notes, markdown };
        }
    }

    /// What the passphrase prompt shows. It never outlives the operation it was opened for, and
    /// stays up, working, while the key is derived.
    pub fn prompt(&self) -> Option<Request> {
        let (purpose, path, refused, working) = match &self.stage {
            Stage::Protecting { path, .. } => (Purpose::Protect, path, false, false),
            Stage::Writing { path, .. } => (Purpose::Protect, path, false, true),
            Stage::Unlocking { path, refused } => (Purpose::Unlock, path, *refused, false),
            Stage::Reading {
                path,
                protected: true,
                ..
            } => (Purpose::Unlock, path, false, true),
            _ => return None,
        };
        Some(Request {
            purpose,
            file_name: file_name(path),
            refused,
            working,
        })
    }

    pub fn answer(&mut self, answer: Answer, backend: &Backend) {
        let passphrase = match answer {
            Answer::Cancel => {
                if self.prompt().is_some_and(|request| !request.working) {
                    self.stage = Stage::Idle;
                }
                return;
            }
            Answer::Plain => None,
            Answer::Phrase(phrase) => Some(plain(phrase)),
        };
        match std::mem::replace(&mut self.stage, Stage::Idle) {
            Stage::Protecting { scope, path } => {
                let report =
                    api::transfer::export(backend, path.clone(), scope.clone(), passphrase);
                self.stage = Stage::Writing {
                    scope,
                    path,
                    report,
                };
            }
            Stage::Unlocking { path, .. } => {
                self.stage = reading(backend, path, passphrase, true);
            }
            other => self.stage = other,
        }
    }

    /// Once a frame, with the library open: the answers that arrived move the operation on.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        dialog: &Dialog,
        now: DateTime<Utc>,
        reports: &mut Reports<'_>,
    ) {
        self.stage = match std::mem::replace(&mut self.stage, Stage::Idle) {
            Stage::Counting { scope, count } => {
                counted(scope, count, (backend, dialog, now), reports)
            }
            Stage::Placing { scope, place } => placed(scope, place, reports),
            Stage::Writing {
                scope,
                path,
                report,
            } => written(scope, path, report, reports),
            Stage::Picking(path) => picked(path, backend, reports),
            Stage::Probing { path, protected } => probed(path, protected, backend, reports),
            Stage::Reading {
                path,
                protected,
                report,
            } => read(path, protected, report, reports),
            Stage::Sharing { notes, markdown } => shared(notes, markdown, ctx, reports),
            waiting => waiting,
        };
    }
}

fn counted(
    scope: ExportScope,
    mut count: Pending<u32>,
    (backend, dialog, now): (&Backend, &Dialog, DateTime<Utc>),
    reports: &mut Reports<'_>,
) -> Stage {
    match count.poll() {
        None => Stage::Counting { scope, count },
        Some(Ok(0)) => {
            reports.status.report(Message::new(empty_key(&scope)));
            Stage::Idle
        }
        Some(Ok(_)) => {
            let name = default_file_name(now.with_timezone(&Local));
            let place = api::transfer::choose_destination(backend, Arc::clone(dialog), name);
            Stage::Placing { scope, place }
        }
        Some(Err(error)) => failed(reports, keys::errors::EXPORT_FAILED, &error),
    }
}

fn placed(
    scope: ExportScope,
    mut place: Pending<Option<PathBuf>>,
    reports: &mut Reports<'_>,
) -> Stage {
    match place.poll() {
        None => Stage::Placing { scope, place },
        Some(Ok(Some(path))) => Stage::Protecting {
            scope,
            path: path.to_string_lossy().into_owned(),
        },
        Some(Ok(None)) => Stage::Idle,
        Some(Err(error)) => failed(reports, keys::errors::EXPORT_FAILED, &error),
    }
}

fn written(
    scope: ExportScope,
    path: String,
    mut report: Pending<ExportReport>,
    reports: &mut Reports<'_>,
) -> Stage {
    match report.poll() {
        None => Stage::Writing {
            scope,
            path,
            report,
        },
        Some(Ok(report)) => {
            reports.status.report(exported(&report, &scope, &path));
            Stage::Idle
        }
        Some(Err(error)) => failed(reports, keys::errors::EXPORT_FAILED, &error),
    }
}

fn picked(
    mut path: Pending<Option<PathBuf>>,
    backend: &Backend,
    reports: &mut Reports<'_>,
) -> Stage {
    match path.poll() {
        None => Stage::Picking(path),
        Some(Ok(Some(path))) => {
            let path = path.to_string_lossy().into_owned();
            let protected = api::transfer::is_protected(backend, path.clone());
            Stage::Probing { path, protected }
        }
        Some(Ok(None)) => Stage::Idle,
        Some(Err(error)) => failed(reports, keys::errors::IMPORT_FAILED, &error),
    }
}

fn probed(
    path: String,
    mut protected: Pending<bool>,
    backend: &Backend,
    reports: &mut Reports<'_>,
) -> Stage {
    match protected.poll() {
        None => Stage::Probing { path, protected },
        Some(Ok(true)) => Stage::Unlocking {
            path,
            refused: false,
        },
        Some(Ok(false)) => reading(backend, path, None, false),
        Some(Err(error)) => failed(reports, keys::errors::IMPORT_FAILED, &error),
    }
}

fn read(
    path: String,
    protected: bool,
    mut report: Pending<ImportReport>,
    reports: &mut Reports<'_>,
) -> Stage {
    match report.poll() {
        None => Stage::Reading {
            path,
            protected,
            report,
        },
        Some(Ok(report)) => {
            reports.status.report(imported(&report, &path));
            // The spaces and the folders follow the revision, as the canvas does.
            if report.notes_imported > 0 || report.spaces_created > 0 || report.folders_created > 0
            {
                reports.revision.bump();
            }
            Stage::Idle
        }
        // The ordinary answer to a typo: a file nobody can reopen for one is a file lost.
        Some(Err(error)) if protected && error.code == ErrorCode::WrongPassphrase => {
            Stage::Unlocking {
                path,
                refused: true,
            }
        }
        Some(Err(error)) => failed(reports, keys::errors::IMPORT_FAILED, &error),
    }
}

fn shared(
    notes: u32,
    mut markdown: Pending<String>,
    ctx: &egui::Context,
    reports: &mut Reports<'_>,
) -> Stage {
    match markdown.poll() {
        None => Stage::Sharing { notes, markdown },
        Some(Ok(markdown)) => {
            ctx.copy_text(markdown);
            reports
                .status
                .report(Message::new(keys::file::COPIED).with("notes", notes));
            Stage::Idle
        }
        Some(Err(error)) => failed(reports, keys::errors::SHARE_FAILED, &error),
    }
}

fn reading(backend: &Backend, path: String, passphrase: Option<String>, protected: bool) -> Stage {
    let report = api::transfer::import(backend, path.clone(), passphrase);
    Stage::Reading {
        path,
        protected,
        report,
    }
}

fn failed(reports: &mut Reports<'_>, key: Key, error: &AppError) -> Stage {
    reports.errors.report(key, error);
    Stage::Idle
}

/// The core takes the phrase as a `String` and wipes it: moved, never copied.
fn plain(mut phrase: Zeroizing<String>) -> String {
    std::mem::take(&mut *phrase)
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Stamped to the second, and in local time: to the minute, two exports in a row were offered the
/// same name, and replacing the first was one Enter away.
fn default_file_name(now: DateTime<Local>) -> String {
    now.format("devnotes-%Y-%m-%d-%H%M%S.devnotes").to_string()
}

fn empty_key(scope: &ExportScope) -> Key {
    match scope {
        ExportScope::Space { .. } => keys::file::EMPTY_SPACE,
        _ => keys::file::EMPTY_LIBRARY,
    }
}

/// Which of the two files was written is part of the report; so is where it landed.
fn exported(report: &ExportReport, scope: &ExportScope, path: &str) -> Message {
    // Emptied between the count and the write: the core wrote no file.
    if report.notes == 0 {
        return Message::new(empty_key(scope));
    }
    let key = match (report.protected, report.attachments > 0) {
        (true, true) => keys::file::EXPORTED_PROTECTED_WITH_ATTACHMENTS,
        (true, false) => keys::file::EXPORTED_PROTECTED,
        (false, true) => keys::file::EXPORTED_WITH_ATTACHMENTS,
        (false, false) => keys::file::EXPORTED,
    };
    Message::new(key)
        .with("notes", report.notes)
        .with("attachments", report.attachments)
        .with("path", file_name(path))
}

/// A ladder of what most deserves saying: an attachment the archive named and did not carry
/// leaves a preview that will never load, and only this says why.
fn imported(report: &ImportReport, path: &str) -> Message {
    let key = if report.notes_imported == 0 {
        keys::file::IMPORTED_NOTHING
    } else if report.attachments_missing > 0 {
        keys::file::IMPORTED_WITHOUT_SOME_ATTACHMENTS
    } else if report.notes_degraded > 0 {
        keys::file::IMPORTED_FROM_NEWER_VERSION
    } else if report.folders_created > 0 {
        keys::file::IMPORTED_INTO_FOLDERS
    } else if report.attachments_imported > 0 {
        keys::file::IMPORTED_WITH_ATTACHMENTS
    } else {
        keys::file::IMPORTED
    };
    Message::new(key)
        .with("notes", report.notes_imported)
        .with("skipped", report.notes_skipped)
        .with("degraded", report.notes_degraded)
        .with("attachments", report.attachments_imported)
        .with("missing", report.attachments_missing)
        .with("folders", report.folders_created)
        .with("path", file_name(path))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use platform::dialog::Fixed;

    use super::*;
    use crate::services::errors::ErrorNotifier;
    use crate::services::status::StatusNotifier;
    use crate::state::Revision;
    use crate::testing;

    const PHRASE: &str = "an export passphrase";

    struct Bench {
        backend: Backend,
        _profile: tempfile::TempDir,
        ctx: egui::Context,
        dialog: Dialog,
        file: PathBuf,
        _directory: tempfile::TempDir,
        transfer: TransferState,
        status: StatusNotifier,
        errors: ErrorNotifier,
        revision: Revision,
    }

    impl Bench {
        fn new() -> Self {
            let (backend, profile) = testing::unlocked();
            let directory = tempfile::tempdir().unwrap();
            let file = directory.path().join("library.devnotes");
            Self {
                backend,
                _profile: profile,
                ctx: egui::Context::default(),
                dialog: Arc::new(Fixed(file.clone())),
                file,
                _directory: directory,
                transfer: TransferState::default(),
                status: StatusNotifier::default(),
                errors: ErrorNotifier::default(),
                revision: Revision::default(),
            }
        }

        /// Until the operation stops, or stands at the prompt.
        fn run(&mut self) {
            while self.transfer.is_busy()
                && self.transfer.prompt().is_none_or(|request| request.working)
            {
                self.backend.settle();
                let mut reports = Reports {
                    status: &mut self.status,
                    errors: &mut self.errors,
                    revision: &mut self.revision,
                };
                self.transfer.update(
                    &self.ctx,
                    &self.backend,
                    &self.dialog,
                    Utc::now(),
                    &mut reports,
                );
            }
        }

        fn export(&mut self) {
            self.transfer.export(&self.backend, ExportScope::Library);
            self.run();
        }

        fn import(&mut self) {
            self.transfer.import(&self.backend, &self.dialog);
            self.run();
        }

        fn answer(&mut self, answer: Answer) {
            self.transfer.answer(answer, &self.backend);
            self.run();
        }

        fn said(&self) -> Option<Key> {
            self.status.latest().map(|(_, message)| message.key)
        }
    }

    fn phrase(words: &str) -> Answer {
        Answer::Phrase(Zeroizing::new(words.to_owned()))
    }

    #[test]
    fn an_export_of_nothing_asks_nothing_and_writes_nothing() {
        let mut bench = Bench::new();

        bench.export();

        assert_eq!(bench.said(), Some(keys::file::EMPTY_LIBRARY));
        assert!(bench.transfer.prompt().is_none());
        assert!(!bench.file.exists());
    }

    #[test]
    fn an_export_asks_for_a_phrase_and_says_it_went_out_in_the_clear() {
        let mut bench = Bench::new();
        testing::note(&bench.backend, "Worth exporting");

        bench.export();
        let request = bench.transfer.prompt().unwrap();
        assert_eq!(request.purpose, Purpose::Protect);
        assert_eq!(request.file_name, "library.devnotes");
        bench.answer(Answer::Plain);

        assert_eq!(bench.said(), Some(keys::file::EXPORTED));
        assert!(bench.transfer.prompt().is_none());
        assert_eq!(&std::fs::read(&bench.file).unwrap()[..2], b"PK");
    }

    #[test]
    fn a_cancelled_export_writes_nothing() {
        let mut bench = Bench::new();
        testing::note(&bench.backend, "Worth exporting");

        bench.export();
        bench.answer(Answer::Cancel);

        assert!(!bench.transfer.is_busy());
        assert!(bench.status.latest().is_none());
        assert!(!bench.file.exists());
    }

    #[test]
    fn importing_what_is_already_there_says_so() {
        let mut bench = Bench::new();
        testing::note(&bench.backend, "Worth exporting");
        bench.export();
        bench.answer(Answer::Plain);
        let before = bench.revision;

        bench.import();

        assert_eq!(bench.said(), Some(keys::file::IMPORTED_NOTHING));
        assert_eq!(bench.revision, before);
    }

    #[test]
    fn a_refused_phrase_asks_again_and_the_right_one_imports() {
        let mut bench = Bench::new();
        testing::note(&bench.backend, "Worth exporting");
        bench.export();
        bench.answer(phrase(PHRASE));
        assert_eq!(bench.said(), Some(keys::file::EXPORTED_PROTECTED));

        bench.import();
        let request = bench.transfer.prompt().unwrap();
        assert_eq!(request.purpose, Purpose::Unlock);
        assert!(!request.refused);

        bench.answer(phrase("not the phrase"));
        assert!(bench.transfer.prompt().unwrap().refused);

        bench.answer(phrase(PHRASE));
        assert_eq!(bench.said(), Some(keys::file::IMPORTED_NOTHING));
        assert!(bench.transfer.prompt().is_none());
    }

    #[test]
    fn a_file_that_does_not_read_fails_the_import_and_says_so() {
        let mut bench = Bench::new();
        std::fs::write(&bench.file, "{ half").unwrap();

        bench.import();

        assert!(bench.errors.notice().is_some());
        assert!(!bench.transfer.is_busy());
    }

    #[test]
    fn a_share_lands_on_the_clipboard_and_is_said() {
        let mut bench = Bench::new();
        let note = testing::note(&bench.backend, "Worth sharing");

        bench.transfer.share(&bench.backend, vec![note.id.clone()]);
        bench.run();

        assert_eq!(bench.said(), Some(keys::file::COPIED));
    }

    #[test]
    fn the_name_offered_is_stamped_to_the_second() {
        let now = DateTime::parse_from_rfc3339("2026-11-06T09:05:07+01:00")
            .unwrap()
            .with_timezone(&Local);

        let name = default_file_name(now);

        assert!(name.starts_with("devnotes-2026-11-0"), "{name}");
        assert!(name.ends_with(".devnotes"), "{name}");
    }

    #[test]
    fn a_second_operation_waits_for_the_first() {
        let mut bench = Bench::new();
        testing::note(&bench.backend, "Worth exporting");
        bench.export();

        bench.transfer.import(&bench.backend, &bench.dialog);

        assert!(bench.transfer.prompt().is_some());
    }
}
