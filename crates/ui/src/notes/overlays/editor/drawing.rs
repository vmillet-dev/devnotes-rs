//! A Dessin's editor (the reference's `drawing-editor`): the toolbar over the stage. `drawing`
//! holds the scene and every gesture; this hands it the pointer and the keys and paints what it
//! answers. The scene is saved once the hand pauses; its pictures go only when it is left, as
//! taking two is dear.

use drawing::{Editor, Fill, Format, Frame, Grip, Hue, Mark, Point, Tool};
use egui::epaint::{PathShape, PathStroke, TextShape};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, Key, Layout, Modifiers, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};
use i18n::{Language, keys};

use crate::theme::icons::Icon;
use crate::theme::tokens::Palette;
use crate::theme::{fonts, palette};
use crate::widgets::{self, Button, IconButton};

/// What is drawn is saved once the hand pauses: each save writes the scene.
const SAVE_AFTER: f64 = 0.4;
const GRIP_SIZE: f32 = 9.0;
/// The reference's 0.06 and 0.14, out of 255.
const INK_TINT: u8 = 15;
const HUE_TINT: u8 = 36;
/// How far above a line's baseline its first glyphs start, in font sizes: Inter's ascent.
const ASCENT: f32 = 0.97;

const TOOLS: [(Tool, Icon, i18n::Key, &str, Key); 7] = [
    (
        Tool::Select,
        Icon::MousePointer,
        keys::drawing::SELECT,
        "select",
        Key::V,
    ),
    (
        Tool::Rectangle,
        Icon::Square,
        keys::drawing::RECTANGLE,
        "rectangle",
        Key::R,
    ),
    (
        Tool::Ellipse,
        Icon::Circle,
        keys::drawing::ELLIPSE,
        "ellipse",
        Key::O,
    ),
    (
        Tool::Diamond,
        Icon::Diamond,
        keys::drawing::DIAMOND,
        "diamond",
        Key::D,
    ),
    (
        Tool::Arrow,
        Icon::ArrowRight,
        keys::drawing::ARROW,
        "arrow",
        Key::A,
    ),
    (Tool::Pen, Icon::Pencil, keys::drawing::PEN, "pen", Key::P),
    (Tool::Text, Icon::Type, keys::drawing::TEXT, "text", Key::T),
];

/// The field over a text.
struct Typing {
    words: String,
    /// The press that opened it is not over: letting it go is not leaving the field.
    settling: bool,
}

/// Right to left, as the toolbar lays them out.
const EXPORTS: [(Format, i18n::Key, &str); 2] = [
    (Format::Svg, keys::drawing::EXPORT_SVG, "drawing-export-svg"),
    (Format::Png, keys::drawing::EXPORT_PNG, "drawing-export-png"),
];

pub(crate) struct DrawingView {
    editor: Editor,
    marks: Vec<Mark>,
    /// The editor's revision `marks` were read at.
    drawn: Option<u64>,
    typing: Option<Typing>,
    /// When what changed is to be written.
    save_at: Option<f64>,
    /// Something was drawn: from then on the body is the scene, not what the note holds.
    touched: bool,
    /// Something was drawn since the pictures were last taken.
    unpictured: bool,
    /// The stage held the keys at the last frame: egui drops the focus on Escape before it.
    held_keys: bool,
}

impl DrawingView {
    pub(super) fn open(content: &str) -> Self {
        Self {
            editor: Editor::open(content, fonts::for_drawings()),
            marks: Vec::new(),
            drawn: None,
            typing: None,
            save_at: None,
            touched: false,
            unpictured: false,
            held_keys: false,
        }
    }

    /// The body to write; `None` while nothing was drawn, which leaves a draft unwritten.
    pub(super) fn body(&self) -> Option<String> {
        self.touched.then(|| self.editor.body())
    }

    pub(super) fn shapes(&self) -> usize {
        self.editor.shapes()
    }

