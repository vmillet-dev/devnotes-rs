//! The drawing being edited (the reference's `drawing-scene.ts`, Fabric's canvas included): the
//! tools, what is selected, the gestures on it, the arrows bound to the shapes they start and end
//! on, and the history. A widget hands it the pointer in the scene's units and draws what it
//! answers.

use std::sync::Arc;

use uuid::Uuid;

use crate::geometry::{
    Box, Ends, Point, arrow_path, bound_ends, diamond_points, drag_box, smooth_path,
};
use crate::scene::{Drawing, Hue, Kind, Object};
use crate::text::{Fonts, text_height, wrap};

/// A shape let go smaller than this was a click: it takes the size a click gives.
const CLICKED: f64 = 6.0;
const CLICK_SIZE: (f64, f64) = (140.0, 80.0);
const HISTORY: usize = 60;
const TEXT_WIDTH: f64 = 180.0;
const TEXT_MIN_WIDTH: f64 = 20.0;
const TEXT_SIZE: f64 = 18.0;
const TEXT_FONT: &str = "Inter Variable, sans-serif";
const PEN_WIDTH: f64 = 2.5;
const SHAPE_STROKE: f64 = 2.0;
const CORNER_RADIUS: f64 = 6.0;
/// Fabric's rotation handle, above the top side; how near a handle the pointer takes it.
const ROTATE_REACH: f64 = 40.0;
const GRIP_REACH: f64 = 8.0;
/// An object scaled to nothing could not be taken again.
const LEAST_SCALE: f64 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Rectangle,
    Ellipse,
    Diamond,
    Arrow,
    Pen,
    Text,
}

/// A handle of the selection. A corner scales both ways, a side one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grip {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Rotate,
}

impl Grip {
    pub const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Right,
        Self::BottomRight,
        Self::Bottom,
        Self::BottomLeft,
        Self::Left,
        Self::Rotate,
    ];

    /// Where it sits on the frame, from -1 to 1 on each side of its centre.
    fn place(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (-1.0, -1.0),
            Self::Top | Self::Rotate => (0.0, -1.0),
            Self::TopRight => (1.0, -1.0),
            Self::Right => (1.0, 0.0),
            Self::BottomRight => (1.0, 1.0),
            Self::Bottom => (0.0, 1.0),
            Self::BottomLeft => (-1.0, 1.0),
            Self::Left => (-1.0, 0.0),
        }
    }
}

/// What the selection's handles sit on: one object's turned box, or the box around several.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub centre: Point,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
}

impl Frame {
    /// A point of the frame's own plane, in the scene's.
    fn on_scene(&self, x: f64, y: f64) -> Point {
        let (sin, cos) = self.angle.to_radians().sin_cos();
        Point {
            x: self.centre.x + x * cos - y * sin,
            y: self.centre.y + x * sin + y * cos,
        }
    }

    fn on_frame(&self, at: Point) -> (f64, f64) {
        let (sin, cos) = (-self.angle).to_radians().sin_cos();
        let (x, y) = (at.x - self.centre.x, at.y - self.centre.y);
        (x * cos - y * sin, x * sin + y * cos)
    }

    pub fn grip(&self, grip: Grip) -> Point {
        let (x, y) = grip.place();
        let reach = if grip == Grip::Rotate {
            ROTATE_REACH
        } else {
            0.0
        };
        self.on_scene(x * self.width / 2.0, y * self.height / 2.0 - reach)
    }

    pub fn corners(&self) -> [Point; 4] {
        [
            Grip::TopLeft,
            Grip::TopRight,
            Grip::BottomRight,
            Grip::BottomLeft,
        ]
        .map(|corner| self.grip(corner))
    }
}

/// What a press started, until the release.
#[derive(Debug, Clone)]
enum Gesture {
    Shape {
        tool: Tool,
        from: Point,
    },
    Arrow {
        from: Point,
        from_id: Option<String>,
    },
    Pen(Vec<Point>),
    Moving {
        from: Point,
        before: Selected,
        reselected: bool,
    },
    Scaling {
        grip: Grip,
        frame: Frame,
        before: Selected,
    },
    Rotating {
        frame: Frame,
        from: f64,
        before: Selected,
    },
    Marquee {
        from: Point,
        to: Point,
    },
}

