//! The three keys taken from the whole machine, as the saved settings have them, never the draft
//! of the preferences. Taking them at the launch and after a change says what another application
//! holds; taking them back after a field listened says nothing, it was said when they were set.

use i18n::{Message, keys};
use platform::shortcuts::{Global, NoShortcuts, Shortcuts};

use super::errors::ErrorNotifier;
use super::shortcuts::{self, Bindings, Scope};

pub struct GlobalKeys {
    system: Box<dyn Shortcuts>,
    taken: Vec<(Global, String)>,
    released: bool,
    held: Vec<String>,
}

impl Default for GlobalKeys {
    fn default() -> Self {
        Self::with(Box::new(NoShortcuts))
    }
}

impl GlobalKeys {
    pub fn with(system: Box<dyn Shortcuts>) -> Self {
        Self {
            system,
            taken: Vec::new(),
            released: false,
            held: Vec::new(),
        }
    }

    /// What the last taking could not have: another application holds it.
    pub fn held(&self) -> &[String] {
        &self.held
    }

    /// `listening`: a field of the preferences waits for a keystroke, and Windows would hand it
    /// our own keys rather than the field.
    pub fn update(&mut self, bindings: &Bindings, listening: bool, errors: &mut ErrorNotifier) {
        if listening {
            if !self.released {
                self.released = true;
                self.system.release();
            }
            return;
        }
        let wanted = wanted(bindings);
        let comes_back = std::mem::take(&mut self.released);
        if !comes_back && wanted == self.taken {
            return;
        }
        self.held = self.system.set(&wanted);
        self.taken = wanted;
        if !comes_back && !self.held.is_empty() {
            errors.notify(
                Message::new(keys::shortcuts::UNAVAILABLE)
                    .with("count", u32::try_from(self.held.len()).unwrap_or(u32::MAX))
                    .with("list", self.held.join(", ")),
            );
        }
    }

    /// At the exit.
    pub fn release(&self) {
        self.system.release();
    }
}

fn wanted(bindings: &Bindings) -> Vec<(Global, String)> {
    shortcuts::ACTIONS
        .iter()
        .filter(|action| action.scope == Scope::Global)
        .filter_map(|action| Some((global(action.id)?, bindings.of(*action).to_owned())))
        .collect()
}

fn global(id: &str) -> Option<Global> {
    match id {
        shortcuts::PALETTE => Some(Global::Palette),
        shortcuts::CAPTURE => Some(Global::Capture),
        shortcuts::NEW_NOTE => Some(Global::NewNote),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    type Wanted = Vec<(Global, String)>;

    /// Refuses what a test said another application holds, and remembers what it was asked.
    #[derive(Clone, Default)]
    struct Fake {
        held_elsewhere: Arc<Mutex<Vec<String>>>,
        sets: Arc<Mutex<Vec<Wanted>>>,
        releases: Arc<Mutex<usize>>,
    }

    impl Shortcuts for Fake {
        fn set(&self, wanted: &[(Global, String)]) -> Vec<String> {
            self.sets.lock().unwrap().push(wanted.to_vec());
            self.held_elsewhere.lock().unwrap().clone()
        }

        fn release(&self) {
            *self.releases.lock().unwrap() += 1;
        }
    }

    fn keys_on(fake: &Fake) -> GlobalKeys {
        GlobalKeys::with(Box::new(fake.clone()))
    }

    #[test]
    fn the_three_keys_are_taken_once_as_the_settings_have_them() {
        let fake = Fake::default();
        let mut keys = keys_on(&fake);
        let mut errors = ErrorNotifier::default();
        let bindings = Bindings::default();

        keys.update(&bindings, false, &mut errors);
        keys.update(&bindings, false, &mut errors);

        let sets = fake.sets.lock().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0][0], (Global::Palette, "Ctrl+Alt+P".to_owned()));
        assert_eq!(sets[0].len(), 3);
    }

    #[test]
    fn a_key_moved_is_taken_again() {
        let fake = Fake::default();
        let mut keys = keys_on(&fake);
        let mut errors = ErrorNotifier::default();
        let mut bindings = Bindings::default();
        keys.update(&bindings, false, &mut errors);

        bindings.set(shortcuts::ACTIONS[0], "Ctrl+Alt+K".to_owned());
        keys.update(&bindings, false, &mut errors);

        let sets = fake.sets.lock().unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[1][0].1, "Ctrl+Alt+K");
    }

    #[test]
    fn what_another_application_holds_is_said_and_remembered() {
        let fake = Fake::default();
        *fake.held_elsewhere.lock().unwrap() = vec!["Ctrl+Alt+P".to_owned()];
        let mut keys = keys_on(&fake);
        let mut errors = ErrorNotifier::default();

        keys.update(&Bindings::default(), false, &mut errors);

        assert_eq!(keys.held(), ["Ctrl+Alt+P"]);
        let said = errors.notice().unwrap().text(i18n::Language::En);
        assert!(said.contains("Ctrl+Alt+P"), "{said}");
    }

    #[test]
    fn a_field_listening_lets_go_once_and_they_come_back_quietly() {
        let fake = Fake::default();
        *fake.held_elsewhere.lock().unwrap() = vec!["Ctrl+Alt+P".to_owned()];
        let mut keys = keys_on(&fake);
        let mut errors = ErrorNotifier::default();
        let bindings = Bindings::default();
        keys.update(&bindings, false, &mut errors);
        errors.dismiss();

        keys.update(&bindings, true, &mut errors);
        keys.update(&bindings, true, &mut errors);
        assert_eq!(*fake.releases.lock().unwrap(), 1);

        keys.update(&bindings, false, &mut errors);
        assert_eq!(fake.sets.lock().unwrap().len(), 2);
        assert!(errors.notice().is_none(), "it was said when they were set");
    }
}
