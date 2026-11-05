//! « Présenter » (the reference's `presentation`): the Note over everything, its title first as
//! a cover, then each slide the core cut from the draft, read larger. `M` turns to the map of
//! its headings, where a heading pressed opens its slide. Escape leaves.

mod mind_map;
#[cfg(test)]
mod tests;

use egui::text::LayoutJob;
use egui::{
    Align2, CornerRadius, FontId, Id, Key, Modifiers, Rect, Sense, TextFormat, Ui, UiBuilder, Vec2,
    pos2, vec2,
};
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Editing};
use crate::state::AppState;
use crate::state::presentation::Presenting;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::rich::RichView;
use crate::widgets::{self, Button, IconButton, Kind};

/// The widest a slide reads.
const WIDEST: f32 = 1100.0;

/// Where the presenter is: a slide (0 is the cover) or the map.
#[derive(Default)]
pub(in crate::notes) struct PresentationView {
    session: u64,
    at: usize,
    map: bool,
}

impl PresentationView {
    fn go(&mut self, at: usize, count: usize) {
        self.at = at.min(count - 1);
    }
}

pub(in crate::notes) fn show(
    ui: &Ui,
    state: &AppState,
    view: &mut PresentationView,
    actions: &mut Actions,
) {
    let Some(presenting) = state.presentation.shown() else {
        return;
    };
    if view.session != presenting.session {
        *view = PresentationView {
            session: presenting.session,
            ..PresentationView::default()
        };
    }
    let p = palette(ui.ctx());
    let id = Id::unique("presentation");
    let screen = ui.ctx().content_rect();
    let area = egui::Modal::default_area(id).anchor(Align2::LEFT_TOP, Vec2::ZERO);
    let shown = egui::Modal::new(id)
        .area(area)
        .backdrop_color(p.bg_0)
        .frame(egui::Frame::new().fill(p.bg_0))
        .show(ui.ctx(), |ui| {
            let (whole, response) = ui.allocate_exact_size(screen.size(), Sense::hover());
            let count = presenting.slides.len() + 1;
            let said = i18n::translate(
                state.language,
                keys::presentation::SLIDE,
                &[
                    ("at", (view.at + 1).to_string().into()),
                    ("count", count.to_string().into()),
                ],
            );
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Dialog, true, &said));
            widgets::address(ui, &response, "presentation");
            keys_of(ui, view, count);
            if view.map {
                let mut map_ui = ui.new_child(UiBuilder::new().max_rect(whole));
                let chosen = mind_map::show(&mut map_ui, whole, &presenting.title, &presenting.map);
                if let Some(at) = chosen {
                    view.map = false;
                    view.go(at, count);
                }
            } else {
                stage(ui, whole, state, presenting, view.at, actions);
            }
            progress(ui, whole, view.at + 1, count);
            bar(ui, whole, state, view, count)
        });
    if shown.inner || shown.should_close() {
        actions.push(Action::Notes(Asked::Editor(Editing::StopPresenting)));
    }
}

/// What moves a slide, as a presenter's clicker sends it too; `M` turns to the map and back.
fn keys_of(ui: &Ui, view: &mut PresentationView, count: usize) {
    let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    if pressed(Key::M) {
        view.map = !view.map;
    }
    if view.map {
        return;
    }
    // A button that has the focus answers Enter and Space itself.
    let focused = ui.memory(|memory| memory.focused().is_some());
    let next = [Key::ArrowRight, Key::ArrowDown, Key::PageDown]
        .into_iter()
        .chain(
            (!focused)
                .then_some([Key::Space, Key::Enter])
                .into_iter()
                .flatten(),
        )
        .any(pressed);
    let previous = [Key::ArrowLeft, Key::ArrowUp, Key::PageUp, Key::Backspace]
        .into_iter()
        .any(pressed);
    if next {
        view.go(view.at + 1, count);
    } else if previous {
        view.go(view.at.saturating_sub(1), count);
    } else if pressed(Key::Home) {
        view.go(0, count);
    } else if pressed(Key::End) {
        view.go(count - 1, count);
    }
}