/// The objects a gesture started on, as they were: each step is read from them, so nothing
/// drifts.
type Selected = Vec<(usize, Object)>;

pub struct Editor {
    drawing: Drawing,
    fonts: Arc<Fonts>,
    tool: Tool,
    hue: Hue,
    selected: Vec<usize>,
    /// What was selected when a tool was taken: Escape gives it back.
    kept: Vec<usize>,
    gesture: Option<Gesture>,
    /// The text being typed into.
    editing: Option<usize>,
    history: Vec<String>,
    at: usize,
    /// Bumped by every change on screen: what is drawn from the scene is kept until it moves.
    revision: u64,
    /// Something the history kept since the body was last asked for.
    unsaved: bool,
}

impl Editor {
    /// A body that holds no scene opens as a new drawing.
    pub fn open(content: &str, fonts: Arc<Fonts>) -> Self {
        let drawing = Drawing::read(content).unwrap_or_else(Drawing::empty);
        let history = vec![drawing.write()];
        Self {
            drawing,
            fonts,
            tool: Tool::Select,
            hue: Hue::Ink,
            selected: Vec::new(),
            kept: Vec::new(),
            gesture: None,
            editing: None,
            history,
            at: 0,
            revision: 0,
            unsaved: false,
        }
    }

    pub fn drawing(&self) -> &Drawing {
        &self.drawing
    }

    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn hue(&self) -> Hue {
        self.hue
    }

    pub fn shapes(&self) -> usize {
        self.drawing.objects.len()
    }

    pub fn selected(&self) -> &[usize] {
        &self.selected
    }

    pub fn can_undo(&self) -> bool {
        self.at > 0
    }

    pub fn can_redo(&self) -> bool {
        self.at + 1 < self.history.len()
    }

    /// The body, as stored.
    pub fn body(&self) -> String {
        self.drawing.write()
    }

    /// `true` once: something changed since the last time it was asked.
    pub fn take_unsaved(&mut self) -> bool {
        std::mem::take(&mut self.unsaved)
    }

