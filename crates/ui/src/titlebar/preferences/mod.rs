//! Preferences: a rail of pages, the page chosen, Cancel / Apply / OK under them. The controls
//! edit a draft; a button writes it through. Only the theme and the density show while they are
//! chosen, which is why the draft lives here and nothing else reads it.

mod general;
mod security;
mod variables;

pub(crate) use security::change_id;

use egui::{Align, CornerRadius, FontFamily, Id, Layout, Rect, Role, Sense, Stroke, Ui, Vec2};
use i18n::{Key, Message, keys};
use zeroize::Zeroizing;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::services::settings::{Settings, SettingsDraft};
use crate::state::AppState;
use crate::state::security::Variable;
use crate::theme::palette;
use crate::widgets::{self, Button, Kind};

const SIZE: Vec2 = Vec2::new(760.0, 560.0);
const RAIL: f32 = 190.0;
const HEADER: f32 = 48.0;
const FOOTER: f32 = 56.0;
/// What the three buttons take of the footer, whatever their language.
const BUTTONS: f32 = 270.0;

/// What the panel asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// The theme or the density changed in the draft.
    Preview,
    Apply,
    ApplyAndClose,
    /// Every way out that drops the draft.
    Discard,
    ChangePassphrase,
    /// Typing again withdraws the refusal.
    TypedCurrent,
    SubmitPassphrase,
    KeepPassphrase,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Page {
    #[default]
    General,
    Security,
    Variables,
}

impl Page {
    /// The order on screen: what the application looks like, then the keys that reach it, then
    /// what protects the library, then the corpus's own variables.
    const ALL: [(Self, Key, &'static str); 3] = [
        (Self::General, keys::settings::pages::GENERAL, "general"),
        (Self::Security, keys::settings::pages::SECURITY, "security"),
        (
            Self::Variables,
            keys::settings::pages::VARIABLES,
            "variables",
        ),
    ];
}

#[derive(Default)]
pub(super) struct View {
    draft: Option<SettingsDraft>,
    page: Page,
    /// Shown in place of the buttons once a close was attempted with work in hand: Escape and
    /// the backdrop produce no click, so either would be a silent Cancel.
    confirming_close: bool,
    variables: Option<Vec<Variable>>,
    change: security::ChangeView,
}

impl View {
    pub(super) fn open(&mut self, settings: &Settings) {
        *self = Self {
            draft: Some(SettingsDraft::open(settings.clone())),
            ..Self::default()
        };
    }

    pub(super) fn previewed(&self) -> Option<Settings> {
        self.draft.as_ref().map(SettingsDraft::previewed)
    }

    pub(super) fn adopt(&mut self) -> Option<Settings> {
        self.confirming_close = false;
        self.draft.as_mut().map(SettingsDraft::apply)
    }

    pub(super) fn changed_variables(&self, stored: Option<&[Variable]>) -> Option<Vec<Variable>> {
        self.variables
            .as_ref()
            .filter(|rows| Some(rows.as_slice()) != stored)
            .cloned()
    }

    pub(super) fn take_phrases(&mut self) -> (Zeroizing<String>, Zeroizing<String>) {
        self.change.take()
    }

    fn pending(&self, stored: Option<&[Variable]>) -> u32 {
        let settings = self.draft.as_ref().map_or(0, SettingsDraft::pending);
        settings + u32::from(self.changed_variables(stored).is_some())
    }
}

pub(crate) fn id() -> Id {
    Id::unique("settings-dialog")
}

pub(super) fn show(ctx: &egui::Context, state: &AppState, view: &mut View, actions: &mut Actions) {
    let shown = widgets::framed_dialog(ctx, &state.dialogs, id(), SIZE, |ui| {
        body(ui, state, view, actions);
    });
    security::dialog(ctx, state, view, actions);
    if !shown.is_some_and(|shown| shown.should_close()) {
        return;
    }
    if view.pending(state.security.variables()) > 0 {
        view.confirming_close = true;
    } else {
        actions.push(Asked::Menu(Action::Preferences(Ask::Discard)));
    }
}

fn body(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let whole = ui.max_rect();
    let language = state.language;
    let title = i18n::translate(language, keys::settings::TITLE, &[]);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let at = egui::pos2(
        whole.left() + 20.0,
        whole.top() + (HEADER - words.size().y) / 2.0,
    );
    let heading = ui.allocate_rect(Rect::from_min_size(at, words.size()), Sense::hover());
    ui.painter().galley(at, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(Role::Heading, true, &title));
    widgets::address(ui, &heading, "settings-dialog");
    let top = whole.top() + HEADER;
    let bottom = whole.bottom() - FOOTER;
    let line = Stroke::new(1.0, p.line_soft);
    ui.painter().hline(whole.x_range(), top, line);
    ui.painter().hline(whole.x_range(), bottom, line);

    let rail = Rect::from_min_max(
        egui::pos2(whole.left(), top),
        egui::pos2(whole.left() + RAIL, bottom),
    );
    ui.painter().rect_filled(rail, CornerRadius::ZERO, p.bg_0);
    ui.painter().vline(rail.right(), rail.y_range(), line);
    let mut pages = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rail.shrink2(Vec2::new(8.0, 10.0)))
            .layout(Layout::top_down(Align::Min)),
    );
    pages_rail(&mut pages, state, view);

    let content = Rect::from_min_max(
        egui::pos2(rail.right(), top),
        egui::pos2(whole.right(), bottom),
    );
    let mut page = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    let area = egui::ScrollArea::vertical()
        .id_salt("settings-content")
        .auto_shrink(false)
        .show(&mut page, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 20,
                    right: 20,
                    top: 16,
                    bottom: 20,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    current_page(ui, state, view, actions);
                });
        });
    let region = widgets::region(
        ui,
        area.inner_rect,
        "settings-content-region",
        Sense::hover(),
    );
    widgets::address(ui, &region, "settings-content");

    let footer = Rect::from_min_max(
        egui::pos2(whole.left() + 20.0, bottom),
        egui::pos2(whole.right() - 20.0, whole.bottom()),
    );
    let mut under = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(footer)
            .layout(Layout::left_to_right(Align::Center)),
    );
    if view.confirming_close {
        confirming(&mut under, state, view, actions);
    } else {
        buttons(&mut under, state, view, actions);
    }
}

