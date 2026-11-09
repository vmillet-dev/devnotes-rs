//! The updater of the application, run on an installed package by `packages.yml`:
//! `update <running version> <latest.json URL> <public key>`. It checks, downloads, verifies and
//! installs as the update prompt does; exits 2 when the signature refuses the file, 3 when
//! nothing newer is offered.

use std::process::ExitCode;

use platform::updater::{DownloadError, Source};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [version, endpoint, public_key] = arguments.as_slice() else {
        eprintln!("usage: update <running version> <latest.json URL> <public key>");
        return ExitCode::from(64);
    };
    let updater = platform::updater::system(&Source {
        version,
        endpoint,
        public_key,
    });
    match updater.check() {
        Ok(Some(update)) => println!("offered: {}", update.version),
        Ok(None) => {
            eprintln!("nothing newer than {version}");
            return ExitCode::from(3);
        }
        Err(error) => {
            eprintln!("check failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    match updater.download(&|_| {}) {
        Ok(()) => println!("downloaded and verified"),
        Err(DownloadError::Refused(reason)) => {
            eprintln!("refused: {reason}");
            return ExitCode::from(2);
        }
        Err(DownloadError::Failed(reason)) => {
            eprintln!("download failed: {reason}");
            return ExitCode::FAILURE;
        }
    }
    // Under Windows the process ends in there, the installer running on its own.
    updater.install_and_relaunch();
    println!("installed");
    ExitCode::SUCCESS
}
