//! The application's bar: the window keeps the system's decorations, so this is a menu bar
//! inside it — the menus and the areas on the left, the name in the middle, the theme on the
//! right. 38 px on `bg-0`, a `line-soft` underline.

pub(crate) mod about_menu;
mod file_menu;
pub(crate) mod libraries;
pub(crate) mod preferences;

use egui::{
    CornerRadius, FontFamily, Margin, Rect, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
};
use engine::app_info::METADATA;
use i18n::{Key, Language, Message, keys};
use zeroize::Zeroizing;

use crate::action::{Action as Asked, Actions, AppAction};
use crate::gate::prompt::{self, Answer, PromptView};
use crate::services::settings::Settings;
use crate::state::security::Variable;
use crate::state::{AppState, Area};
use crate::theme::icons::{self, Icon};
use crate::theme::tokens::TITLEBAR_HEIGHT;
use crate::theme::{Appearance, palette};
use crate::widgets::{self, tooltip};

/// In the switch's order, which is also the order of `Ctrl+1/2/3`.
const AREAS: [(Area, Icon, Key, &str); 3] = [
    (Area::Notes, Icon::FileText, keys::areas::NOTES, "notes"),
    (Area::Tools, Icon::Wrench, keys::areas::TOOLS, "tools"),
    (Area::Http, Icon::ArrowLeftRight, keys::areas::HTTP, "http"),
];

/// What the bar's menus ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Import,
    ExportAll,
    ExportSpace,
    ExportSelection,
    CopyMarkdown,
    /// The passphrase prompt answered: the answer waits in the view, out of the actions.
    Prompted,
    Quit,
    OpenLibraries,
    Libraries(libraries::Ask),
    OpenPreferences,
    Preferences(preferences::Ask),
    OpenHelp(about_menu::Panel),
    CloseHelp,
    OpenLink(String),
    Update(crate::banners::update_prompt::Ask),
}

/// What only the bar's drawing sees.
#[derive(Default)]
pub struct View {
    file_menu: file_menu::View,
    /// The menu's control, which gets the focus back from what it opened.
    pub file_anchor: Option<egui::Id>,
    libraries: libraries::View,
    preferences: preferences::View,
    about: about_menu::View,
    update: crate::banners::update_prompt::UpdateView,
    prompt: PromptView,
    /// ⚠️ Holds a phrase until `AppState::apply` takes it, the same frame.
    pub answer: Option<Answer>,
}

impl View {
    pub fn open_preferences(&mut self, settings: &Settings) {
        self.preferences.open(settings);
    }

    pub fn close_preferences(&mut self) {
        self.preferences = preferences::View::default();
    }

    pub fn show_help(&mut self, panel: about_menu::Panel) {
        self.about.show_panel(panel);
    }

    pub fn close_help(&mut self) {
        self.about.close_panel();
    }

    pub fn about_anchor(&self) -> Option<egui::Id> {
        self.about.anchor
    }

    /// What the window shows while the panel is open.
    pub fn previewed(&self) -> Option<Settings> {
        self.preferences.previewed()
    }

    pub fn adopt(&mut self) -> Option<Settings> {
        self.preferences.adopt()
    }

    /// The rows to send, when they differ from `stored`.
    pub fn adopt_variables(&self, stored: Option<&[Variable]>) -> Option<Vec<Variable>> {
        self.preferences.changed_variables(stored)
    }

    /// The phrases typed in the change dialog, its fields left empty.
    pub fn take_phrases(&mut self) -> (Zeroizing<String>, Zeroizing<String>) {
        self.preferences.take_phrases()
    }
}

pub(crate) fn show(ui: &mut Ui, state: &AppState, view: &mut View, actions: &mut Actions) {
    let language = state.language;
    let unlocked = state.gate.is_open();
    let p = palette(ui.ctx());
    let bar = ui.max_rect();
    ui.painter().rect_filled(bar, CornerRadius::ZERO, p.bg_0);
    ui.painter().hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );

    // Centred on the window, not on what the two groups leave between them.
    let title = widgets::text(
        ui,
        METADATA.name,
        12.0,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    );
    let at = bar.center() - title.size() / 2.0;
    ui.painter().galley(at, title, p.text_2);

    let inner = bar - Margin::symmetric(12, 0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.horizontal_centered(|ui| {
            // Every entry of the menus acts on the library: none while it is locked.
            if unlocked {
                let anchor = file_menu::show(ui, state, &mut view.file_menu, actions);
                view.file_anchor = Some(anchor);
            }
            about_menu::show(ui, state, &mut view.about, actions);
            if unlocked {
                area_switch(ui, state.settings.area, language, actions);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                theme_toggle(ui, language, actions);
            });
        });
    });
}

/// What the menus open, over everything else.
pub(crate) fn dialogs(
    ctx: &egui::Context,
    state: &AppState,
    view: &mut View,
    actions: &mut Actions,
) {
    crate::banners::update_prompt::show(ctx, state, &mut view.update, actions);
    about_menu::panels(ctx, state, &mut view.about, actions);
    libraries::show(ctx, state, &mut view.libraries, actions);
    preferences::show(ctx, state, &mut view.preferences, actions);
    if let Some(request) = state.transfer.prompt() {
        let asked = prompt::show(
            ctx,
            &state.dialogs,
            &request,
            &mut view.prompt,
            state.language,
        );
        if let Some(answer) = asked {
            view.answer = Some(answer);
            actions.push(Asked::Menu(Action::Prompted));
        }
    }
}

