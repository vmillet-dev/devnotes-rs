//! A new version: found in `latest.json`, downloaded and checked against its minisign signature,
//! then installed once the application has finished leaving (plan § 4.3, D26). Nothing installs
//! without an explicit gesture, and deb and rpm packages do not update themselves.

mod system;
#[cfg(test)]
mod tests;

pub use system::{Source, system};

/// A version the updater found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    /// The file is not what was signed, or not by our key: nothing is installed.
    Refused(String),
    Failed(String),
}

/// ⚠️ Each blocks until it is done: never on the window's thread.
pub trait Updater {
    /// Whether this installation can update itself at all.
    fn is_available(&self) -> bool;
    /// `None` when the application is up to date.
    fn check(&self) -> Result<Option<AvailableUpdate>, String>;
    /// The version last found, checked and kept. `progress` in percent, when the size is known.
    fn download(&self, progress: &dyn Fn(u32)) -> Result<(), DownloadError>;
    /// ⚠️ The very last thing the process does: under Windows it ends there and then, so
    /// everything is written and the library closed before this is called.
    fn install_and_relaunch(&self);
}

/// An installation that does not update itself, and the tests that have no use for one.
pub struct NoUpdater;

impl Updater for NoUpdater {
    fn is_available(&self) -> bool {
        false
    }

    fn check(&self) -> Result<Option<AvailableUpdate>, String> {
        Ok(None)
    }

    fn download(&self, _progress: &dyn Fn(u32)) -> Result<(), DownloadError> {
        Err(DownloadError::Failed("no updater".to_owned()))
    }

    fn install_and_relaunch(&self) {}
}
