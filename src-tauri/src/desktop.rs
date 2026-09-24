//! Its three commands are synchronous on purpose: the tray and the global shortcuts want
//! the main thread.

use std::str::FromStr;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

closed_enum! {
    /// One event carrying a closed value rather than a topic per action: a mistyped topic is
    /// a silently inert subscription, and a new variant here stops the front compiling.
    pub enum GlobalAction {
        #[default]
        Capture = "capture",
        NewNote = "new-note",
        Palette = "palette",
        Quit = "quit",
    }
}

/// Exported as a constant too, so neither side spells it twice.
pub(crate) const ACTION_EVENT: &str = "devnotes:action";

/// `unminimize` first: a minimized window merely shown stays in the taskbar.
pub(crate) fn reveal(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Shows the window then asks the front end for the action: the note itself is the front's to
/// create, language detection included.
fn reveal_and_emit(app: &AppHandle, action: GlobalAction) {
    reveal(app);
    let _ = app.emit(ACTION_EVENT, action);
}

/// Long enough for a write still behind a debounce, short enough to read as the quit it is.
const QUIT_GRACE: std::time::Duration = std::time::Duration::from_millis(1500);

/// The front end quits, so a write still behind a debounce reaches the disk first. ⚠️ And the
/// process ends anyway after a moment: a front end not listening — the gate is up, or it hung —
/// must not leave a quit that does nothing.
fn quit_through_the_front(app: &AppHandle) {
    let _ = app.emit(ACTION_EVENT, GlobalAction::Quit);

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(QUIT_GRACE);
        app.exit(0);
    });
}

/// Three fields rather than a map, so a missing shortcut is a compile error.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBindings {
    pub capture: String,
    pub new_note: String,
    pub palette: String,
}

impl ShortcutBindings {
    /// ⚠️ Registered before the front end starts, or `Ctrl+Alt+P` is dead during the first
    /// render — exactly when it is used from another application. The front reads them as
    /// `DEFAULT_SHORTCUTS`. Not `Ctrl+Alt+Space`, which widely installed applications hold.
    pub(crate) fn defaults() -> Self {
        Self {
            capture: "Ctrl+Alt+V".to_string(),
            new_note: "Ctrl+Alt+N".to_string(),
            palette: "Ctrl+Alt+P".to_string(),
        }
    }

    fn entries(&self) -> [(&str, GlobalAction); 3] {
        [
            (&self.capture, GlobalAction::Capture),
            (&self.new_note, GlobalAction::NewNote),
            (&self.palette, GlobalAction::Palette),
        ]
    }
}

/// A list rather than three constants: the combinations change with the preferences.
type ActiveShortcuts = Mutex<Vec<(Shortcut, GlobalAction)>>;

/// Every piece of native state is managed here, before any command can run: a command creating
/// its own would race another. A shortcut another application holds is logged, not fatal.
pub(crate) fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(ActiveShortcuts::default());
    app.manage(WindowBehaviorState::default());

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                // Without this filter the key release would replay the action.
                if event.state() != ShortcutState::Pressed {
                    return;
                }

                let Some(action) = action_of(app, shortcut) else {
                    return;
                };

                reveal_and_emit(app, action);
            })
            .build(),
    )?;

    let taken = apply_shortcuts(app, &ShortcutBindings::defaults());
    if !taken.is_empty() {
        log::warn!(
            "Global shortcuts unavailable at startup: {}",
            taken.join(", ")
        );
    }

    Ok(())
}

fn action_of(app: &AppHandle, shortcut: &Shortcut) -> Option<GlobalAction> {
    let active = app.try_state::<ActiveShortcuts>()?;
    let active = active.lock().ok()?;

    active
        .iter()
        .find(|(registered, _)| registered == shortcut)
        .map(|(_, action)| *action)
}

/// Takes the three from scratch and returns what could not be taken. Everything is released
/// first, or a replaced combination would keep firing.
fn apply_shortcuts(app: &AppHandle, bindings: &ShortcutBindings) -> Vec<String> {
    if let Err(error) = app.global_shortcut().unregister_all() {
        log::warn!("Global shortcuts not released: {error}");
    }

    let mut registered = Vec::new();
    let mut unavailable = Vec::new();

    for (accelerator, topic) in bindings.entries() {
        let Ok(shortcut) = Shortcut::from_str(accelerator) else {
            log::warn!("Global shortcut {accelerator} unreadable");
            unavailable.push(accelerator.to_string());
            continue;
        };

        if let Err(error) = app.global_shortcut().register(shortcut) {
            log::warn!("Global shortcut {accelerator} unavailable: {error}");
            unavailable.push(accelerator.to_string());
            continue;
        }

        registered.push((shortcut, topic));
    }

    if let Some(active) = app.try_state::<ActiveShortcuts>()
        && let Ok(mut active) = active.lock()
    {
        *active = registered;
    }

    unavailable
}

