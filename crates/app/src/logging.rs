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
    use std::sync::{Arc, Mutex};

    use super::*;

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
