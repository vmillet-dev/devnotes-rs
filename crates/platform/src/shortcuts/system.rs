//! The keys themselves, which only a desktop can answer for: the OS checklist's (plan § 8.5).

use std::sync::Arc;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use super::{Active, Global, NoShortcuts, Shortcuts, action_of, take_each};

struct System {
    manager: GlobalHotKeyManager,
    active: Active,
}

impl Shortcuts for System {
    fn set(&self, wanted: &[(Global, String)]) -> Vec<String> {
        self.release();
        let (taken, unavailable) = take_each(wanted, |key| self.manager.register(key));
        if let Ok(mut active) = self.active.lock() {
            *active = taken;
        }
        unavailable
    }

    fn release(&self) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        let keys: Vec<HotKey> = active.drain(..).map(|(key, _)| key).collect();
        if let Err(error) = self.manager.unregister_all(&keys) {
            log::warn!("Global shortcuts not released: {error}");
        }
    }
}

/// ⚠️ Called on the window's thread: on Windows its message loop serves the keys.
pub fn system(on_pressed: impl Fn(Global) + Send + Sync + 'static) -> Box<dyn Shortcuts> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            log::warn!("No global shortcuts: {error}");
            return Box::new(NoShortcuts);
        }
    };
    let active = Active::default();
    let heard = Arc::clone(&active);
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        // Without this filter the release would replay the action.
        if event.state() == HotKeyState::Pressed
            && let Some(action) = action_of(&heard, event.id())
        {
            on_pressed(action);
        }
    }));
    Box::new(System { manager, active })
}
