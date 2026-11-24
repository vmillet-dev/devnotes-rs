//! A Dessin's editor (the reference's `drawing-editor`): the toolbar over the stage. `drawing`
//! holds the scene and every gesture; this hands it the pointer and the keys and paints what it
//! answers. The scene is saved once the hand pauses; its pictures go only when it is left, as
//! taking two is dear.

mod camera;
mod look;

use self::camera::Camera;
use crate::theme::tokens::{font, gap};
use drawing::{Attaching, Editor, Fill, Format, Frame, Grip, Hue, Mark, Point, Tool};
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
/// A stroke seen from afar still shows.
const THINNEST: f32 = 1.0;
/// The reference's 0.06 and 0.14, out of 255.
const INK_TINT: u8 = 15;
const HUE_TINT: u8 = 36;
/// How far above a line's baseline its first glyphs start, in font sizes: Inter's ascent.
const ASCENT: f32 = 0.97;

const TOOLS: [(Tool, Icon, i18n::Key, &str, Key); 8] = [
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
    (
        Tool::Eraser,
        Icon::Eraser,
        keys::drawing::ERASER,
        "eraser",
        Key::E,
    ),
    (Tool::Text, Icon::Type, keys::drawing::TEXT, "text", Key::T),
];

/// The field over a text.
struct Typing {
    words: String,
    /// The press that opened it is not over: letting it go is not leaving the field.
    settling: bool,
}

/// Right to left, as the toolbar lays them out.
/// Expanded, the column of tools down the stage's left, and the height it takes: shorter, the
/// tools stay in a row over the stage.
const PALETTE_WIDTH: f32 = 32.0;
const PALETTE_HEIGHT: f32 = 520.0;

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
    camera: Camera,
    look: look::Look,
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
            camera: Camera::default(),
            look: look::Look::default(),
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
pub(crate) fn picture_palette(p: &Palette) -> drawing::Palette {
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

/// Beside the cards, the tools in a row over the stage; `expanded`, in a column down its left
/// where it is tall enough, and the exports in the toolbar's menu.
pub(super) fn show(
    ui: &mut Ui,
    view: &mut DrawingView,
    (title, language): (&str, Language),
    expanded: bool,
) -> Shown {
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    if expanded && ui.max_rect().height() >= PALETTE_HEIGHT {
        let stage = palette_column(ui, view, language);
        let mut stage_ui = ui.new_child(egui::UiBuilder::new().max_rect(stage));
        return on_stage(&mut stage_ui, view, (title, language));
    }
    let export = toolbar(ui, view, language, !expanded);
    Shown {
        export,
        ..on_stage(ui, view, (title, language))
    }
}

fn on_stage(ui: &mut Ui, view: &mut DrawingView, (title, language): (&str, Language)) -> Shown {
    let stage = stage(ui, view, title, language);
    look::open(view, stage.rect);
    keys_of(ui, view, &stage);
    let looking = look::gestures(ui, view, &stage);
    view.editor.set_scale(f64::from(view.camera.zoom));
    if !looking {
        pointer(ui, view, &stage);
    }
    paint(ui, view, stage.rect, language);
    typing(ui, view, stage.rect);
    look::controls(ui, view, stage.rect, language);

    let holds = has_keys(ui, &stage) || view.typing.is_some();
    let left = view.held_keys && !holds;
    view.held_keys = holds;
    Shown {
        save: pause_over(ui, view) || left,
        left,
        export: None,
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

/// The tools in a row; `exports`, the two buttons at its end.
fn toolbar(
    ui: &mut Ui,
    view: &mut DrawingView,
    language: Language,
    exports: bool,
) -> Option<Format> {
    let p = palette(ui.ctx());
    let mut export = None;
    let row = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(gap::TIGHT, gap::SNUG);
        tools(ui, view, language, 14.0);
        if !exports {
            return;
        }
        // At the end of the row, or on the next one in a panel they do not fit in.
        let room = exports_width(ui, language);
        widgets::wrap_before(ui, room);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let drawn = view.editor.shapes() > 0;
            export = ui
                .add_enabled_ui(drawn, |ui| self::exports(ui, language))
                .inner;
        });
    });
    ui.painter().hline(
        ui.max_rect().x_range(),
        row.response.rect.bottom() + 8.0,
        Stroke::new(1.0, p.line_soft),
    );
    ui.add_space(gap::CONTROLS);
    export
}

