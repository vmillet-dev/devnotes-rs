//! The entry itself, which only a session can answer for: the OS checklist's (plan § 8.5).

use std::sync::Arc;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use super::{Autostart, NoAutostart};

const ENTRY: &str = "devnotes";

struct System(AutoLaunch);

impl Autostart for System {
    fn is_enabled(&self) -> Result<bool, String> {
        self.0.is_enabled().map_err(|error| error.to_string())
    }

    fn set(&self, enabled: bool) -> Result<(), String> {
        let done = if enabled {
            self.0.enable()
        } else {
            self.0.disable()
        };
        done.map_err(|error| error.to_string())
    }
}

/// An `AppImage` runs from a mount that goes with the process: what starts it again is its file.
fn launched_file() -> Option<String> {
    std::env::var("APPIMAGE").ok().or_else(|| {
        let exe = std::env::current_exe().ok()?;
        exe.to_str().map(str::to_owned)
    })
}

pub fn system() -> Arc<dyn Autostart + Send + Sync> {
    let built = launched_file().map(|file| {
        AutoLaunchBuilder::new()
            .set_app_name(ENTRY)
            .set_app_path(&file)
            .build()
    });
    match built {
        Some(Ok(entry)) => Arc::new(System(entry)),
        Some(Err(error)) => {
            log::warn!("No autostart: {error}");
            Arc::new(NoAutostart)
        }
        None => Arc::new(NoAutostart),
    }
}