/// A menu's control: its name and a chevron, `bg-2` under the pointer or while it is open.
fn menu_trigger(ui: &mut Ui, label: &str, author_id: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let probe = widgets::text(ui, label, 11.5, 400.0, FontFamily::Proportional, p.text_2);
    let corner = CornerRadius::same(6);
    let size = Vec2::new(probe.size().x + 29.0, 24.0);
    let response = widgets::interact(
        ui,
        size,
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let lit = open || response.hovered();
    if lit {
        ui.painter().rect_filled(response.rect, corner, p.bg_2);
    }
    let ink = if lit { p.text_0 } else { p.text_2 };
    let words = widgets::text(ui, label, 11.5, 400.0, FontFamily::Proportional, ink);
    let at = egui::pos2(
        response.rect.left() + 8.0,
        response.rect.center().y - words.size().y / 2.0,
    );
    ui.painter().galley(at, words, ink);
    let glyph = Rect::from_center_size(
        egui::pos2(response.rect.right() - 10.0, response.rect.center().y),
        Vec2::splat(10.0),
    );
    icons::paint(ui, glyph, Icon::ChevronDown, ink);
    response
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Plain,
    /// Nothing to act on: it stays where it is, muted, and a click does nothing.
    Idle,
    /// Red at rest, like every destructive control.
    Danger,
}

/// A line of the bar's menus: 7 × 10, 6 px, 12.5 px; `bg-3` under the pointer.
fn menu_option(ui: &mut Ui, label: &str, author_id: &str, tone: Tone) -> egui::Response {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(6);
    let response = widgets::interact(
        ui,
        Vec2::new(ui.available_width(), 29.0),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::MenuItem, tone != Tone::Idle, label),
        corner,
    );
    let lit = tone != Tone::Idle && (response.hovered() || response.has_focus());
    if lit {
        ui.painter().rect_filled(response.rect, corner, p.bg_3);
    }
    let ink = match tone {
        Tone::Danger => p.red,
        Tone::Idle => p.text_2,
        Tone::Plain if lit => p.text_0,
        Tone::Plain => p.text_1,
    };
    let words = widgets::text(ui, label, 12.5, 400.0, FontFamily::Proportional, ink);
    let at = egui::pos2(
        response.rect.left() + 10.0,
        response.rect.center().y - words.size().y / 2.0,
    );
    ui.painter().galley(at, words, ink);
    response
}

fn separator(ui: &mut Ui) {
    let line = palette(ui.ctx()).line_soft;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, line));
}

/// The areas in the bar: icons alone, their names for the screen reader and the tooltip.
fn area_switch(ui: &mut Ui, current: Area, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let group = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for (rank, (area, icon, name, id)) in AREAS.into_iter().enumerate() {
                    let name = i18n::translate(language, name, &[]);
                    let on = area == current;
                    let response = widgets::interact(
                        ui,
                        Vec2::new(26.0, 24.0),
                        Sense::click(),
                        &format!("area-option-{id}"),
                        || WidgetInfo::selected(Role::Tab, true, on, &name),
                        CornerRadius::same(6),
                    );
                    let hint = Message::new(keys::areas::HINT)
                        .with("area", name.as_str())
                        .with("key", format!("Ctrl+{}", rank + 1))
                        .text(language);
                    let response = tooltip(response, &hint);
                    if on {
                        ui.painter()
                            .rect_filled(response.rect, CornerRadius::same(6), p.bg_3);
                    }
                    let ink = if on {
                        p.amber_text
                    } else if response.hovered() {
                        p.text_0
                    } else {
                        p.text_2
                    };
                    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(14.0));
                    icons::paint(ui, glyph, icon, ink);
                    if response.clicked() && !on {
                        actions.push(Asked::App(AppAction::ShowArea(area)));
                    }
                }
            });
        });
    widgets::address(ui, &group.response, "area-switch");
}

/// One gesture, two themes: the icon and the label both name what it switches **to**.
fn theme_toggle(ui: &mut Ui, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let to = match ui.ctx().theme() {
        egui::Theme::Dark => Appearance::Light,
        egui::Theme::Light => Appearance::Dark,
    };
    let (icon, label) = match to {
        Appearance::Dark => (Icon::Moon, keys::settings::theme::TO_DARK),
        _ => (Icon::Sun, keys::settings::theme::TO_LIGHT),
    };
    let label = i18n::translate(language, label, &[]);
    let corner = CornerRadius::same(6);
    let response = widgets::interact(
        ui,
        Vec2::new(27.0, 24.0),
        Sense::click(),
        "theme-toggle",
        || WidgetInfo::labeled(Role::Button, true, &label),
        corner,
    );
    let response = tooltip(response, &label);
    ui.painter().rect(
        response.rect.shrink2(Vec2::new(0.0, 2.5)),
        corner,
        egui::Color32::TRANSPARENT,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    let ink = if response.hovered() {
        p.text_1
    } else {
        p.text_2
    };
    let glyph = Rect::from_center_size(response.rect.center(), Vec2::splat(13.0));
    icons::paint(ui, glyph, icon, ink);
    if response.clicked() {
        actions.push(Asked::App(AppAction::SetAppearance(to)));
    }
}

/// The bar's height, for the panel that holds it.
pub(crate) const HEIGHT: f32 = TITLEBAR_HEIGHT;
