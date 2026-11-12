//! The offer of a new version. The notes are open and in a block of fixed height: the geometry
//! does not depend on a two-line release against a forty-line one, and « Install now » stays
//! under the pointer. Once installing starts it cannot be dismissed: files are being replaced.

use egui::{Align, Id, Layout, RichText, Ui, Vec2};
use engine::app_info::METADATA;
use i18n::{Key, Message, keys};
use platform::updater::AvailableUpdate;

use crate::action::{Action as Asked, Actions};
use crate::services::updates::Status;
use crate::state::AppState;
use crate::theme::palette;
use crate::titlebar::Action;
use crate::widgets::{self, Button, Checkbox, Kind};

const WIDTH: f32 = 480.0;
const NOTES: f32 = 160.0;

/// What the dialog holds until it is answered.
#[derive(Default)]
pub(crate) struct UpdateView {
    /// Read on the way out: Escape and the backdrop honour a box already ticked.
    skip: bool,
}

/// What the prompt asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Check,
    Install,
    Later { skip: bool },
}

pub(crate) fn id() -> Id {
    Id::unique("update-prompt")
}

pub(crate) fn show(
    ctx: &egui::Context,
    state: &AppState,
    view: &mut UpdateView,
    actions: &mut Actions,
) {
    let Some(update) = state.updates.offered() else {
        return;
    };
    let shown = widgets::dialog(ctx, &state.dialogs, id(), WIDTH, |ui| {
        body(ui, state, update, view, actions);
    });
    let busy = state.updates.is_busy();
    if shown.is_some_and(|shown| shown.should_close()) && !busy {
        let skip = view.skip;
        actions.push(Asked::Menu(Action::Update(Ask::Later { skip })));
    }
}

fn body(
    ui: &mut Ui,
    state: &AppState,
    update: &AvailableUpdate,
    view: &mut UpdateView,
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = ui.label(
        RichText::new(tr(keys::update::TITLE))
            .size(15.0)
            .variation(*b"wght", 600.0)
            .color(p.text_0),
    );
    widgets::address(ui, &title, "update-prompt");
    let said = Message::new(keys::update::SUMMARY)
        .with("app", METADATA.name)
        .with("version", update.version.as_str())
        .with("current", update.current_version.as_str())
        .text(language);
    let summary = RichText::new(said).size(12.5).color(p.text_1);
    ui.add(egui::Label::new(summary).wrap());
    if let Some(notes) = state.updates.notes() {
        let label = tr(keys::update::NOTES_LABEL).to_uppercase();
        ui.label(RichText::new(label).size(11.0).color(p.text_2));
        let block = egui::ScrollArea::vertical()
            .id_salt("update-notes")
            .max_height(NOTES)
            .min_scrolled_height(NOTES)
            .auto_shrink([false, false])
            .show(ui, |ui| widgets::release_notes(ui, notes));
        let region = widgets::region(ui, block.inner_rect, "update-notes", egui::Sense::hover());
        widgets::address(ui, &region, "update-notes");
    }
    if state.updates.is_busy() {
        progress(ui, state);
        return;
    }
    let skip = Message::new(keys::update::SKIP_VERSION)
        .with("version", update.version.as_str())
        .text(language);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        Checkbox::new(&mut view.skip, &skip, "update-skip").show(ui);
        let words = RichText::new(&skip).size(12.0).color(p.text_1);
        let label = ui.add(egui::Label::new(words).sense(egui::Sense::click()));
        widgets::address(ui, &label, "update-skip-label");
        if label.clicked() {
            view.skip = !view.skip;
        }
    });
    let skip = view.skip;
    let mut ask = |ask| actions.push(Asked::Menu(Action::Update(ask)));
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::update::INSTALL), "update-install")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
        {
            ask(Ask::Install);
        }
        if Button::new(&tr(keys::update::LATER), "update-later")
            .show(ui)
            .clicked()
        {
            ask(Ask::Later { skip });
        }
    });
}

fn progress(ui: &mut Ui, state: &AppState) {
    let language = state.language;
    let p = palette(ui.ctx());
    let percent = state.updates.progress();
    let said = match (state.updates.status(), percent) {
        (Status::Installed, _) => Message::new(keys::update::RESTARTING)
            .with("app", METADATA.name)
            .text(language),
        (_, Some(percent)) => Message::new(keys::update::DOWNLOADING_PERCENT)
            .with("percent", percent)
            .text(language),
        _ => i18n::translate(language, keys::update::DOWNLOADING, &[]),
    };
    let status = ui.label(RichText::new(said).size(12.0).color(p.text_1));
    widgets::address(ui, &status, "update-status");
    #[expect(clippy::cast_precision_loss, reason = "a percent")]
    let done = percent.map_or(0.0, |percent| percent as f32 / 100.0);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
    let corner = egui::CornerRadius::same(3);
    ui.painter().rect_filled(rect, corner, p.bg_3);
    let filled = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * done, rect.height()));
    ui.painter().rect_filled(filled, corner, p.amber_fill);
}