    /// What to write, for the note of that title.
    pub(super) fn export(
        &self,
        title: &str,
        format: Format,
        language: Language,
        p: &Palette,
    ) -> crate::notes::DrawingExport {
        crate::notes::DrawingExport {
            name: file_name(title, language),
            content: self.editor.body(),
            palette: std::sync::Arc::new(picture_palette(p)),
            format,
        }
    }

    /// The body once more when its pictures are owed: `true` is theirs to take.
    pub(super) fn take_unpictured(&mut self) -> Option<String> {
        std::mem::take(&mut self.unpictured).then(|| self.editor.body())
    }

    /// What is on it, for a drawing that may have come with no picture at all.
    pub(super) fn content_if_drawn(&self) -> Option<String> {
        (self.editor.shapes() > 0).then(|| self.editor.body())
    }
}

/// What a file name holds on every system: no control character, none of `\\ / : * ? " < > |`.
fn file_name(title: &str, language: Language) -> String {
    let printable: String = title
        .chars()
        .filter(|character| *character >= ' ')
        .map(|character| {
            if r#"\/:*?"<>|"#.contains(character) {
                '-'
            } else {
                character
            }
        })
        .collect();
    let name = printable.trim();
    if name.is_empty() {
        return i18n::translate(language, keys::notes::kind_tag::DRAWING, &[]);
    }
    name.to_owned()
}

/// The colours of the pictures, from the tokens the stage paints with.
pub(super) fn picture_palette(p: &Palette) -> drawing::Palette {
    drawing::Palette {
        stroke: Hue::ALL.map(|hue| css(stroke(p, hue))),
        fill: Hue::ALL.map(|hue| css(tint(p, hue))),
        paper: css(p.bg_1),
    }
}

fn css(colour: Color32) -> String {
    let [r, g, b, a] = colour.to_srgba_unmultiplied();
    if a == u8::MAX {
        return format!("#{r:02x}{g:02x}{b:02x}");
    }
    format!("rgba({r}, {g}, {b}, {:.2})", f32::from(a) / 255.0)
}

fn stroke(p: &Palette, hue: Hue) -> Color32 {
    match hue {
        Hue::Ink => p.text_0,
        Hue::Blue => p.blue,
        Hue::Green => p.green,
        Hue::Amber => p.amber_fill,
        Hue::Red => p.red,
        Hue::Purple => p.purple,
    }
}

fn tint(p: &Palette, hue: Hue) -> Color32 {
    let (colour, alpha) = match hue {
        Hue::Ink => (p.text_1, INK_TINT),
        _ => (stroke(p, hue), HUE_TINT),
    };
    let [r, g, b, _] = colour.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// What came of a frame on the drawing.
#[derive(Default)]
pub(super) struct Shown {
    /// The pause is over, or the drawing was left: the scene is written.
    pub(super) save: bool,
    /// The drawing was left: its pictures go with it.
    pub(super) left: bool,
    pub(super) export: Option<Format>,
}

pub(super) fn show(ui: &mut Ui, view: &mut DrawingView, title: &str, language: Language) -> Shown {
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let export = toolbar(ui, view, language);
    let stage = stage(ui, view, title, language);
    keys_of(ui, view, &stage);
    pointer(ui, view, &stage);
    paint(ui, view, stage.rect, language);
    typing(ui, view, stage.rect);

    let holds = has_keys(ui, &stage) || view.typing.is_some();
    let left = view.held_keys && !holds;
    view.held_keys = holds;
    Shown {
        save: pause_over(ui, view) || left,
        left,
        export,
    }
}

fn pause_over(ui: &Ui, view: &mut DrawingView) -> bool {
    let now = ui.input(|input| input.time);
    if view.editor.take_unsaved() {
        view.touched = true;
        view.unpictured = true;
        view.save_at = Some(now + SAVE_AFTER);
    }
    let Some(due) = view.save_at else {
        return false;
    };
    if now < due {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(due - now));
        return false;
    }
    view.save_at = None;
    true
}

