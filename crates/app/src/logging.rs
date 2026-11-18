//! The log file, and what a panic leaves in it.

use std::panic::PanicHookInfo;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// What a process that panicked exits with, as Rust's own runtime does.
const PANICKED: i32 = 101;

/// What the person is told when the application stops: set once the language is known.
static NOTICE: OnceLock<(String, String)> = OnceLock::new();
static STOPPING: AtomicBool = AtomicBool::new(false);

/// A panic ends the process, on whatever thread (D18): the place is logged, never the message
/// — a panic slicing a string quotes it, a note's text perhaps —, the person is told, and the
/// process exits. What was written is in SQLite's journal; the next open checks it (the gate).
/// ⚠️ It leaves by `exit`: the abort that would follow hands the process, the open library in
/// its memory, to the system's crash reports.
/// The one exception is the tray's thread (`platform::tray::THREAD`), which a third-party load
/// makes panic on a desktop without an indicator: that thread is parked for good — returning
/// would let the abort follow — and the application goes on without a tray. ⚠️ It holds the
/// hook's lock: nothing may set or take the hook after.
pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("{}", report(info));
        log::logger().flush();
        if contained(std::thread::current().name()) {
            loop {
                std::thread::park();
            }
        }
        // A second panic, the dialog's say, ends at once.
        if STOPPING.swap(true, Ordering::SeqCst) {
            std::process::exit(PANICKED);
        }
        if let Some((title, words)) = NOTICE.get() {
            platform::dialog::stopped(title, words);
        }
        std::process::exit(PANICKED);
    }));
}

/// Whether a panic on the thread named `name` stops that thread alone.
fn contained(name: Option<&str>) -> bool {
    name == Some(platform::tray::THREAD)
}

/// The window's process says why it stopped, in `language`, naming the folder of its log.
pub(crate) fn tell_when_stopped(language: i18n::Language, logs: &Path) {
    let _ = NOTICE.set(notice(language, logs));
}

