//! The « start with the system » switch. At the launch the system is read and the preference
//! follows it; after that the preference is pushed, and only when it differs: writing the entry
//! on every start would bring back one a person removed. Nothing here stops a launch, and a
//! failure stays in the log with the switch as the person set it.

use std::sync::Arc;

use platform::autostart::{Autostart, NoAutostart};

use super::settings::Settings;
use crate::backend::{Backend, Pending};

pub struct AutostartState {
    system: Arc<dyn Autostart + Send + Sync>,
    reading: Option<Pending<Result<bool, String>>>,
    /// What the system was last told, or read to hold.
    known: Option<bool>,
}

impl Default for AutostartState {
    fn default() -> Self {
        Self::with(Arc::new(NoAutostart))
    }
}

impl AutostartState {
    pub fn with(system: Arc<dyn Autostart + Send + Sync>) -> Self {
        Self {
            system,
            reading: None,
            known: None,
        }
    }

    pub fn read(&mut self, backend: &Backend) {
        let system = Arc::clone(&self.system);
        self.reading = Some(backend.compute(move |_| Ok(system.is_enabled())));
    }

    /// `true` when the settings changed and are to be saved.
    pub fn update(&mut self, backend: &Backend, settings: &mut Settings) -> bool {
        let followed = self.adopt_reading(settings);
        if self.reading.is_none() && self.known != Some(settings.start_with_system) {
            self.push(backend, settings.start_with_system);
        }
        followed
    }

    fn adopt_reading(&mut self, settings: &mut Settings) -> bool {
        let Some(answer) = self.reading.as_mut().and_then(Pending::poll) else {
            return false;
        };
        self.reading = None;
        let enabled = match answer {
            Ok(Ok(enabled)) => enabled,
            Ok(Err(detail)) => {
                log::warn!("Autostart could not be read: {detail}");
                settings.start_with_system
            }
            Err(error) => {
                log::warn!("Autostart could not be read: {}", error.detail);
                settings.start_with_system
            }
        };
        self.known = Some(enabled);
        let followed = settings.start_with_system != enabled;
        settings.start_with_system = enabled;
        followed
    }

    fn push(&mut self, backend: &Backend, enabled: bool) {
        self.known = Some(enabled);
        let system = Arc::clone(&self.system);
        drop(backend.compute(move |_| {
            if system.is_enabled() != Ok(enabled)
                && let Err(detail) = system.set(enabled)
            {
                log::warn!("Autostart could not be set: {detail}");
            }
            Ok(())
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::testing;

    #[derive(Default)]
    struct Fake {
        enabled: Mutex<bool>,
        writes: Mutex<Vec<bool>>,
        unreadable: Mutex<bool>,
    }

    impl Autostart for Fake {
        fn is_enabled(&self) -> Result<bool, String> {
            if *self.unreadable.lock().unwrap() {
                return Err("denied".to_owned());
            }
            Ok(*self.enabled.lock().unwrap())
        }

        fn set(&self, enabled: bool) -> Result<(), String> {
            *self.enabled.lock().unwrap() = enabled;
            self.writes.lock().unwrap().push(enabled);
            Ok(())
        }
    }

    fn launched(fake: &Arc<Fake>, settings: &mut Settings) -> (AutostartState, Backend, bool) {
        let (backend, profile) = testing::backend();
        std::mem::forget(profile);
        let mut state = AutostartState::with(Arc::clone(fake) as Arc<dyn Autostart + Send + Sync>);
        state.read(&backend);
        backend.settle();
        let followed = state.update(&backend, settings);
        backend.settle();
        (state, backend, followed)
    }

    #[test]
    fn the_preference_follows_what_the_system_holds() {
        let fake = Arc::new(Fake::default());
        let mut settings = Settings {
            start_with_system: true,
            ..Settings::default()
        };

        let (_, _, followed) = launched(&fake, &mut settings);

        assert!(followed, "to be saved");
        assert!(!settings.start_with_system);
        assert!(fake.writes.lock().unwrap().is_empty());
    }

    #[test]
    fn nothing_is_written_at_a_launch_that_agrees() {
        let fake = Arc::new(Fake::default());
        let mut settings = Settings::default();

        let (_, _, followed) = launched(&fake, &mut settings);

        assert!(!followed);
        assert!(fake.writes.lock().unwrap().is_empty());
    }

    #[test]
    fn a_change_of_the_switch_reaches_the_system_once() {
        let fake = Arc::new(Fake::default());
        let mut settings = Settings::default();
        let (mut state, backend, _) = launched(&fake, &mut settings);

        settings.start_with_system = true;
        state.update(&backend, &mut settings);
        backend.settle();
        state.update(&backend, &mut settings);
        backend.settle();

        assert_eq!(*fake.writes.lock().unwrap(), [true]);
    }

    #[test]
    fn a_system_that_cannot_be_read_leaves_the_switch_alone() {
        let fake = Arc::new(Fake::default());
        *fake.unreadable.lock().unwrap() = true;
        let mut settings = Settings {
            start_with_system: true,
            ..Settings::default()
        };

        let (_, _, followed) = launched(&fake, &mut settings);

        assert!(!followed);
        assert!(settings.start_with_system);
    }
}