fn toolbar(ui: &mut Ui, view: &mut DrawingView, language: Language) -> Option<Format> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut export = None;
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tool, icon, key, name, _) in TOOLS {
            let id = format!("drawing-tool-{name}");
            if IconButton::new(icon, &tr(key), &id)
                .pressed(view.editor.tool() == tool)
                .show(ui)
                .clicked()
            {
                view.editor.set_tool(tool);
            }
        }
        ui.add_space(14.0);
        for hue in Hue::ALL {
            swatch(ui, view, hue, language);
        }
        ui.add_space(14.0);
        ui.add_enabled_ui(view.editor.can_undo(), |ui| {
            if IconButton::new(Icon::Undo, &tr(keys::drawing::UNDO), "drawing-undo")
                .show(ui)
                .clicked()
            {
                view.editor.undo();
            }
        });
        ui.add_enabled_ui(view.editor.can_redo(), |ui| {
            if IconButton::new(Icon::Redo, &tr(keys::drawing::REDO), "drawing-redo")
                .show(ui)
                .clicked()
            {
                view.editor.redo();
            }
        });
        if IconButton::new(Icon::Trash, &tr(keys::drawing::REMOVE), "drawing-delete")
            .show(ui)
            .clicked()
        {
            view.editor.remove_selection();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let drawn = view.editor.shapes() > 0;
            for (format, key, id) in EXPORTS {
                if Button::new(&tr(key), id).enabled(drawn).show(ui).clicked() {
                    export = Some(format);
                }
            }
        });
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        row.response.rect.bottom() + 8.0,
        Stroke::new(1.0, p.line_soft),
    );
    ui.add_space(8.0);
    export
}

fn swatch(ui: &mut Ui, view: &mut DrawingView, hue: Hue, language: Language) {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let on = view.editor.hue() == hue;
    let name = i18n::translate(language, colour_key(hue), &[]);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::RadioButton, true, on, name.clone())
    });
    widgets::address(ui, &response, &format!("drawing-hue-{}", hue.name()));
    let painter = ui.painter();
    painter.circle_filled(rect.center(), 8.0, stroke(p, hue));
    if on {
        painter.circle_stroke(rect.center(), 11.0, Stroke::new(1.5, p.amber_edge));
    }
    if response.on_hover_text(&name).clicked() {
        view.editor.set_hue(hue);
    }
}

fn colour_key(hue: Hue) -> i18n::Key {
    match hue {
        Hue::Ink => keys::drawing::colour::INK,
        Hue::Blue => keys::drawing::colour::BLUE,
        Hue::Green => keys::drawing::colour::GREEN,
        Hue::Amber => keys::drawing::colour::AMBER,
        Hue::Red => keys::drawing::colour::RED,
        Hue::Purple => keys::drawing::colour::PURPLE,
    }
}

/// The stage takes the keys, and says how many shapes it holds.
fn stage(ui: &mut Ui, view: &DrawingView, title: &str, language: Language) -> Response {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let label = i18n::translate(language, keys::drawing::LABEL, &[("title", title.into())]);
    let shapes = view.editor.shapes().to_string();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label(label.clone());
        node.set_value(shapes.clone());
    });
    widgets::address(ui, &response, "drawing-stage");
    if view.editor.tool() != Tool::Select && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }
    debug_assert!(rect.is_finite());
    response
}

fn scene_at(stage: Rect, at: Pos2) -> Point {
    Point {
        x: f64::from(at.x - stage.left()),
        y: f64::from(at.y - stage.top()),
    }
}

fn screen_at(stage: Rect, at: Point) -> Pos2 {
    #[expect(clippy::cast_possible_truncation, reason = "a point on the screen")]
    Pos2::new(stage.left() + at.x as f32, stage.top() + at.y as f32)
}

fn pointer(ui: &Ui, view: &mut DrawingView, stage: &Response) {
    let (pressed, released, at, shift) = ui.input(|input| {
        (
            input.pointer.primary_pressed(),
            input.pointer.primary_released(),
            input.pointer.latest_pos(),
            input.modifiers.shift,
        )
    });
    let Some(at) = at else { return };
    let scene = scene_at(stage.rect, at);
    if pressed && stage.hovered() {
        leave_text(view);
        stage.request_focus();
        view.editor.press(scene, shift);
    } else if released {
        view.editor.release(scene);
    } else if stage.is_pointer_button_down_on() {
        view.editor.drag(scene);
    }
}

