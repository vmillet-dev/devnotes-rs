//! The dialog itself, which only a desktop can show: the OS checklist's (plan § 8.5).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::FileDialog;

/// ⚠️ It blocks until answered: never called on the window's thread.
struct System<F> {
    on_missing: F,
}

impl<F: Fn()> System<F> {
    fn shown(&self) -> bool {
        let shown = portal_answers();
        if !shown {
            (self.on_missing)();
        }
        shown
    }
}

impl<F: Fn()> FileDialog for System<F> {
    fn open(&self) -> Option<PathBuf> {
        self.shown().then(|| rfd::FileDialog::new().pick_file())?
    }

    fn open_of(&self, extensions: &[&str]) -> Option<PathBuf> {
        self.shown().then(|| {
            rfd::FileDialog::new()
                .add_filter(extensions[0], extensions)
                .pick_file()
        })?
    }

    fn save(&self, name: &str) -> Option<PathBuf> {
        if !self.shown() {
            return None;
        }
        let dialog = rfd::FileDialog::new().set_file_name(name);
        let extension = Path::new(name).extension().and_then(|it| it.to_str());
        match extension {
            Some(extension) => dialog.add_filter(extension, &[extension]).save_file(),
            None => dialog.save_file(),
        }
    }
}

pub(super) fn system(
    on_missing: impl Fn() + Send + Sync + 'static,
) -> Arc<dyn FileDialog + Send + Sync> {
    Arc::new(System { on_missing })
}

/// Under Linux the dialog is the XDG portal's (D29), and without one `rfd` answers `None` like a
/// cancel (S1): asked first, running or ready to be started by the bus.
#[cfg(target_os = "linux")]
fn portal_answers() -> bool {
    use zbus::blocking::{Connection, fdo::DBusProxy};

    const PORTAL: &str = "org.freedesktop.portal.Desktop";

    let on_the_bus = || -> zbus::Result<bool> {
        let connection = Connection::session()?;
        let bus = DBusProxy::new(&connection)?;
        let running = bus.list_names()?;
        let startable = bus.list_activatable_names()?;
        Ok(running
            .iter()
            .chain(&startable)
            .any(|name| name.as_str() == PORTAL))
    };
    on_the_bus().unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn portal_answers() -> bool {
    true
}