fn pages_rail(ui: &mut Ui, state: &AppState, view: &mut View) {
    let p = palette(ui.ctx());
    ui.spacing_mut().item_spacing.y = 2.0;
    for (page, name, id) in Page::ALL {
        let name = i18n::translate(state.language, name, &[]);
        let on = view.page == page;
        let corner = CornerRadius::same(6);
        let response = widgets::interact(
            ui,
            Vec2::new(ui.available_width(), 31.0),
            Sense::click(),
            &format!("settings-page-{id}"),
            || egui::WidgetInfo::selected(Role::Tab, true, on, &name),
            corner,
        );
        if on {
            ui.painter()
                .rect_filled(response.rect, corner, p.amber_fill.gamma_multiply(0.12));
        } else if response.hovered() {
            ui.painter().rect_filled(response.rect, corner, p.bg_2);
        }
        let ink = if on {
            p.amber_text
        } else if response.hovered() {
            p.text_0
        } else {
            p.text_1
        };
        let words = widgets::text(ui, &name, 12.5, 400.0, FontFamily::Proportional, ink);
        let at = egui::pos2(
            response.rect.left() + 10.0,
            response.rect.center().y - words.size().y / 2.0,
        );
        ui.painter().galley(at, words, ink);
        if response.clicked() {
            view.page = page;
        }
    }
}

fn current_page(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let (_, name, _) = Page::ALL
        .into_iter()
        .find(|(page, ..)| *page == view.page)
        .unwrap_or(Page::ALL[0]);
    let name = i18n::translate(state.language, name, &[]);
    let words = widgets::text(ui, &name, 13.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, _) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    ui.add_space(14.0);
    let Some(draft) = &mut view.draft else {
        return;
    };
    match view.page {
        Page::General => general::show(ui, state, draft, actions),
        Page::Security => {
            let mut settings = draft.draft().clone();
            security::page(ui, state, &mut settings, actions);
            draft.edit(|edited| *edited = settings);
        }
        Page::Variables => variables::page(ui, state, &mut view.variables),
    }
}

fn buttons(ui: &mut Ui, state: &AppState, view: &View, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    if view.page == Page::General {
        let hint = egui::RichText::new(tr(keys::settings::PREVIEW_HINT))
            .size(11.5)
            .color(p.text_2);
        let room = Vec2::new(ui.available_width() - BUTTONS, FOOTER);
        ui.allocate_ui_with_layout(room, Layout::left_to_right(Align::Center), |ui| {
            ui.add(egui::Label::new(hint).wrap());
        });
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut ask = |ask| actions.push(Asked::Menu(Action::Preferences(ask)));
        if Button::new(&tr(keys::settings::OK), "settings-close")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
        {
            ask(Ask::ApplyAndClose);
        }
        if Button::new(&tr(keys::settings::APPLY), "settings-apply")
            .enabled(view.pending(state.security.variables()) > 0)
            .show(ui)
            .clicked()
        {
            ask(Ask::Apply);
        }
        if Button::new(&tr(keys::settings::CANCEL), "settings-cancel")
            .show(ui)
            .clicked()
        {
            ask(Ask::Discard);
        }
    });
}

fn confirming(ui: &mut Ui, state: &AppState, view: &View, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let said = Message::new(keys::settings::UNAPPLIED)
        .with("count", view.pending(state.security.variables()))
        .text(language);
    let warning = ui.label(egui::RichText::new(said).size(12.0).color(p.red));
    widgets::address(ui, &warning, "settings-unapplied");
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut ask = |ask| actions.push(Asked::Menu(Action::Preferences(ask)));
        if Button::new(&tr(keys::settings::APPLY_AND_CLOSE), "settings-apply-close")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
        {
            ask(Ask::ApplyAndClose);
        }
        if Button::new(&tr(keys::settings::CLOSE_WITHOUT), "settings-discard")
            .kind(Kind::Destructive)
            .show(ui)
            .clicked()
        {
            ask(Ask::Discard);
        }
    });
}
