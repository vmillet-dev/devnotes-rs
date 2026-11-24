//! Moving the look on the drawing: the wheel and the pinch, a drag of the middle button or of any
//! with Space held, their keys, and the buttons at the stage's corner.

use egui::{Align, CursorIcon, Key, Layout, Modifiers, PointerButton, Rect, Response, Ui, Vec2};
use i18n::{Language, keys};

use super::DrawingView;
use super::camera::{Camera, STEP};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::theme::tokens::{gap, radius};
use crate::widgets::{self, Button, IconButton, Kind};

/// What an arrow moves the look by.
const KEY_STEP: f32 = 60.0;

#[derive(Default)]
pub(super) struct Look {
    /// The stage was measured: the drawing opened at the size that shows it.
    placed: bool,
    /// The pointer moves the look, until it is let go.
    panning: bool,
}

/// The first time the stage has a size.
pub(super) fn open(view: &mut DrawingView, stage: Rect) {
    if !std::mem::replace(&mut view.look.placed, true) {
        let drawn = view.editor.drawing().bounds();
        view.camera = Camera::opening(stage.size(), drawn.as_ref());
    }
}

/// The pointer's part; `true` while it moves the look, and draws nothing.
pub(super) fn gestures(ui: &Ui, view: &mut DrawingView, stage: &Response) -> bool {
    if stage.contains_pointer() {
        wheel(ui, view, stage.rect);
    }
    let (middle, space, down, released, moved) = ui.input(|input| {
        (
            input.pointer.button_down(PointerButton::Middle),
            input.key_down(Key::Space),
            input.pointer.primary_down(),
            input.pointer.primary_released(),
            input.pointer.delta(),
        )
    });
    let space = space && view.typing.is_none();
    let pressed = ui.input(|input| input.pointer.any_pressed());
    if pressed && stage.contains_pointer() && (middle || space) {
        // What the pointer did before it, in the same frame, is not the drag's.
        view.look.panning = true;
        stage.request_focus();
        return true;
    }
    if view.look.panning {
        view.camera.pan_by(moved);
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        view.look.panning = middle || down;
        // The release that ends it is the look's too.
        return true;
    }
    // Pressed in the middle of a shape being drawn, Space leaves it to its release.
    let waiting = space && !down && !released;
    if waiting && stage.contains_pointer() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    waiting
}

/// The wheel moves the look; with Ctrl held, or pinched, it zooms where the pointer is.
fn wheel(ui: &Ui, view: &mut DrawingView, stage: Rect) {
    let (zoomed, scrolled, at) = ui.input_mut(|input| {
        let zoomed = input.zoom_delta();
        let scrolled = std::mem::take(&mut input.smooth_scroll_delta);
        (zoomed, scrolled, input.pointer.hover_pos())
    });
    if let Some(at) = at
        && (zoomed - 1.0).abs() > f32::EPSILON
    {
        view.camera.zoom_at(stage, at, zoomed);
    } else if scrolled != Vec2::ZERO {
        view.camera.pan_by(scrolled);
    }
}

/// `Ctrl` and +, − or 0 for the size, `F` to fit, `Ctrl` and an arrow to move.
pub(super) fn keys_of(ui: &Ui, view: &mut DrawingView, stage: Rect) {
    let pressed = |modifiers, key| ui.input_mut(|input| input.consume_key(modifiers, key));
    let closer = pressed(Modifiers::COMMAND, Key::Plus) || pressed(Modifiers::COMMAND, Key::Equals);
    if closer {
        zoom(view, stage, STEP);
    }
    if pressed(Modifiers::COMMAND, Key::Minus) {
        zoom(view, stage, 1.0 / STEP);
    }
    if pressed(Modifiers::COMMAND, Key::Num0) {
        whole_size(view, stage);
    }
    if pressed(Modifiers::NONE, Key::F) {
        fit(view, stage);
    }
    for (key, moved) in [
        (Key::ArrowLeft, Vec2::new(KEY_STEP, 0.0)),
        (Key::ArrowRight, Vec2::new(-KEY_STEP, 0.0)),
        (Key::ArrowUp, Vec2::new(0.0, KEY_STEP)),
        (Key::ArrowDown, Vec2::new(0.0, -KEY_STEP)),
    ] {
        if pressed(Modifiers::COMMAND, key) {
            view.camera.pan_by(moved);
        }
    }
}

fn zoom(view: &mut DrawingView, stage: Rect, factor: f32) {
    view.camera.zoom_at(stage, stage.center(), factor);
}

fn whole_size(view: &mut DrawingView, stage: Rect) {
    let factor = 1.0 / view.camera.zoom;
    zoom(view, stage, factor);
}

fn fit(view: &mut DrawingView, stage: Rect) {
    view.camera = view
        .editor
        .drawing()
        .bounds()
        .map_or_else(Camera::default, |drawn| Camera::fit(stage.size(), &drawn));
}

/// At the stage's corner: further, the size it is drawn at, closer, and the whole of it.
pub(super) fn controls(ui: &mut Ui, view: &mut DrawingView, stage: Rect, language: Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    let shown = format!("{:.0} %", view.camera.zoom * 100.0);
    // Quiet, the button is 6 px of padding a side, not the 12 its width counts.
    let widest = Button::width(ui, "800 %") - 12.0;
    let size = Vec2::new(
        3.0 * (widgets::MIN_TARGET + gap::LINE) + widest + 2.0 * gap::TIGHT,
        widgets::MIN_TARGET + 2.0 * gap::TIGHT,
    );
    let room = Rect::from_min_size(stage.max - size - Vec2::splat(gap::CONTROLS), size);
    ui.painter().rect(
        room,
        radius::MENU,
        p.bg_2,
        egui::Stroke::new(1.0, p.line_soft),
        egui::StrokeKind::Inside,
    );
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(room.shrink2(Vec2::new(gap::TIGHT, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = gap::LINE;
    let out = tr(keys::drawing::ZOOM_OUT);
    if IconButton::new(Icon::Minus, &out, "drawing-zoom-out")
        .show(&mut row)
        .clicked()
    {
        zoom(view, stage, 1.0 / STEP);
    }
    let reset = Button::new(&shown, "drawing-zoom")
        .kind(Kind::Quiet)
        .show(&mut row);
    if widgets::tooltip(reset, &tr(keys::drawing::ZOOM_RESET)).clicked() {
        whole_size(view, stage);
    }
    let closer = tr(keys::drawing::ZOOM_IN);
    if IconButton::new(Icon::Plus, &closer, "drawing-zoom-in")
        .show(&mut row)
        .clicked()
    {
        zoom(view, stage, STEP);
    }
    let whole = tr(keys::drawing::FIT);
    if IconButton::new(Icon::Maximize, &whole, "drawing-fit")
        .show(&mut row)
        .clicked()
    {
        fit(view, stage);
    }
}
