//! The client itself (D26). What it installs, only a real installation can answer for: the OS
//! checklist's (plan § 8.5).

use std::cell::Cell;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cargo_packager_updater::semver::Version;
use cargo_packager_updater::url::Url;
use cargo_packager_updater::{Config, Error, Update, UpdaterBuilder};

use super::{AvailableUpdate, DownloadError, NoUpdater, Updater};

/// Where the releases are, and who signs them.
pub struct Source<'a> {
    /// The running version.
    pub version: &'a str,
    pub endpoint: &'a str,
    /// The minisign public key, in base64 as `latest.json`'s signatures are.
    pub public_key: &'a str,
}

pub(super) struct System {
    pub(super) current: Version,
    pub(super) config: Config,
    pub(super) found: Mutex<Option<Update>>,
    pub(super) downloaded: Mutex<Option<Vec<u8>>>,
    pub(super) can_install: bool,
}

impl Updater for System {
    fn is_available(&self) -> bool {
        self.can_install
    }

    fn check(&self) -> Result<Option<AvailableUpdate>, String> {
        let found = UpdaterBuilder::new(self.current.clone(), self.config.clone())
            .build()
            .and_then(|updater| updater.check())
            .map_err(|error| error.to_string())?;
        let offered = found.as_ref().map(|update| AvailableUpdate {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone(),
        });
        if let Ok(mut kept) = self.found.lock() {
            *kept = found;
        }
        Ok(offered)
    }

    fn download(&self, progress: &dyn Fn(u32)) -> Result<(), DownloadError> {
        let found = self.found.lock().ok().and_then(|found| found.clone());
        let Some(update) = found else {
            return Err(DownloadError::Failed("nothing to download".to_owned()));
        };
        let received = Cell::new(0_u64);
        let on_chunk = |chunk: usize, total: Option<u64>| {
            received.set(received.get() + chunk as u64);
            if let Some(total) = total.filter(|total| *total > 0) {
                let percent = (received.get() * 100 / total).min(100);
                progress(u32::try_from(percent).unwrap_or(100));
            }
        };
        let bytes = update
            .download_extended(on_chunk, || {})
            .map_err(|error| match error {
                Error::Minisign(_) => DownloadError::Refused(error.to_string()),
                _ => DownloadError::Failed(error.to_string()),
            })?;
        if let Ok(mut kept) = self.downloaded.lock() {
            *kept = Some(bytes);
        }
        Ok(())
    }

    fn install_and_relaunch(&self) {
        let update = self.found.lock().ok().and_then(|mut found| found.take());
        let bytes = self
            .downloaded
            .lock()
            .ok()
            .and_then(|mut bytes| bytes.take());
        let (Some(update), Some(bytes)) = (update, bytes) else {
            return;
        };
        match update.install(bytes) {
            Ok(()) => come_back(),
            Err(error) => log::error!("The update was not installed: {error}"),
        }
    }
}

/// The installer relaunches under Windows (`/R`), and the process has ended by now.
#[cfg(not(target_os = "linux"))]
fn come_back() {}

/// Started only once this process is gone: one that came up beside it would find the profile
/// taken, hand over to it, and leave nothing running.
#[cfg(target_os = "linux")]
fn come_back() {
    const AFTER_US: &str =
        r#"while kill -0 "$1" 2>/dev/null; do sleep 0.1; done; shift; exec "$@""#;
    let Some(image) = std::env::var_os("APPIMAGE") else {
        return;
    };
    let started = std::process::Command::new("sh")
        .args(["-c", AFTER_US, "sh"])
        .arg(std::process::id().to_string())
        .arg(image)
        .args(std::env::args_os().skip(1))
        .spawn();
    if let Err(error) = started {
        log::error!("The application was not started again: {error}");
    }
}

/// Only what an installer put in place replaces itself: an `AppImage`, or a folder with the NSIS
/// uninstaller in it. A build run from `target/` asks nobody for anything.
fn installs_itself() -> bool {
    if cfg!(target_os = "linux") {
        return std::env::var_os("APPIMAGE").is_some();
    }
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(|folder| folder.join("uninstall.exe").is_file())
}

pub(super) fn built(source: &Source<'_>, can_install: bool) -> Option<System> {
    let current = Version::parse(source.version).ok()?;
    let endpoint = Url::parse(source.endpoint).ok()?;
    Some(System {
        current,
        config: Config {
            endpoints: vec![endpoint],
            pubkey: source.public_key.to_owned(),
            windows: None,
        },
        found: Mutex::new(None),
        downloaded: Mutex::new(None),
        can_install,
    })
}

pub fn system(source: &Source<'_>) -> Arc<dyn Updater + Send + Sync> {
    if let Some(system) = built(source, installs_itself()) {
        return Arc::new(system);
    }
    log::warn!("No updater: its version or its address does not read");
    Arc::new(NoUpdater)
}