    /// Taking a tool drops the selection; Escape puts it back on the way to leaving.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool == Tool::Select && tool != Tool::Select {
            self.kept = std::mem::take(&mut self.selected);
        }
        self.tool = tool;
    }

    /// For what is drawn next, and for what is selected now.
    pub fn set_hue(&mut self, hue: Hue) {
        self.hue = hue;
        if self.selected.is_empty() {
            return;
        }
        for &index in &self.selected {
            self.drawing.objects[index].set_hue(hue);
        }
        self.keep();
    }

    /// What the arrows bound to it hold on to is let go: they stay where they were.
    pub fn remove_selection(&mut self) -> bool {
        if self.selected.is_empty() || self.editing.is_some() {
            return false;
        }
        let mut gone: Vec<usize> = std::mem::take(&mut self.selected);
        gone.sort_unstable();
        let ids: Vec<Option<String>> = gone
            .iter()
            .map(|&index| self.drawing.objects[index].id().map(str::to_owned))
            .collect();
        for &index in gone.iter().rev() {
            self.drawing.objects.remove(index);
        }
        let held = |id: Option<&str>| id.is_some() && ids.iter().any(|gone| gone.as_deref() == id);
        for object in &mut self.drawing.objects {
            if held(object.from()) {
                object.set_bound("from", None);
            }
            if held(object.to()) {
                object.set_bound("to", None);
            }
        }
        self.keep();
        true
    }

    /// Escape on a tool: back to the selection, as it was before the tool was taken.
    pub fn back_to_selection(&mut self) {
        self.tool = Tool::Select;
        self.selected = std::mem::take(&mut self.kept);
        self.revision += 1;
    }

    /// `false` when nothing was selected, which leaves Escape to whatever holds the drawing.
    pub fn deselect(&mut self) -> bool {
        let had = !self.selected.is_empty();
        self.selected.clear();
        had
    }

    pub fn undo(&mut self) {
        if self.can_undo() {
            self.at -= 1;
            self.restore();
        }
    }

    pub fn redo(&mut self) {
        if self.can_redo() {
            self.at += 1;
            self.restore();
        }
    }

    fn restore(&mut self) {
        if let Some(drawing) = Drawing::read(&self.history[self.at]) {
            self.drawing = drawing;
        }
        self.selected.clear();
        self.kept.clear();
        self.editing = None;
        self.revision += 1;
        self.unsaved = true;
    }

    /// One state of the history, after a change: the scene as it is saved.
    fn keep(&mut self) {
        self.revision += 1;
        let snapshot = self.drawing.write();
        if snapshot == self.history[self.at] {
            return;
        }
        self.history.truncate(self.at + 1);
        self.history.push(snapshot);
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        self.at = self.history.len() - 1;
        self.unsaved = true;
    }

    /// The handles' frame; `None` with nothing selected, or a text typed into.
    pub fn frame(&self) -> Option<Frame> {
        if self.editing.is_some() {
            return None;
        }
        if let [index] = self.selected[..] {
            let object = &self.drawing.objects[index];
            let (width, height) = object.dimensions();
            return Some(Frame {
                centre: object.centre(),
                width,
                height,
                angle: object.angle(),
            });
        }
        let boxes: Vec<Box> = self
            .selected
            .iter()
            .map(|&index| self.drawing.objects[index].bounds())
            .collect();
        let around = crate::geometry::scene_box(&boxes, 0.0)?;
        Some(Frame {
            centre: around.centre(),
            width: around.width,
            height: around.height,
            angle: 0.0,
        })
    }

    /// The rectangle a drag on the paper is selecting with.
    pub fn marquee(&self) -> Option<Box> {
        match self.gesture {
            Some(Gesture::Marquee { from, to }) => Some(drag_box(from, to)),
            _ => None,
        }
    }

    /// The pen's line, as it is drawn: it is an object only once let go.
    pub fn pen_line(&self) -> Option<&[Point]> {
        match &self.gesture {
            Some(Gesture::Pen(points)) => Some(points),
            _ => None,
        }
    }

    fn grip_at(&self, at: Point) -> Option<Grip> {
        let frame = self.frame()?;
        Grip::ALL.into_iter().find(|&grip| {
            let handle = frame.grip(grip);
            (handle.x - at.x).abs() <= GRIP_REACH && (handle.y - at.y).abs() <= GRIP_REACH
        })
    }

    /// The topmost object under a point.
    fn object_at(&self, at: Point) -> Option<usize> {
        self.drawing
            .objects
            .iter()
            .rposition(|object| object.contains(at))
    }

    /// A pointer pressed. `adding` is Shift: an object joins the selection or leaves it.
    pub fn press(&mut self, at: Point, adding: bool) {
        match self.tool {
            Tool::Select => self.press_selecting(at, adding),
            Tool::Pen => self.gesture = Some(Gesture::Pen(vec![at])),
            Tool::Text => self.make_text(at),
            Tool::Arrow => {
                let from_id = self.shape_at(at);
                self.gesture = Some(Gesture::Arrow {
                    from: at,
                    from_id: from_id.clone(),
                });
                let mut arrow = self.new_object("Path");
                arrow.put_number("strokeWidth", SHAPE_STROKE);
                arrow.set_bound("from", from_id.as_deref());
                arrow.set_bound("to", None);
                set_arrow(&mut arrow, [at.x, at.y, at.x, at.y]);
                self.drawing.objects.push(arrow);
                self.revision += 1;
            }
            Tool::Rectangle | Tool::Ellipse | Tool::Diamond => {
                let tool = self.tool;
                self.gesture = Some(Gesture::Shape { tool, from: at });
                let shape = self.new_shape(tool, drag_box(at, at));
                self.drawing.objects.push(shape);
                self.revision += 1;
            }
        }
    }

    fn press_selecting(&mut self, at: Point, adding: bool) {
        if let Some(grip) = self.grip_at(at).filter(|_| !adding)
            && let Some(frame) = self.frame()
        {
            let before = self.selection();
            self.gesture = Some(if grip == Grip::Rotate {
                Gesture::Rotating {
                    frame,
                    from: bearing(frame.centre, at),
                    before,
                }
            } else {
                Gesture::Scaling {
                    grip,
                    frame,
                    before,
                }
            });
            return;
        }
        let Some(index) = self.object_at(at) else {
            if !adding {
                self.selected.clear();
            }
            self.gesture = Some(Gesture::Marquee { from: at, to: at });
            return;
        };
        let reselected = self.selected == [index];
        if adding {
            match self.selected.iter().position(|&held| held == index) {
                Some(place) => {
                    self.selected.remove(place);
                }
                None => self.selected.push(index),
            }
        } else if !self.selected.contains(&index) {
            self.selected = vec![index];
        }
        let before = self.selection();
        self.gesture = Some(Gesture::Moving {
            from: at,
            before,
            reselected,
        });
    }

    fn selection(&self) -> Selected {
        self.selected
            .iter()
            .map(|&index| (index, self.drawing.objects[index].clone()))
            .collect()
    }

    /// The pointer moved while pressed.
    pub fn drag(&mut self, to: Point) {
        let Some(mut gesture) = self.gesture.take() else {
            return;
        };
        match &mut gesture {
            Gesture::Shape { tool, from } => self.reshape_last(*tool, drag_box(*from, to)),
            Gesture::Arrow { from, .. } => {
                if let Some(arrow) = self.drawing.objects.last_mut() {
                    set_arrow(arrow, [from.x, from.y, to.x, to.y]);
                }
            }
            Gesture::Pen(points) => points.push(to),
            Gesture::Moving { from, before, .. } => {
                if *from != to {
                    move_to(self, before, *from, to);
                }
            }
            Gesture::Scaling {
                grip,
                frame,
                before,
            } => scale_to(self, before, *grip, *frame, to),
            Gesture::Rotating {
                frame,
                from,
                before,
            } => {
                rotate_to(self, before, *frame, bearing(frame.centre, to) - *from);
            }
            Gesture::Marquee { to: end, .. } => *end = to,
        }
        self.gesture = Some(gesture);
        self.revision += 1;
    }

    /// The pointer let go.
    pub fn release(&mut self, at: Point) {
        match self.gesture.take() {
            None => {}
            Some(Gesture::Shape { tool, from }) => {
                let dragged = drag_box(from, at);
                let clicked = dragged.width < CLICKED && dragged.height < CLICKED;
                let (width, height) = CLICK_SIZE;
                let bounds = if clicked {
                    Box {
                        left: from.x,
                        top: from.y,
                        width,
                        height,
                    }
                } else {
                    dragged
                };
                self.reshape_last(tool, bounds);
                self.finish();
            }
            Some(Gesture::Arrow { from, from_id }) => {
                // The arrow being drawn is under the pointer: it is not what it ends on.
                let Some(mut arrow) = self.drawing.objects.pop() else {
                    return;
                };
                let to_id = self.shape_at(at).filter(|id| Some(id) != from_id.as_ref());
                arrow.set_bound("to", to_id.as_deref());
                self.drawing.objects.push(arrow);
                let last = self.drawing.objects.len() - 1;
                self.bind(last, [from.x, from.y, at.x, at.y]);
                self.finish();
            }
            Some(Gesture::Pen(points)) => self.adopt_pen(&points),
            Some(Gesture::Marquee { from, to }) => self.select_within(drag_box(from, to)),
            Some(Gesture::Moving {
                from, reselected, ..
            }) => {
                if from != at {
                    self.settle();
                } else if reselected {
                    self.begin_text();
                }
            }
            Some(Gesture::Scaling { .. } | Gesture::Rotating { .. }) => self.settle(),
        }
    }

    /// Double-click on a text, or a click on a text already selected.
    fn begin_text(&mut self) {
        if let [index] = self.selected[..]
            && self.drawing.objects[index].kind() == Kind::Textbox
        {
            self.editing = Some(index);
            self.revision += 1;
        }
    }

    /// The text typed into and what it holds.
    pub fn editing(&self) -> Option<(usize, &Object)> {
        let index = self.editing?;
        Some((index, &self.drawing.objects[index]))
    }

    pub fn set_text(&mut self, text: &str) {
        let Some(index) = self.editing else { return };
        let object = &mut self.drawing.objects[index];
        object.put("text", text);
        let lines = wrap(text, object.width(), object.font_size(), &self.fonts);
        let height = text_height(lines.len(), object.font_size(), object.line_height());
        object.set_size(object.width(), height);
        self.revision += 1;
    }

    /// A text left empty was a click: it goes, and the history never knew it.
    pub fn leave_text(&mut self) {
        let Some(index) = self.editing.take() else {
            return;
        };
        if self.drawing.objects[index]
            .text()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            self.drawing.objects.remove(index);
            self.selected.clear();
            self.revision += 1;
            return;
        }
        self.keep();
    }

    fn new_object(&self, kind: &str) -> Object {
        Object::make(kind, Uuid::new_v4().to_string(), self.hue)
    }

    fn new_shape(&self, tool: Tool, bounds: Box) -> Object {
        let kind = match tool {
            Tool::Ellipse => "Ellipse",
            Tool::Diamond => "Polygon",
            _ => "Rect",
        };
        let mut shape = self.new_object(kind);
        shape.put_number("strokeWidth", SHAPE_STROKE);
        shape.put("strokeUniform", true);
        if tool == Tool::Rectangle {
            shape.put_number("rx", CORNER_RADIUS);
            shape.put_number("ry", CORNER_RADIUS);
        }
        reshape(&mut shape, tool, bounds);
        shape
    }

    fn reshape_last(&mut self, tool: Tool, bounds: Box) {
        if let Some(shape) = self.drawing.objects.last_mut() {
            reshape(shape, tool, bounds);
        }
    }

    /// Its left edge where the click was: Fabric places it by its centre.
    fn make_text(&mut self, at: Point) {
        let mut text = self.new_object("Textbox");
        text.put("text", "");
        text.put_number("fontSize", TEXT_SIZE);
        text.put("fontFamily", TEXT_FONT);
        text.put_number("strokeWidth", 0.0);
        text.set_size(TEXT_WIDTH, text_height(1, TEXT_SIZE, text.line_height()));
        text.set_centre(Point {
            x: at.x + TEXT_WIDTH / 2.0,
            y: at.y,
        });
        self.drawing.objects.push(text);
        self.tool = Tool::Select;
        self.kept.clear();
        let index = self.drawing.objects.len() - 1;
        self.selected = vec![index];
        self.editing = Some(index);
        self.revision += 1;
    }

    fn adopt_pen(&mut self, points: &[Point]) {
        let Some(path) = smooth_path(points) else {
            self.revision += 1;
            return;
        };
        let mut line = self.new_object("Path");
        line.put_number("strokeWidth", PEN_WIDTH);
        line.put("strokeLineCap", "round");
        line.put("strokeLineJoin", "round");
        line.put("fill", serde_json::Value::Null);
        line.set_path(&path);
        self.drawing.objects.push(line);
        self.keep();
    }

    /// Back to the selection, the new object selected.
    fn finish(&mut self) {
        self.kept.clear();
        self.tool = Tool::Select;
        self.selected = vec![self.drawing.objects.len() - 1];
        self.keep();
    }

    fn select_within(&mut self, marquee: Box) {
        let touched = self
            .drawing
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| {
                let bounds = object.bounds();
                bounds.left < marquee.left + marquee.width
                    && marquee.left < bounds.left + bounds.width
                    && bounds.top < marquee.top + marquee.height
                    && marquee.top < bounds.top + bounds.height
            });
        let touched: Vec<usize> = touched.map(|(index, _)| index).collect();
        if marquee.width > 0.0 || marquee.height > 0.0 {
            for index in touched {
                if !self.selected.contains(&index) {
                    self.selected.push(index);
                }
            }
        }
        self.revision += 1;
    }

    /// Shapes hold arrows; a text or another arrow does not.
    fn shape_at(&self, at: Point) -> Option<String> {
        let object = &self.drawing.objects[self.object_at(at)?];
        let holds = object.ends().is_none() && object.kind() != Kind::Textbox;
        object.id().filter(|_| holds).map(str::to_owned)
    }

    fn box_of(&self, id: Option<&str>) -> Option<Box> {
        let id = id?;
        let object = self
            .drawing
            .objects
            .iter()
            .find(|object| object.id() == Some(id))?;
        Some(object.bounds())
    }

    /// Its ends on the edges of the shapes it is bound to, or where they were drawn.
    fn bind(&mut self, index: usize, drawn: Ends) {
        let arrow = &self.drawing.objects[index];
        let from = self.box_of(arrow.from());
        let to = self.box_of(arrow.to());
        set_arrow(
            &mut self.drawing.objects[index],
            bound_ends(drawn, from.as_ref(), to.as_ref()),
        );
    }

    /// The arrows bound to anything take their place from it.
    fn follow(&mut self) {
        for index in 0..self.drawing.objects.len() {
            let arrow = &self.drawing.objects[index];
            if let Some(ends) = arrow.ends()
                && (arrow.from().is_some() || arrow.to().is_some())
            {
                self.bind(index, ends);
            }
        }
    }

    fn settle(&mut self) {
        self.follow();
        self.keep();
    }
}