/// The native side can only fail silently, and a log line is not an interface.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn set_global_shortcuts(bindings: ShortcutBindings, app: AppHandle) -> Vec<String> {
    apply_shortcuts(&app, &bindings)
}

/// Pushed from the preferences panel as it changes, like the tray labels.
#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowBehavior {
    pub close_to_tray: bool,
    pub minimize_to_tray: bool,
}

impl Default for WindowBehavior {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            minimize_to_tray: false,
        }
    }
}

type WindowBehaviorState = Mutex<WindowBehavior>;

#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn set_window_behavior(behavior: WindowBehavior, state: State<'_, WindowBehaviorState>) {
    if let Ok(mut current) = state.lock() {
        *current = behavior;
    }
}

fn window_behavior(app: &AppHandle) -> WindowBehavior {
    app.try_state::<WindowBehaviorState>()
        .and_then(|state: State<'_, WindowBehaviorState>| state.lock().ok().map(|current| *current))
        .unwrap_or_default()
}

/// Both require a tray: without one, a hidden window leaves a process nothing can call back.
pub(crate) fn hides_on_close(app: &AppHandle) -> bool {
    window_behavior(app).close_to_tray && tray_exists(app)
}

/// Tauri emits no "minimized" event: the caller asks the window after a `Resized`.
pub(crate) fn hides_on_minimize(app: &AppHandle) -> bool {
    window_behavior(app).minimize_to_tray && tray_exists(app)
}

const TRAY_ID: &str = "devnotes";

const OPEN_ITEM: &str = "open";
const NEW_NOTE_ITEM: &str = "new-note";
const CAPTURE_ITEM: &str = "capture";
const PALETTE_ITEM: &str = "palette";
const QUIT_ITEM: &str = "quit";

/// Labels cross already translated: the interface language is a front-end preference.
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    pub open: String,
    pub new_note: String,
    pub capture: String,
    pub palette: String,
    pub quit: String,
}

/// Replaces only the menu when the tray exists, so a language change does not flicker. No
/// `Result`: without a tray, `tray_exists` is what stops closing from hiding the window.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn sync_tray(labels: TrayLabels, app: AppHandle) {
    let menu = match build_menu(&app, &labels) {
        Ok(menu) => menu,
        Err(error) => {
            log::warn!("System tray menu unavailable: {error}");
            return;
        }
    };

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Err(error) = tray.set_menu(Some(menu)) {
            log::warn!("System tray menu not updated: {error}");
        }
        return;
    }

    if let Err(error) = build_tray(&app, &menu) {
        log::warn!("System tray unavailable: {error}");
    }
}

pub(crate) fn tray_exists(app: &AppHandle) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

fn build_menu(app: &AppHandle, labels: &TrayLabels) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, OPEN_ITEM, &labels.open, true, None::<&str>)?;
    let new_note = MenuItem::with_id(app, NEW_NOTE_ITEM, &labels.new_note, true, None::<&str>)?;
    let capture = MenuItem::with_id(app, CAPTURE_ITEM, &labels.capture, true, None::<&str>)?;
    let palette = MenuItem::with_id(app, PALETTE_ITEM, &labels.palette, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT_ITEM, &labels.quit, true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&open, &new_note, &capture, &palette, &separator, &quit],
    )
}

fn build_tray(app: &AppHandle, menu: &Menu<Wry>) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or(tauri::Error::UnknownPath)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("DevNotes")
        // The menu stays on right click, where Windows expects it.
        .show_menu_on_left_click(false)
        .menu(menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN_ITEM => reveal(app),
            NEW_NOTE_ITEM => reveal_and_emit(app, GlobalAction::NewNote),
            CAPTURE_ITEM => reveal_and_emit(app, GlobalAction::Capture),
            PALETTE_ITEM => reveal_and_emit(app, GlobalAction::Palette),
            // The only path that terminates the process: the close button only hides.
            QUIT_ITEM => quit_through_the_front(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
