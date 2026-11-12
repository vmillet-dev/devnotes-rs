//! The log file, and what a panic leaves in it.

use std::panic::PanicHookInfo;
use std::path::Path;

use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming};

/// Kept by `main` until the end: dropping the handle stops the logger. `None` when the
/// directory cannot be written, and the application runs without a log.
pub(crate) fn start(directory: &Path) -> Option<LoggerHandle> {
    let file = FileSpec::default()
        .directory(directory)
        .basename("devnotes")
        .suppress_timestamp();
    Logger::try_with_str("info")
        .and_then(|logger| {
            logger
                .log_to_file(file)
                .append()
                .rotate(
                    Criterion::Size(5_000_000),
                    Naming::Numbers,
                    Cleanup::KeepLogFiles(3),
                )
                .duplicate_to_stderr(Duplicate::Warn)
                .start()
        })
        .inspect_err(|error| eprintln!("devnotes: no log file: {error}"))
        .ok()
}

/// Replaces the default hook, which prints the message: a panic slicing a string quotes the
/// string, a note's text perhaps (D18).
pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| log::error!("{}", report(info))));
}

/// Where and on which thread, never the payload.
fn report(info: &PanicHookInfo<'_>) -> String {
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    match info.location() {
        Some(at) => format!(
            "Panicked at {}:{}:{} on thread {name}",
            at.file(),
            at.line(),
            at.column()
        ),
        None => format!("Panicked on thread {name}"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, OnceLock};

    use engine::notes::model::NotePatch;
    use tempfile::TempDir;

    use super::*;

    /// One logger a process: the tests share it, and each looks for its own line.
    fn logger() -> &'static (TempDir, LoggerHandle) {
        static LOGGER: OnceLock<(TempDir, LoggerHandle)> = OnceLock::new();
        LOGGER.get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let logger = start(directory.path()).unwrap();
            (directory, logger)
        })
    }

    fn written() -> String {
        let (directory, logger) = logger();
        logger.flush();
        std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect()
    }

    #[test]
    fn the_log_is_written_in_its_directory() {
        logger();

        log::warn!("a line the test looks for");

        let written = written();
        assert!(written.contains("a line the test looks for"), "{written}");
    }

    #[test]
    fn a_failing_operation_is_logged_without_the_text_of_the_note() {
        logger();
        let profile = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let paths = engine::Paths {
            profile: profile.path().to_path_buf(),
            resources: profile.path().to_path_buf(),
        };
        let sink = Arc::new(engine::RecordingSink::default());
        let closed = engine::Engine::new(paths, sink, runtime.handle().clone());
        let patch = NotePatch {
            title: Some("title never logged".to_owned()),
            content: Some("psql -h prod -W hunter2".to_owned()),
            ..Default::default()
        };

        let failed = engine::db::run_here(&closed, |engine| {
            engine::notes::update_note(engine, "note-id".to_owned(), patch)
        });
        ui::services::errors::ErrorNotifier::default()
            .report(i18n::keys::errors::NOTE_GONE, &failed.unwrap_err());

        let written = written();
        assert!(written.contains("errors.noteGone"), "{written}");
        assert!(!written.contains("hunter2"), "{written}");
        assert!(!written.contains("title never logged"), "{written}");
    }

    #[test]
    fn a_panic_is_reported_by_its_place_and_never_by_its_message() {
        let reports = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&reports);
        std::panic::set_hook(Box::new(move |info| {
            kept.lock().unwrap().push(report(info));
        }));

        let outcome = std::panic::catch_unwind(|| panic!("psql -h prod -W hunter2"));
        let _ = std::panic::take_hook();

        assert!(outcome.is_err());
        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].contains("logging.rs"), "{}", reports[0]);
        assert!(!reports[0].contains("hunter2"), "{}", reports[0]);
    }
}
