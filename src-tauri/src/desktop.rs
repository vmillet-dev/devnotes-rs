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
    /// One event carrying a closed value, not one topic per action: a topic string
    /// mirrored on both sides makes a typo into a silently inert subscription. It
    /// crosses as a generated union, so a variant added here stops the front compiling.
    pub enum GlobalAction {
        #[default]
        Capture = "capture",
        NewNote = "new-note",
        Palette = "palette",
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

/// Shows the window then asks the front end for the action: the native side never
/// creates the note itself, which spares it duplicating language detection.
fn reveal_and_emit(app: &AppHandle, action: GlobalAction) {
    reveal(app);
    let _ = app.emit(ACTION_EVENT, action);
}

/// Three fields rather than a map, which would leave the compiler silent about a
/// missing shortcut.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBindings {
    pub capture: String,
    pub new_note: String,
    pub palette: String,
}

impl ShortcutBindings {
    /// ⚠️ Taken before the front end has started: without them `Ctrl+Alt+P` is dead for
    /// the length of the first render — exactly the second it is used from another
    /// application. That is why the native side has defaults of its own at all.
    ///
    /// The front end keeps no copy: these cross as the `DEFAULT_SHORTCUTS` constant.
    ///
    /// ⚠️ Not `Ctrl+Alt+Space` for the palette: widely installed applications hold it
    /// already, and a global shortcut is first come, first served.
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

/// A list rather than three constants: the combinations change with the preferences,
/// so the handler cannot compare them against captured values.
type ActiveShortcuts = Mutex<Vec<(Shortcut, GlobalAction)>>;

/// ⚠️ Every piece of state the native side owns is managed here, before any command can
/// run: a command creating its own would race another doing the same. A shortcut already
/// taken by another application is logged but not fatal.
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

/// Takes the three from scratch and returns what could not be taken. ⚠️ Everything is
/// released first, or the combination a setting replaced would keep firing.
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

/// Pushed from the preferences panel like the tray labels: reading `preferences.json`
/// back from Rust would be a second source to keep in step.
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

/// ⚠️ Both require a tray: without one, hiding the window leaves a process nothing can
/// call back.
pub(crate) fn hides_on_close(app: &AppHandle) -> bool {
    window_behavior(app).close_to_tray && tray_exists(app)
}

/// ⚠️ Tauri emits no "minimized" event: only `Resized` comes through, and it is up to
/// the caller to ask the window where it stands.
pub(crate) fn hides_on_minimize(app: &AppHandle) -> bool {
    window_behavior(app).minimize_to_tray && tray_exists(app)
}

const TRAY_ID: &str = "devnotes";

const OPEN_ITEM: &str = "open";
const NEW_NOTE_ITEM: &str = "new-note";
const CAPTURE_ITEM: &str = "capture";
const PALETTE_ITEM: &str = "palette";
const QUIT_ITEM: &str = "quit";

/// Labels cross the bridge already translated: the interface language is a front-end
/// preference, and a translation table in Rust would be a second one.
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    pub open: String,
    pub new_note: String,
    pub capture: String,
    pub palette: String,
    pub quit: String,
}

/// Replaces only the menu when the tray already exists, so a language change does not
/// make it flicker. No `Result`: an absent tray is not a failure the front can handle,
/// and [`tray_exists`] is what then keeps closing from hiding the window.
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
            QUIT_ITEM => app.exit(0),
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
