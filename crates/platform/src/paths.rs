//! Where the application reads and writes, resolved once at launch (plan § 4.1).

use std::path::PathBuf;

/// The installed profile, named after the packages' identifier (`[package.metadata.packager]`).
/// Until 1.0.0 it was `com.devnotes.app.dev`, kept apart from the Angular application (D16).
pub const DEFAULT_PROFILE: &str = "com.devnotes.app";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    /// The registry, `libraries/` and the application's `preferences.json`.
    pub profile: PathBuf,
    pub logs: PathBuf,
    /// The window's size and place, a file of its own.
    pub window: PathBuf,
}

impl Locations {
    /// The installed layout of `identifier`: roaming data on Windows, the XDG directories on
    /// Linux. `None` when the system names no home for them.
    pub fn installed(identifier: &str) -> Option<Self> {
        Some(Self {
            profile: dirs::data_dir()?.join(identifier),
            logs: dirs::data_local_dir()?.join(identifier).join("logs"),
            window: dirs::config_dir()?.join(identifier).join(WINDOW_FILE),
        })
    }

    /// Everything inside `directory`, the `--profile` of tests and development: nothing of a
    /// run is written anywhere else.
    pub fn inside(directory: PathBuf) -> Self {
        Self {
            logs: directory.join("logs"),
            window: directory.join(WINDOW_FILE),
            profile: directory,
        }
    }
}

const WINDOW_FILE: &str = "window.json";

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn a_profile_directory_holds_every_file_of_the_run() {
        let directory = PathBuf::from("run");

        let locations = Locations::inside(directory.clone());

        assert_eq!(locations.profile, directory);
        assert!(locations.logs.starts_with(&directory));
        assert!(locations.window.starts_with(&directory));
    }

    #[test]
    fn the_installed_profile_is_named_by_its_identifier() {
        let Some(locations) = Locations::installed(DEFAULT_PROFILE) else {
            return;
        };

        assert!(locations.profile.ends_with(DEFAULT_PROFILE));
        assert!(
            locations
                .logs
                .ends_with(Path::new(DEFAULT_PROFILE).join("logs"))
        );
        assert!(
            locations
                .window
                .ends_with(Path::new(DEFAULT_PROFILE).join(WINDOW_FILE))
        );
    }
}
