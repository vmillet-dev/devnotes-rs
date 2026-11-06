//! Whether the system starts the application with the session. The real state is the system's: a
//! person can turn it off from the task manager, and the preference then follows.

mod system;

pub use system::system;

pub trait Autostart {
    /// ⚠️ Each reads or writes the system: never on the window's thread.
    fn is_enabled(&self) -> Result<bool, String>;
    fn set(&self, enabled: bool) -> Result<(), String>;
}

/// Where nothing can be set, and the tests and scenarios, which leave the session alone.
pub struct NoAutostart;

impl Autostart for NoAutostart {
    fn is_enabled(&self) -> Result<bool, String> {
        Err("no autostart here".to_owned())
    }

    fn set(&self, _enabled: bool) -> Result<(), String> {
        Ok(())
    }
}