/// Expanded: the tools down the stage's left. Gives the room left to the stage.
fn palette_column(ui: &mut Ui, view: &mut DrawingView, language: Language) -> Rect {
    let p = palette(ui.ctx());
    let whole = ui.max_rect();
    let column = Rect::from_min_size(whole.min, Vec2::new(PALETTE_WIDTH, whole.height()));
    let mut column_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(column)
            .layout(Layout::top_down(Align::Center)),
    );
    column_ui.spacing_mut().item_spacing.y = gap::TIGHT;
    tools(&mut column_ui, view, language, 10.0);
    let edge = whole.left() + PALETTE_WIDTH + 8.0;
    ui.painter()
        .vline(edge, whole.y_range(), Stroke::new(1.0, p.line_soft));
    ui.advance_cursor_after_rect(whole);
    Rect::from_min_max(egui::pos2(edge + 8.0, whole.top()), whole.max)
}

/// The tools, the colours, then undo, redo and delete, `gap` apart in groups.
fn tools(ui: &mut Ui, view: &mut DrawingView, language: Language, gap: f32) {
    let tr = |key| i18n::translate(language, key, &[]);
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
    ui.add_space(gap);
    for hue in Hue::ALL {
        swatch(ui, view, hue, language);
    }
    ui.add_space(gap);
    widgets::wrap_before(ui, widgets::MIN_TARGET);
    ui.add_enabled_ui(view.editor.can_undo(), |ui| {
        if IconButton::new(Icon::Undo, &tr(keys::drawing::UNDO), "drawing-undo")
            .show(ui)
            .clicked()
        {
            view.editor.undo();
        }
    });
    widgets::wrap_before(ui, widgets::MIN_TARGET);
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
}

/// « Export as SVG », « Export as PNG »: the one clicked.
pub(super) fn exports(ui: &mut Ui, language: Language) -> Option<Format> {
    let mut chosen = None;
    for (format, key, id) in EXPORTS {
        let words = i18n::translate(language, key, &[]);
        if Button::new(&words, id).show(ui).clicked() {
            chosen = Some(format);
        }
    }
    chosen
}

fn exports_width(ui: &Ui, language: Language) -> f32 {
    let widths = EXPORTS.map(|(_, key, _)| {
        let words = i18n::translate(language, key, &[]);
        Button::width(ui, &words) + gap::TIGHT
    });
    widths.iter().sum()
}

fn swatch(ui: &mut Ui, view: &mut DrawingView, hue: Hue, language: Language) {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    crate::widgets::pointer(ui, &response);
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
    let scene = view.camera.scene_at(stage.rect, at);
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
    look::keys_of(ui, view, stage.rect);
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
            font::READING,
            400.0,
            egui::FontFamily::Proportional,
            p.text_2,
        );
        let at = stage.center() - galley.size() / 2.0;
        painter.galley(at, galley, p.text_2);
    }
    for mark in &view.marks {
        paint_mark(ui, &painter, p, (stage, view.camera), mark);
    }
    if let Some(points) = view.editor.pen_line() {
        let at = |at: &Point| view.camera.screen_at(stage, *at);
        let line: Vec<Pos2> = points.iter().map(at).collect();
        let width = view.camera.length(drawing::edit::PEN_WIDTH);
        painter.add(Shape::line(
            line,
            Stroke::new(width, stroke(p, view.editor.hue())),
        ));
    }
    if view.editor.tool() == Tool::Eraser
        && let Some(at) = ui.input(|input| input.pointer.hover_pos())
        && stage.contains(at)
        && let Some(frame) = view.editor.erasable(view.camera.scene_at(stage, at))
    {
        // What a press would take away.
        let corners = frame.corners().map(|at| view.camera.screen_at(stage, at));
        painter.add(Shape::closed_line(
            corners.to_vec(),
            Stroke::new(2.0, p.red),
        ));
    }
    if let Some(at) = ui.input(|input| input.pointer.latest_pos())
        && stage.contains(at)
        && let Some(held) = view.editor.attaching(view.camera.scene_at(stage, at))
    {
        paint_attaching(&painter, p, (stage, view.camera), &held);
    }
    if let Some(ends) = view.editor.arrow_ends() {
        for (_, at) in ends {
            let centre = view.camera.screen_at(stage, at);
            painter.circle(
                centre,
                GRIP_SIZE / 2.0,
                p.amber_edge,
                Stroke::new(1.0, p.bg_1),
            );
        }
    }
    if let Some(frame) = view.editor.frame() {
        paint_frame(&painter, p, (stage, view.camera), &frame);
    }
    if let Some(marquee) = view.editor.marquee() {
        painter.rect(
            view.camera.rect(stage, &marquee),
            CornerRadius::ZERO,
            p.amber_edge.gamma_multiply(0.08),
            Stroke::new(1.0, p.amber_edge),
            StrokeKind::Inside,
        );
    }
}