fn notice(language: i18n::Language, logs: &Path) -> (String, String) {
    use i18n::{Value, keys};

    let app = || Value::from(engine::app_info::METADATA.name.to_owned());
    let title = i18n::translate(language, keys::crash::TITLE, &[("app", app())]);
    let folder = Value::from(logs.display().to_string());
    let words = i18n::translate(
        language,
        keys::crash::BODY,
        &[("app", app()), ("folder", folder)],
    );
    (title, words)
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
        };
        let sink = Arc::new(engine::RecordingSink::default());
        let closed = engine::Engine::new(paths, sink, runtime.handle().clone());
        let patch = NotePatch {
            title: Some("title never logged".to_owned()),
            content: Some("psql -h prod -W hunter2".to_owned()),
            ..Default::default()
        };

        let failed = engine::db::run_here(&closed, |engine| {
            engine::notes::update_note(engine, "note-id", &patch)
        });
        ui::services::errors::ErrorNotifier::default()
            .report(i18n::keys::errors::NOTE_GONE, &failed.unwrap_err());

        let written = written();
        assert!(written.contains("errors.noteGone"), "{written}");
        assert!(!written.contains("hunter2"), "{written}");
        assert!(!written.contains("title never logged"), "{written}");
    }

    #[test]
    fn an_error_is_logged_without_what_was_typed() {
        logger();
        let profile = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let paths = engine::Paths {
            profile: profile.path().to_path_buf(),
        };
        let sink = Arc::new(engine::RecordingSink::default());
        let engine = engine::Engine::new(paths, sink, runtime.handle().clone());
        let draft = || engine::spaces::model::SpaceDraft {
            name: "Clients Dupont-Aignan".to_owned(),
        };
        let duplicate = engine::db::run_here(&engine, |engine| {
            let phrase = engine::vault::secret("correct horse battery staple".to_owned());
            engine::vault::create_vault(engine, phrase)?;
            engine::spaces::create_space(engine, &draft())?;
            engine::spaces::create_space(engine, &draft())
        })
        .unwrap_err();
        let too_large = engine::error::StorageError::TooLarge {
            name: "contrat-signé-dupont.pdf".to_owned(),
            limit_mb: 10,
        };
        let stop = engine::http::scripts::model::ScriptStop {
            message: "token ghp_s3cr3t refused".to_owned(),
            ..Default::default()
        };
        let script = engine::http::scripts::model::ScriptError::Stopped(stop);

        let mut notifier = ui::services::errors::ErrorNotifier::default();
        notifier.report(i18n::keys::errors::NOTE_GONE, &duplicate);
        notifier.report(i18n::keys::errors::NOTE_GONE, &too_large.into());
        notifier.report(i18n::keys::errors::NOTE_GONE, &script.into());

        let written = written();
        for typed in ["Dupont-Aignan", "contrat-signé", "ghp_s3cr3t"] {
            assert!(!written.contains(typed), "{typed} in {written}");
        }
        assert!(written.contains("DuplicateSpaceName"), "{written}");
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

    #[test]
    fn the_person_is_told_where_the_log_is_in_their_language() {
        let logs = Path::new("C:/profil/logs");

        let (title, words) = notice(i18n::Language::Fr, logs);
        let (_, english) = notice(i18n::Language::En, logs);

        assert!(title.contains(engine::app_info::METADATA.name), "{title}");
        assert!(words.contains("C:/profil/logs"), "{words}");
        assert_ne!(words, english);
        assert!(
            !words.contains("{{") && !english.contains("{{"),
            "{words} {english}"
        );
    }

    #[test]
    fn only_the_tray_s_thread_is_stopped_alone() {
        assert!(contained(Some(platform::tray::THREAD)));
        assert!(!contained(Some("db")));
        assert!(!contained(Some("main")));
        assert!(!contained(None));
    }

    /// Where the process that panics for the test below writes its log.
    const PANIC_IN: &str = "DEVNOTES_TEST_PANIC_IN";

    /// Run again as a process of its own, which the hook ends: its log is read from here.
    #[test]
    fn a_panic_ends_the_process_once_its_place_is_in_the_log() {
        if let Some(directory) = std::env::var_os(PANIC_IN) {
            let _logger = start(Path::new(&directory));
            install_panic_hook();
            panic!("psql -h prod -W hunter2");
        }
        let directory = tempfile::tempdir().unwrap();

        let ended = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "logging::tests::a_panic_ends_the_process_once_its_place_is_in_the_log",
            ])
            .env(PANIC_IN, directory.path())
            .output()
            .unwrap();

        let said = String::from_utf8_lossy(&ended.stdout);
        assert_eq!(ended.status.code(), Some(PANICKED));
        assert!(!said.contains("test result"), "the harness went on: {said}");
        let written: String = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert!(written.contains("Panicked at"), "{written}");
        assert!(written.contains("logging.rs"), "{written}");
        assert!(!written.contains("hunter2"), "{written}");
    }

    /// Where the process whose tray panics, for the test below, writes its log.
    const TRAY_PANIC_IN: &str = "DEVNOTES_TEST_TRAY_PANIC_IN";

    #[test]
    fn a_panic_on_the_tray_s_thread_leaves_the_application_running() {
        if let Some(directory) = std::env::var_os(TRAY_PANIC_IN) {
            let _logger = start(Path::new(&directory));
            install_panic_hook();
            let (fell, fallen) = std::sync::mpsc::channel::<()>();
            std::thread::Builder::new()
                .name(platform::tray::THREAD.to_owned())
                .spawn(move || {
                    let _told = fell;
                    panic!("Failed to load ayatana-appindicator3");
                })
                .unwrap();
            // The tray's thread parks: its sender is never dropped, and the wait times out.
            let parked = fallen
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_err();
            log::warn!("still running, tray parked: {parked}");
            log::logger().flush();
            // The parked thread holds the hook's lock, which the harness would wait for.
            std::process::exit(0);
        }
        let directory = tempfile::tempdir().unwrap();

        let ended = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "logging::tests::a_panic_on_the_tray_s_thread_leaves_the_application_running",
            ])
            .env(TRAY_PANIC_IN, directory.path())
            .output()
            .unwrap();

        assert!(ended.status.success(), "{ended:?}");
        let written: String = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert!(written.contains("on thread tray"), "{written}");
        assert!(
            written.contains("still running, tray parked: true"),
            "{written}"
        );
    }
}