fn set_arrow(arrow: &mut Object, ends: Ends) {
    arrow.set_ends(ends);
    arrow.set_path(&arrow_path(ends));
}

/// ⚠️ Fabric 7 places an object by its centre: `left` and `top` are the box's middle.
fn reshape(shape: &mut Object, tool: Tool, bounds: Box) {
    match tool {
        Tool::Ellipse => {
            shape.put_number("rx", bounds.width / 2.0);
            shape.put_number("ry", bounds.height / 2.0);
        }
        Tool::Diamond => {
            let points = diamond_points(bounds.width, bounds.height);
            shape.set_points(&points);
        }
        _ => {}
    }
    shape.set_size(bounds.width, bounds.height);
    shape.set_centre(bounds.centre());
}

/// Where a point lies from a centre, in degrees, up being none.
fn bearing(centre: Point, at: Point) -> f64 {
    (at.x - centre.x).atan2(centre.y - at.y).to_degrees()
}

fn move_to(editor: &mut Editor, before: &Selected, from: Point, to: Point) {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    for (index, object) in before {
        let centre = object.centre();
        let moved = &mut editor.drawing.objects[*index];
        moved.set_centre(Point {
            x: centre.x + dx,
            y: centre.y + dy,
        });
        if let Some([x1, y1, x2, y2]) = object.ends() {
            moved.set_ends([x1 + dx, y1 + dy, x2 + dx, y2 + dy]);
        }
    }
    editor.follow();
}

