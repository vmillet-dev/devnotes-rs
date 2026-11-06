//! The About menu and the four panels it opens. It reads nothing of the library, so it stays
//! at the gate too.

mod card;
mod guide;
mod sheet;
mod whats_new;

use egui::{
    Align, FontFamily, Id, Layout, Popup, PopupCloseBehavior, Rect, Sense, Stroke, Ui, Vec2,
};
use engine::app_info::METADATA;
use i18n::{Message, keys};

use super::{Action, Tone};
use crate::action::{Action as Asked, Actions};
use crate::banners::update_prompt::Ask;
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets;

const WIDTH: f32 = 210.0;
const FOOTER: f32 = 56.0;

/// What the menu opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    WhatsNew,
    /// At a chapter: the canvas and the board link straight into the one that explains them.
    Guide(usize),
    Sheet,
    About,
}

#[derive(Default)]
pub(super) struct View {
    panel: Option<Panel>,
    pub(super) anchor: Option<Id>,
}

impl View {
    pub(super) fn show_panel(&mut self, panel: Panel) {
        self.panel = Some(panel);
    }

    pub(super) fn close_panel(&mut self) {
        self.panel = None;
    }
}

pub(crate) fn id() -> Id {
    Id::unique("about-panel")
}

pub(super) fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let label = i18n::translate(state.language, keys::about::MENU_LABEL, &[]);
    let anchor = super::menu_trigger(ui, &label, "about-open");
    view.anchor = Some(anchor.id);
    if state.updates.has_pending() {
        dot(ui, &anchor, state, "update-dot");
    }
    // The check keeps the menu open for its answer; an offer is answered in its own dialog.
    if state.updates.offered().is_some() {
        Popup::close_id(ui.ctx(), Popup::default_response_id(&anchor));
    }
    Popup::menu(&anchor)
        .frame(widgets::menu_frame(ui.ctx()))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| entries(ui, state, actions));
}

fn entries(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 2.0;
    let tr = |key| i18n::translate(state.language, key, &[]);
    let about = Message::new(keys::about::OPEN_ABOUT)
        .with("app", METADATA.name)
        .text(state.language);
    let mut ids = Vec::with_capacity(5);
    if state.updates.can_check() {
        ids.push(check_entry(ui, state, actions));
        super::separator(ui);
    }
    let mut entry = |ui: &mut Ui, label: &str, id: &str, panel| {
        let row = super::menu_option(ui, label, id, Tone::Plain);
        ids.push(row.id);
        if row.clicked() {
            actions.push(Asked::Menu(Action::OpenHelp(panel)));
            ui.close();
        }
    };
    entry(
        ui,
        &tr(keys::about::WHATS_NEW),
        "about-whats-new",
        Panel::WhatsNew,
    );
    entry(
        ui,
        &tr(keys::about::GETTING_STARTED),
        "about-getting-started",
        Panel::Guide(0),
    );
    entry(
        ui,
        &tr(keys::about::SHORTCUTS),
        "about-shortcuts",
        Panel::Sheet,
    );
    super::separator(ui);
    entry(ui, &about, "about-about", Panel::About);
    widgets::menu_arrows(ui, &ids);
}

/// The trigger's says something is waiting; the entry's says what for.
fn dot(ui: &mut Ui, under: &egui::Response, state: &AppState, author_id: &str) {
    let p = palette(ui.ctx());
    let centre = under.rect.right_top() + Vec2::new(-5.0, 6.0);
    ui.painter().circle_filled(centre, 3.0, p.amber_fill);
    let waiting = i18n::translate(state.language, keys::about::UPDATE_WAITING, &[]);
    let rect = Rect::from_center_size(centre, Vec2::splat(6.0));
    let marked = ui.allocate_rect(rect, Sense::hover());
    marked.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &waiting));
    widgets::address(ui, &marked, author_id);
}

/// « Check for updates », and under it what the check answered: it comes after the click, and
/// is said without moving the focus.
fn check_entry(ui: &mut Ui, state: &AppState, actions: &mut Actions) -> Id {
    use crate::services::updates::CheckState;
    let language = state.language;
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::about::CHECK_UPDATES, &[]);
    let row = super::menu_option(ui, &label, "about-check-updates", Tone::Plain);
    if state.updates.has_pending() {
        dot(ui, &row, state, "update-dot-option");
    }
    let checking = state.updates.check_state() == CheckState::Checking;
    if row.clicked() && !checking {
        actions.push(Asked::Menu(Action::Update(Ask::Check)));
    }
    let said = match state.updates.check_state() {
        CheckState::Idle => None,
        CheckState::Checking => Some(Message::new(keys::about::CHECKING)),
        CheckState::UpToDate => {
            Some(Message::new(keys::about::UP_TO_DATE).with("app", METADATA.name))
        }
        CheckState::Failed => Some(Message::new(keys::about::CHECK_FAILED)),
    };
    if let Some(said) = said {
        let words = egui::RichText::new(said.text(language))
            .size(11.0)
            .monospace()
            .color(p.text_2);
        let status = ui.label(words);
        widgets::address(ui, &status, "about-check-status");
    }
    row.id
}

pub(super) fn panels(
    ctx: &egui::Context,
    state: &AppState,
    view: &mut View,
    actions: &mut Actions,
) {
    let Some(panel) = view.panel else {
        return;
    };
    let close = match panel {
        Panel::WhatsNew => whats_new::show(ctx, state, actions),
        Panel::Guide(chapter) => guide::show(ctx, state, chapter, actions, &mut view.panel),
        Panel::Sheet => sheet::show(ctx, state, actions),
        Panel::About => card::show(ctx, state, actions),
    };
    if close {
        actions.push(Asked::Menu(Action::CloseHelp));
    }
}

/// A framed panel: its title and what it is, a body that scrolls, and a footer.
fn framed(
    ui: &mut Ui,
    (title, subtitle): (&str, &str),
    body: impl FnOnce(&mut Ui),
    footer: impl FnOnce(&mut Ui),
) {
    let p = palette(ui.ctx());
    let whole = ui.max_rect();
    let line = Stroke::new(1.0, p.line_soft);
    let bottom = whole.bottom() - FOOTER;
    let words = widgets::text(ui, title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let at = egui::pos2(whole.left() + 20.0, whole.top() + 16.0);
    let height = words.size().y;
    let heading = ui.allocate_rect(Rect::from_min_size(at, words.size()), Sense::hover());
    ui.painter().galley(at, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, title));
    let under = ui.painter().layout(
        subtitle.to_owned(),
        egui::FontId::proportional(11.5),
        p.text_2,
        whole.width() - 40.0,
    );
    let top = at.y + height + 4.0 + under.size().y + 12.0;
    ui.painter()
        .galley(at + Vec2::new(0.0, height + 4.0), under, p.text_2);
    ui.painter().hline(whole.x_range(), top, line);
    ui.painter().hline(whole.x_range(), bottom, line);

    let room = Rect::from_min_max(
        egui::pos2(whole.left(), top),
        egui::pos2(whole.right(), bottom),
    );
    let mut page = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(room)
            .layout(Layout::top_down(Align::Min)),
    );
    egui::ScrollArea::vertical()
        .id_salt("about-panel-body")
        .auto_shrink(false)
        .show(&mut page, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui);
                });
        });

    let rest = Rect::from_min_max(
        egui::pos2(whole.left() + 20.0, bottom),
        egui::pos2(whole.right() - 20.0, whole.bottom()),
    );
    let mut under = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rest)
            .layout(Layout::left_to_right(Align::Center)),
    );
    footer(&mut under);
}