fn paint_mark(
    ui: &Ui,
    painter: &egui::Painter,
    p: &Palette,
    (stage, camera): (Rect, Camera),
    mark: &Mark,
) {
    match mark {
        Mark::Line {
            points,
            closed,
            fill,
            hue,
            width,
        } => {
            let colour = stroke(p, *hue);
            let at = |at: &Point| camera.screen_at(stage, *at);
            let line: Vec<Pos2> = points.iter().map(at).collect();
            let edge = PathStroke::new(camera.length(*width).max(THINNEST), colour);
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
            #[expect(clippy::cast_possible_truncation, reason = "an angle on the screen")]
            let turned = angle.to_radians() as f32;
            let size = camera.length(*size);
            let galley = widgets::text(
                ui,
                text,
                size,
                400.0,
                egui::FontFamily::Proportional,
                colour,
            );
            let above = egui::emath::Rot2::from_angle(turned) * Vec2::new(0.0, -ASCENT * size);
            let shape = TextShape::new(camera.screen_at(stage, *at) + above, galley, colour);
            painter.add(shape.with_angle(turned));
        }
    }
}

/// What the end of the arrow would hold on to, and the points it can be held by.
fn paint_attaching(
    painter: &egui::Painter,
    p: &Palette,
    (stage, camera): (Rect, Camera),
    held: &Attaching,
) {
    let edge = Stroke::new(2.0, p.amber_edge);
    let corners = held.frame.corners().map(|at| camera.screen_at(stage, at));
    painter.add(Shape::closed_line(corners.to_vec(), edge));
    for point in &held.points {
        let chosen = held.chosen == Some(*point);
        let fill = if chosen { p.amber_edge } else { p.bg_1 };
        painter.circle(
            camera.screen_at(stage, *point),
            GRIP_SIZE / 2.0,
            fill,
            Stroke::new(1.0, p.amber_edge),
        );
    }
}

fn paint_frame(
    painter: &egui::Painter,
    p: &Palette,
    (stage, camera): (Rect, Camera),
    frame: &Frame,
) {
    let edge = Stroke::new(1.0, p.amber_edge);
    let corners: Vec<Pos2> = frame
        .corners()
        .iter()
        .map(|at| camera.screen_at(stage, *at))
        .collect();
    painter.add(Shape::closed_line(corners, edge));
    painter.line_segment(
        [
            camera.screen_at(stage, frame.grip(Grip::Top)),
            camera.screen_at(stage, frame.grip(Grip::Rotate)),
        ],
        edge,
    );
    for grip in Grip::ALL {
        let centre = camera.screen_at(stage, frame.grip(grip));
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
    let place = view.camera.rect(stage, &object.bounds());
    let font_size = view.camera.length(object.font_size());
    let colour = stroke(p, object.hue());
    let mut field = view.typing.take().unwrap_or_else(|| Typing {
        words: object.text().unwrap_or_default().to_owned(),
        settling: true,
    });
    let mut on_stage = ui.new_child(egui::UiBuilder::new().max_rect(place));
    on_stage.set_clip_rect(stage.intersect(ui.clip_rect()));
    let response = on_stage.put(
        place,
        egui::TextEdit::multiline(&mut field.words)
            .font(egui::FontId::new(font_size, egui::FontFamily::Proportional))
            .frame(egui::Frame::new())
            .margin(egui::Margin::ZERO)
            .text_color(colour)
            .desired_rows(1)
            .desired_width(place.width()),
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
