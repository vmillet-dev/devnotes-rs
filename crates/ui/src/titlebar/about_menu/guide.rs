//! The written guide, one chapter at a time: a schematic of the screen it is about and two
//! sentences beside it, short enough to be read standing up. « Close » is on every chapter: a walk
//! you can only leave by finishing it is a wall with extra steps.

use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Role, Sense, Stroke, Ui, Vec2};
use engine::app_info::METADATA;
use i18n::{Key, Language, Message, keys};

use super::{Panel, framed};
use crate::action::Actions;
use crate::services::guide::{self, CHAPTERS, Ink, Shape};
use crate::services::shortcuts;
use crate::services::shortcuts::Bindings;
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::{self, Button, Kind};

const SIZE: Vec2 = Vec2::new(620.0, 560.0);
/// The space the reference's figures are drawn in.
const FIGURE: Vec2 = Vec2::new(320.0, 120.0);
/// What the fourteen dots take, each 24 px and 2 apart.
const DOTS: f32 = 14.0 * 26.0;

pub(super) fn show(
    ctx: &egui::Context,
    state: &AppState,
    chapter: usize,
    _actions: &mut Actions,
    panel: &mut Option<Panel>,
) -> bool {
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let mut close = false;
    let (mut dot, mut step) = (None, None);
    let shown = widgets::framed_dialog(ctx, &state.dialogs, super::id(), SIZE, |ui| {
        let title = tr(keys::getting_started::TITLE);
        let subtitle = Message::new(keys::getting_started::SUBTITLE)
            .with("app", METADATA.name)
            .text(language);
        framed(
            ui,
            (&title, &subtitle),
            |ui| {
                dot = body(ui, state, chapter);
            },
            |ui| {
                (step, close) = footer(ui, language, chapter);
            },
        );
    });
    if let Some(chapter) = dot.or(step) {
        *panel = Some(Panel::Guide(chapter));
    }
    close || shown.is_some_and(|shown| shown.should_close())
}

/// The chapter a dot asked for.
fn body(ui: &mut Ui, state: &AppState, at: usize) -> Option<usize> {
    let language = state.language;
    let p = palette(ui.ctx());
    let chapter = CHAPTERS[at];
    figure(ui, chapter);
    ui.add_space(12.0);
    let step = format!("{} / {}", at + 1, CHAPTERS.len());
    ui.label(RichText::new(step).size(11.0).monospace().color(p.text_2));
    if let Some((title, words)) = guide::words(chapter) {
        let heading = ui.label(
            RichText::new(i18n::translate(language, title, &[]))
                .size(13.5)
                .variation(*b"wght", 600.0)
                .color(p.text_0),
        );
        widgets::address(ui, &heading, "guide-chapter");
        let said = body_text(words, &state.settings.bindings, language);
        let said = RichText::new(said).size(12.5).color(p.text_1);
        ui.add(egui::Label::new(said).wrap());
    }
    ui.add_space(12.0);
    dots(ui, language, at)
}

/// A body cannot spell a key out: the palette's follows the preference.
fn body_text(key: Key, bindings: &Bindings, language: Language) -> String {
    let bound = |id| {
        shortcuts::every()
            .find(|action| action.id == id)
            .map_or_else(String::new, |action| bindings.of(action).to_owned())
    };
    Message::new(key)
        .with("palette", bound(shortcuts::PALETTE))
        .with("capture", bound(shortcuts::CAPTURE))
        .with("newNote", bound(shortcuts::NEW_NOTE))
        .with("check", bound("canvas.check"))
        .with("app", METADATA.name)
        .text(language)
}

/// Decorative: the text says the same thing, so nothing is lost by not seeing it.
fn figure(ui: &mut Ui, chapter: &str) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let scale = width / FIGURE.x;
    let (band, _) = ui.allocate_exact_size(Vec2::new(width, FIGURE.y * scale), Sense::hover());
    ui.painter()
        .rect_filled(band, CornerRadius::same(8), p.bg_0);
    let place = |x: f32, y: f32| band.min + Vec2::new(x, y) * scale;
    for shape in guide::figure(chapter) {
        let colour = ink(ui, shape);
        let stroke = Stroke::new(scale, colour);
        let stroked = matches!(shape.class, Ink::Line | Ink::AccentLine);
        if let Some([x, y, w, h, corner]) = shape.rect {
            let rect = Rect::from_min_size(place(x, y), Vec2::new(w, h) * scale);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a few points"
            )]
            let corner = CornerRadius::same((corner * scale) as u8);
            if stroked {
                ui.painter()
                    .rect_stroke(rect, corner, stroke, egui::StrokeKind::Middle);
            } else {
                ui.painter().rect_filled(rect, corner, colour);
            }
        }
        if let Some([x, y, radius]) = shape.circle {
            ui.painter()
                .circle_filled(place(x, y), radius * scale, colour);
        }
        for line in shape.lines.iter().flatten() {
            let points = line.iter().map(|[x, y]| place(*x, *y)).collect();
            ui.painter().add(egui::Shape::line(points, stroke));
        }
    }
}

fn ink(ui: &Ui, shape: &Shape) -> Color32 {
    let p = palette(ui.ctx());
    let colour = match shape.class {
        Ink::Panel => p.bg_2,
        Ink::Line => p.line,
        Ink::Ink => p.text_2,
        Ink::Accent => p.amber_fill,
        Ink::AccentLine => p.amber_dim,
    };
    colour.gamma_multiply(shape.opacity.unwrap_or(1.0))
}

/// Dots rather than a list of names: the walk is the navigation, and a reader who knows which
/// chapter they want can still land on it.
fn dots(ui: &mut Ui, language: Language, at: usize) -> Option<usize> {
    let p = palette(ui.ctx());
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add_space(((ui.available_width() - DOTS) / 2.0).max(0.0));
        for (index, chapter) in CHAPTERS.into_iter().enumerate() {
            let name = guide::words(chapter).map_or_else(String::new, |(title, _)| {
                i18n::translate(language, title, &[])
            });
            let on = index == at;
            let response = widgets::interact(
                ui,
                Vec2::splat(24.0),
                Sense::click(),
                &format!("guide-dot-{chapter}"),
                || egui::WidgetInfo::selected(Role::Tab, true, on, &name),
                CornerRadius::same(12),
            );
            let response = widgets::tooltip(response, &name);
            let ink = if on { p.amber_fill } else { p.line };
            ui.painter()
                .circle_filled(response.rect.center(), if on { 4.5 } else { 3.0 }, ink);
            if response.clicked() {
                chosen = Some(index);
            }
        }
    });
    chosen
}

/// Where the arrows go, and whether « Close » was pressed.
fn footer(ui: &mut Ui, language: Language, at: usize) -> (Option<usize>, bool) {
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let mut to = None;
    if Button::new(&tr(keys::getting_started::PREVIOUS), "guide-previous")
        .enabled(at > 0)
        .show(ui)
        .clicked()
    {
        to = at.checked_sub(1);
    }
    let close = ui
        .with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if at + 1 < CHAPTERS.len()
                && Button::new(&tr(keys::getting_started::NEXT), "guide-next")
                    .kind(Kind::Filled)
                    .show(ui)
                    .clicked()
            {
                to = Some(at + 1);
            }
            Button::new(&tr(keys::getting_started::CLOSE), "guide-close")
                .show(ui)
                .clicked()
        })
        .inner;
    (to, close)
}
