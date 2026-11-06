//! Where the application reads and writes, resolved once at launch (plan § 4.1).

use std::path::{Path, PathBuf};

/// The profile of the egui application until 1.0.0: the Angular reference keeps
/// `com.devnotes.app`, and the two never share a library (D16).
pub const DEFAULT_PROFILE: &str = "com.devnotes.app.dev";

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

/// What ships beside the binary (the identity tool's lists): next to the executable on
/// Windows, under `lib/devnotes` in a Linux package, `development` when neither holds them.
pub fn resources(development: &Path) -> PathBuf {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let mut candidates: Vec<PathBuf> = beside.into_iter().collect();
    if cfg!(target_os = "linux") {
        if let Some(image) = std::env::var_os("APPDIR") {
            candidates.push(PathBuf::from(image).join("usr/lib/devnotes"));
        }
        candidates.push(PathBuf::from("/usr/lib/devnotes"));
    }
    candidates
        .into_iter()
        .find(|directory| directory.join("identity").is_dir())
        .unwrap_or_else(|| development.to_path_buf())
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn resources_fall_back_to_the_development_folder() {
        let development = tempfile::tempdir().unwrap();

        assert_eq!(resources(development.path()), development.path());
    }
}
