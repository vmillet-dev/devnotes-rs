//! Keys taken from the whole machine: first come, first served, and the loser gets no error from
//! the system, so what could not be taken is answered and said. They do not work under Wayland,
//! where nothing lets an application listen outside its windows.

mod system;

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use global_hotkey::hotkey::HotKey;

pub use system::system;

/// What a key can be bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Global {
    Palette,
    Capture,
    NewNote,
}

pub trait Shortcuts {
    /// Takes `wanted` from scratch, or a replaced key would keep firing. Answers the keys it
    /// could not take, as they were written.
    fn set(&self, wanted: &[(Global, String)]) -> Vec<String>;
    /// Lets go of all of them: a key of ours would never reach a field listening for one.
    fn release(&self);
}

/// Where the system takes no key, and the tests that have no use for one.
pub struct NoShortcuts;

impl Shortcuts for NoShortcuts {
    fn set(&self, _wanted: &[(Global, String)]) -> Vec<String> {
        Vec::new()
    }

    fn release(&self) {}
}

/// A list and not three constants: the keys change with the preferences.
type Active = Arc<Mutex<Vec<(HotKey, Global)>>>;

/// What was taken, and what was not: unreadable, or held by another application.
fn take_each<E: std::fmt::Display>(
    wanted: &[(Global, String)],
    mut register: impl FnMut(HotKey) -> Result<(), E>,
) -> (Vec<(HotKey, Global)>, Vec<String>) {
    let mut taken = Vec::new();
    let mut unavailable = Vec::new();
    for (action, accelerator) in wanted {
        let Ok(key) = HotKey::from_str(accelerator) else {
            log::warn!("Global shortcut {accelerator} unreadable");
            unavailable.push(accelerator.clone());
            continue;
        };
        match register(key) {
            Ok(()) => taken.push((key, *action)),
            Err(error) => {
                log::warn!("Global shortcut {accelerator} unavailable: {error}");
                unavailable.push(accelerator.clone());
            }
        }
    }
    (taken, unavailable)
}

fn action_of(active: &Active, id: u32) -> Option<Global> {
    let active = active.lock().ok()?;
    active
        .iter()
        .find(|(key, _)| key.id() == id)
        .map(|(_, action)| *action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted() -> Vec<(Global, String)> {
        vec![
            (Global::Palette, "Ctrl+Alt+P".to_owned()),
            (Global::Capture, "Ctrl+Alt+V".to_owned()),
        ]
    }

    #[test]
    fn what_is_free_is_taken_and_found_again_by_its_id() {
        let (taken, unavailable) = take_each(&wanted(), |_| Ok::<(), String>(()));
        let id = taken[1].0.id();

        assert!(unavailable.is_empty());
        assert_eq!(
            action_of(&Arc::new(Mutex::new(taken)), id),
            Some(Global::Capture)
        );
    }

    #[test]
    fn a_key_another_application_holds_is_answered_and_the_others_still_taken() {
        let held = HotKey::from_str("Ctrl+Alt+P").unwrap();

        let (taken, unavailable) = take_each(&wanted(), |key| {
            if key == held {
                Err("already registered")
            } else {
                Ok(())
            }
        });

        assert_eq!(unavailable, ["Ctrl+Alt+P"]);
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].1, Global::Capture);
    }

    #[test]
    fn a_key_that_does_not_read_is_answered_as_written() {
        let wanted = [(Global::NewNote, "Ctrl+Nothing".to_owned())];

        let (taken, unavailable) = take_each(&wanted, |_| Ok::<(), String>(()));

        assert!(taken.is_empty());
        assert_eq!(unavailable, ["Ctrl+Nothing"]);
    }

    #[test]
    fn the_keys_shipped_all_read() {
        for key in ["Ctrl+Alt+P", "Ctrl+Alt+V", "Ctrl+Alt+N", "Ctrl+Shift+F12"] {
            assert!(HotKey::from_str(key).is_ok(), "{key}");
        }
    }
}