/// ⚠️ Not `Response::has_focus`, which asks the window's focus too: that one is for style.
fn has_keys(ui: &Ui, stage: &Response) -> bool {
    ui.memory(|memory| memory.has_focus(stage.id))
}

/// The tools by their letter, the history by its keys; Escape first drops the tool, then the
/// selection, and only then leaves, to the modal frame.
fn keys_of(ui: &Ui, view: &mut DrawingView, stage: &Response) {
    if view.typing.is_some() || !(has_keys(ui, stage) || view.held_keys) {
        return;
    }
    let pressed = |modifiers, key| {
        ui.ctx()
            .input_mut(|input| input.consume_key(modifiers, key))
    };
    if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || pressed(Modifiers::COMMAND, Key::Y)
    {
        view.editor.redo();
    } else if pressed(Modifiers::COMMAND, Key::Z) {
        view.editor.undo();
    }
    if (pressed(Modifiers::NONE, Key::Delete) || pressed(Modifiers::NONE, Key::Backspace))
        && !view.editor.remove_selection()
    {
        return;
    }
    let escaped = ui.input(|input| input.key_pressed(Key::Escape));
    if escaped {
        let dropped = if view.editor.tool() == Tool::Select {
            view.editor.deselect()
        } else {
            view.editor.back_to_selection();
            true
        };
        if dropped {
            pressed(Modifiers::NONE, Key::Escape);
            stage.request_focus();
        }
        return;
    }
    for (tool, _, _, _, key) in TOOLS {
        if pressed(Modifiers::NONE, key) {
            view.editor.set_tool(tool);
        }
    }
}

fn paint(ui: &Ui, view: &mut DrawingView, stage: Rect, language: Language) {
    let p = palette(ui.ctx());
    let editing = view.editor.editing().map(|(index, _)| index);
    let revision = view.editor.revision();
    if view.drawn != Some(revision) {
        view.marks = drawing::marks(view.editor.drawing(), view.editor.fonts(), editing);
        view.drawn = Some(revision);
    }
    let painter = ui.painter_at(stage);
    if view.editor.shapes() == 0 && view.editor.pen_line().is_none() {
        let words = i18n::translate(language, keys::drawing::EMPTY, &[]);
        let galley = widgets::text(
            ui,
            &words,
            13.0,
            400.0,
            egui::FontFamily::Proportional,
            p.text_2,
        );
        let at = stage.center() - galley.size() / 2.0;
        painter.galley(at, galley, p.text_2);
    }
    for mark in &view.marks {
        paint_mark(ui, &painter, p, stage, mark);
    }
    if let Some(points) = view.editor.pen_line() {
        let line: Vec<Pos2> = points.iter().map(|at| screen_at(stage, *at)).collect();
        painter.add(Shape::line(
            line,
            Stroke::new(2.5, stroke(p, view.editor.hue())),
        ));
    }
    if let Some(frame) = view.editor.frame() {
        paint_frame(&painter, p, stage, &frame);
    }
    if let Some(marquee) = view.editor.marquee() {
        let min = screen_at(
            stage,
            Point {
                x: marquee.left,
                y: marquee.top,
            },
        );
        #[expect(clippy::cast_possible_truncation, reason = "a size on the screen")]
        let size = Vec2::new(marquee.width as f32, marquee.height as f32);
        painter.rect(
            Rect::from_min_size(min, size),
            CornerRadius::ZERO,
            p.amber_edge.gamma_multiply(0.08),
            Stroke::new(1.0, p.amber_edge),
            StrokeKind::Inside,
        );
    }
}