/// The cover, or the slide read from across a room.
fn stage(
    ui: &mut Ui,
    whole: Rect,
    state: &AppState,
    presenting: &Presenting,
    at: usize,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let (vw, vh) = (whole.width() / 100.0, whole.height() / 100.0);
    let room = Rect::from_min_max(
        whole.min + vec2(6.0 * vw, 6.0 * vh),
        whole.max - vec2(6.0 * vw, 10.0 * vh),
    );
    let width = room.width().min(WIDEST);
    let column = Rect::from_center_size(room.center(), vec2(width, room.height()));
    let Some(slide) = at.checked_sub(1).and_then(|at| presenting.slides.get(at)) else {
        let mut format =
            TextFormat::simple(FontId::proportional((6.0 * vw).clamp(36.0, 72.0)), p.text_0);
        format.coords.push(*b"wght", 700.0);
        let mut job = LayoutJob::single_section(presenting.title.clone(), format);
        job.wrap.max_width = width;
        let galley = ui.painter().layout_job(job);
        let rect = Rect::from_center_size(column.center(), galley.size());
        let response = widgets::region(ui, rect, "presentation-cover", Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Heading, true, &presenting.title)
        });
        widgets::address(ui, &response, "presentation-cover");
        ui.painter().galley(rect.min, galley, p.text_0);
        return;
    };
    let scale = (2.2 * vw).clamp(20.0, 30.0) / crate::theme::fonts::BASE;
    let language = state.language;
    let pictures = |id: &str| crate::notes::overlays::editor::embedded(state, id);
    let missing = i18n::translate(language, keys::drawing::MISSING, &[]);
    let ctx = ui.ctx().clone();
    let diagrams = |source: &str| state.caches.diagrams.get(&ctx, source);
    let mut slide_ui = ui.new_child(UiBuilder::new().max_rect(column));
    // Centred by the height it took last frame: a slide is laid out before it is measured.
    let measured = ui.unique_id().with(("presentation-slide", at));
    let last = ui.data(|data| data.get_temp::<f32>(measured));
    let lead = last.map_or(0.0, |height| ((column.height() - height) / 2.0).max(0.0));
    let painted = egui::ScrollArea::vertical()
        .id_salt(("presentation-slide", at))
        .show(&mut slide_ui, |ui| {
            ui.set_width(width);
            ui.add_space(lead);
            let shown = ui.scope(|ui| {
                RichView::new(slide)
                    .scale(scale)
                    .pictures(&pictures, &missing)
                    .diagrams(&diagrams, language)
                    .show(ui)
            });
            let (painted, height) = (shown.inner, shown.response.rect.height());
            if last.is_none_or(|last| (last - height).abs() > 0.5) {
                ui.data_mut(|data| data.insert_temp(measured, height));
                ui.ctx().request_repaint();
            }
            painted
        })
        .inner;
    for id in painted.wanted {
        actions.push(Action::Notes(Asked::Picture(id)));
    }
    for sketch in painted.sketches {
        actions.push(Action::Notes(Asked::Diagram(sketch.source)));
    }
    if let Some(url) = painted.link {
        actions.push(Action::Notes(Asked::Editor(Editing::OpenLink(url))));
    }
}

/// How far along: a line at the foot of the screen.
#[expect(clippy::cast_precision_loss, reason = "a count of slides")]
fn progress(ui: &Ui, whole: Rect, shown: usize, count: usize) {
    let done = shown as f32 / count as f32;
    let p = palette(ui.ctx());
    let track = Rect::from_min_max(pos2(whole.left(), whole.bottom() - 3.0), whole.max);
    ui.painter()
        .rect_filled(track, CornerRadius::ZERO, p.line_soft);
    let filled = Rect::from_min_size(track.min, vec2(track.width() * done, track.height()));
    ui.painter()
        .rect_filled(filled, CornerRadius::ZERO, p.amber_fill);
}

/// Out of the way of the slide, dimmed until the pointer reaches it; whether it was left.
fn bar(
    ui: &mut Ui,
    whole: Rect,
    state: &AppState,
    view: &mut PresentationView,
    count: usize,
) -> bool {
    let tr = |key| i18n::translate(state.language, key, &[]);
    let room = Rect::from_min_max(whole.max - vec2(320.0, 58.0), whole.max - vec2(16.0, 18.0));
    let mut bar_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(room)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    if !bar_ui.rect_contains_pointer(room) {
        bar_ui.set_opacity(0.45);
    }
    bar_ui.spacing_mut().item_spacing.x = 6.0;
    let closed = IconButton::new(
        Icon::X,
        &tr(keys::presentation::CLOSE),
        "presentation-close",
    )
    .show(&mut bar_ui)
    .clicked();
    let toggle = if view.map {
        keys::presentation::SLIDES
    } else {
        keys::presentation::MAP
    };
    if Button::new(&tr(toggle), "presentation-map")
        .kind(Kind::Outline)
        .show(&mut bar_ui)
        .clicked()
    {
        view.map = !view.map;
    }
    let next = tr(keys::presentation::NEXT);
    let can = !view.map && view.at + 1 < count;
    let next = bar_ui.add_enabled_ui(can, |ui| {
        IconButton::new(Icon::ArrowRight, &next, "presentation-next").show(ui)
    });
    if next.inner.clicked() {
        view.go(view.at + 1, count);
    }
    let words = format!("{} / {count}", view.at + 1);
    let p = palette(ui.ctx());
    let counted = bar_ui.add_sized(
        [56.0, 24.0],
        egui::Label::new(
            egui::RichText::new(words)
                .monospace()
                .size(12.5)
                .color(p.text_1),
        ),
    );
    widgets::address(&bar_ui, &counted, "presentation-count");
    let previous = tr(keys::presentation::PREVIOUS);
    let can = !view.map && view.at > 0;
    let previous = bar_ui.add_enabled_ui(can, |ui| {
        IconButton::new(Icon::ArrowLeft, &previous, "presentation-previous").show(ui)
    });
    if previous.inner.clicked() {
        view.go(view.at.saturating_sub(1), count);
    }
    closed
}