fn scale_to(editor: &mut Editor, before: &Selected, grip: Grip, frame: Frame, to: Point) {
    let (place_x, place_y) = grip.place();
    let (x, y) = frame.on_frame(to);
    // From the handle's opposite, which stays where it is.
    let anchor = (-place_x * frame.width / 2.0, -place_y * frame.height / 2.0);
    let along = |place: f64, size: f64, at: f64, from: f64| {
        if place == 0.0 {
            1.0
        } else {
            ((at - from) * place / size).max(LEAST_SCALE)
        }
    };
    let mut scale_x = along(place_x, frame.width, x, anchor.0);
    let mut scale_y = along(place_y, frame.height, y, anchor.1);
    // A corner keeps the proportions, as Fabric's `uniformScaling`.
    if place_x != 0.0 && place_y != 0.0 {
        let both = scale_x.max(scale_y);
        (scale_x, scale_y) = (both, both);
    }
    for (index, object) in before {
        let (cx, cy) = frame.on_frame(object.centre());
        let centre = frame.on_scene(
            anchor.0 + (cx - anchor.0) * scale_x,
            anchor.1 + (cy - anchor.1) * scale_y,
        );
        let scaled = &mut editor.drawing.objects[*index];
        scaled.set_centre(centre);
        if object.kind() == Kind::Textbox && place_y == 0.0 {
            let width = (object.width() * scale_x).max(TEXT_MIN_WIDTH);
            let lines = wrap(
                object.text().unwrap_or_default(),
                width,
                object.font_size(),
                &editor.fonts,
            );
            let height = text_height(lines.len(), object.font_size(), object.line_height());
            scaled.set_size(width, height);
        } else {
            scaled.set_scale(object.scale_x() * scale_x, object.scale_y() * scale_y);
        }
    }
    editor.follow();
}

fn rotate_to(editor: &mut Editor, before: &Selected, frame: Frame, degrees: f64) {
    let turned = Frame {
        angle: degrees,
        ..frame
    };
    let still = Frame {
        angle: 0.0,
        ..frame
    };
    for (index, object) in before {
        let (x, y) = still.on_frame(object.centre());
        let rotated = &mut editor.drawing.objects[*index];
        rotated.set_centre(turned.on_scene(x, y));
        rotated.set_angle(object.angle() + degrees);
    }
    editor.follow();
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "the numbers stored, read back")]
mod tests;