fn paint_mark(ui: &Ui, painter: &egui::Painter, p: &Palette, stage: Rect, mark: &Mark) {
    match mark {
        Mark::Line {
            points,
            closed,
            fill,
            hue,
            width,
        } => {
            let colour = stroke(p, *hue);
            let line: Vec<Pos2> = points.iter().map(|at| screen_at(stage, *at)).collect();
            #[expect(clippy::cast_possible_truncation, reason = "a stroke on the screen")]
            let edge = PathStroke::new(*width as f32, colour);
            let inside = match fill {
                Fill::None => Color32::TRANSPARENT,
                Fill::Tint => tint(p, *hue),
                Fill::Stroke => colour,
            };
            painter.add(if *closed {
                PathShape::convex_polygon(line, inside, edge)
            } else {
                PathShape::line(line, edge)
            });
        }
        Mark::Words {
            text,
            at,
            angle,
            size,
            hue,
        } => {
            let colour = stroke(p, *hue);
            #[expect(clippy::cast_possible_truncation, reason = "sizes on the screen")]
            let (size, turned) = (*size as f32, angle.to_radians() as f32);
            let galley = widgets::text(
                ui,
                text,
                size,
                400.0,
                egui::FontFamily::Proportional,
                colour,
            );
            let above = egui::emath::Rot2::from_angle(turned) * Vec2::new(0.0, -ASCENT * size);
            let shape = TextShape::new(screen_at(stage, *at) + above, galley, colour);
            painter.add(shape.with_angle(turned));
        }
    }
}

fn paint_frame(painter: &egui::Painter, p: &Palette, stage: Rect, frame: &Frame) {
    let edge = Stroke::new(1.0, p.amber_edge);
    let corners: Vec<Pos2> = frame
        .corners()
        .iter()
        .map(|at| screen_at(stage, *at))
        .collect();
    painter.add(Shape::closed_line(corners, edge));
    painter.line_segment(
        [
            screen_at(stage, frame.grip(Grip::Top)),
            screen_at(stage, frame.grip(Grip::Rotate)),
        ],
        edge,
    );
    for grip in Grip::ALL {
        let centre = screen_at(stage, frame.grip(grip));
        painter.rect(
            Rect::from_center_size(centre, Vec2::splat(GRIP_SIZE)),
            CornerRadius::ZERO,
            p.amber_edge,
            Stroke::new(1.0, p.bg_1),
            StrokeKind::Inside,
        );
    }
}

/// The field over a text being typed into, where the text sits.
fn typing(ui: &mut Ui, view: &mut DrawingView, stage: Rect) {
    let Some((_, object)) = view.editor.editing() else {
        view.typing = None;
        return;
    };
    let p = palette(ui.ctx());
    let bounds = object.bounds();
    #[expect(clippy::cast_possible_truncation, reason = "sizes on the screen")]
    let (size, font_size) = (
        Vec2::new(bounds.width as f32, bounds.height as f32),
        object.font_size() as f32,
    );
    let colour = stroke(p, object.hue());
    let at = screen_at(
        stage,
        Point {
            x: bounds.left,
            y: bounds.top,
        },
    );
    let mut field = view.typing.take().unwrap_or_else(|| Typing {
        words: object.text().unwrap_or_default().to_owned(),
        settling: true,
    });
    let response = ui.put(
        Rect::from_min_size(at, size),
        egui::TextEdit::multiline(&mut field.words)
            .font(egui::FontId::new(font_size, egui::FontFamily::Proportional))
            .frame(egui::Frame::new())
            .margin(egui::Margin::ZERO)
            .text_color(colour)
            .desired_rows(1)
            .desired_width(size.x),
    );
    widgets::address(ui, &response, "drawing-text");
    if field.settling {
        response.request_focus();
        field.settling = ui.input(|input| input.pointer.any_down() || input.pointer.any_released());
    }
    if response.changed() {
        view.editor.set_text(&field.words);
    }
    let left = response.lost_focus() && !field.settling;
    view.typing = Some(field);
    if left {
        leave_text(view);
    }
}

fn leave_text(view: &mut DrawingView) {
    if view.typing.take().is_some() {
        view.editor.leave_text();
    }
}

#[cfg(test)]
mod tests;
