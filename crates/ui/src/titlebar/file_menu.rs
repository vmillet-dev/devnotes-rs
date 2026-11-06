//! The File menu: what leaves or enters the library, and quitting. It is there only while a
//! library is open: every entry acts on it.

use egui::{Popup, PopupCloseBehavior, Ui};
use engine::app_info::METADATA;
use i18n::{Message, keys};

use super::{Action, Tone};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::widgets;

const WIDTH: f32 = 210.0;

#[derive(Default)]
pub(super) struct View {
    /// Quitting closes the application for good: a second click confirms it.
    confirming_quit: bool,
}

/// The menu's control, for what it opens to give the focus back to.
pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    view: &mut View,
    actions: &mut Actions,
) -> egui::Id {
    let label = i18n::translate(state.language, keys::file::MENU_LABEL, &[]);
    let anchor = super::menu_trigger(ui, &label, "file-menu");
    let shown = Popup::menu(&anchor)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| entries(ui, state, view, actions));
    if shown.is_none() {
        view.confirming_quit = false;
    }
    anchor.id
}

fn entries(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 2.0;
    let tr = |key| i18n::translate(state.language, key, &[]);
    let on_screen = state.on_screen();
    let on_screen: Vec<&str> = on_screen.iter().map(String::as_str).collect();
    let checked = !state.selection.checked_in(&on_screen).is_empty();
    let space = state.spaces.active_space().is_some();
    let rows = [
        (keys::file::IMPORT, "import", true, Action::Import),
        (keys::file::EXPORT_ALL, "exportAll", true, Action::ExportAll),
        (
            keys::file::EXPORT_SPACE,
            "exportSpace",
            space,
            Action::ExportSpace,
        ),
        (
            keys::file::EXPORT_SELECTION,
            "exportSelection",
            checked,
            Action::ExportSelection,
        ),
        (
            keys::file::COPY_MARKDOWN,
            "copyMarkdown",
            checked,
            Action::CopyMarkdown,
        ),
    ];
    let mut ids = Vec::with_capacity(rows.len() + 3);
    for (key, entry, enabled, action) in rows {
        let id = format!("file-option-{entry}");
        let tone = if enabled { Tone::Plain } else { Tone::Idle };
        let row = super::menu_option(ui, &tr(key), &id, tone);
        ids.push(row.id);
        if enabled && row.clicked() {
            actions.push(Asked::Menu(action));
            ui.close();
        }
    }
    super::separator(ui);
    let row = super::menu_option(
        ui,
        &tr(keys::libraries::MENU_ENTRY),
        "file-libraries",
        Tone::Plain,
    );
    ids.push(row.id);
    if row.clicked() {
        actions.push(Asked::Menu(Action::OpenLibraries));
        ui.close();
    }
    let row = super::menu_option(
        ui,
        &tr(keys::file::PREFERENCES),
        "file-preferences",
        Tone::Plain,
    );
    ids.push(row.id);
    if row.clicked() {
        actions.push(Asked::Menu(Action::OpenPreferences));
        ui.close();
    }
    let quit = if view.confirming_quit {
        tr(keys::file::QUIT_CONFIRM)
    } else {
        Message::new(keys::file::QUIT)
            .with("app", METADATA.name)
            .text(state.language)
    };
    let row = super::menu_option(ui, &quit, "file-quit", Tone::Danger);
    ids.push(row.id);
    if row.clicked() {
        if view.confirming_quit {
            actions.push(Asked::Menu(Action::Quit));
            ui.close();
        }
        view.confirming_quit = true;
    }
    widgets::menu_arrows(ui, &ids);
}
