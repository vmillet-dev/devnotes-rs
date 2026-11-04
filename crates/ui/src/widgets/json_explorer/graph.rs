//! The document as nodes on a ground (the reference's `json-graph`), drawn in the layout's own
//! units: a column is a character of the monospace font, a line 24 px. A gesture that starts on
//! a node moves it, anywhere else it pans; the wheel pans, the zoom is three buttons. The
//! keyboard twin is a cursor, the ground the one tab stop: the arrows walk a node's rows, into a
//! child and back up, and Alt with an arrow moves the node.

use egui::epaint::CubicBezierShape;
use egui::{
    Color32, CornerRadius, CursorIcon, FontFamily, Key, Modifiers, Pos2, Rect, Response, Role,
    Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2, vec2,
};
use engine::json::model::{JsonGraph, JsonKind, JsonNode, JsonView};
use i18n::{Language, keys};

use super::reads::{child, find, lineage, parent};
use super::{Explorer, TOOL_HEIGHT, kind_colour};
use crate::theme::palette;
use crate::widgets::{self, address, text};

/// One line of a node; a column is the monospace font's `0` at `FONT`.
const ROW_PX: f32 = 24.0;
const FONT: f32 = 11.5;
const SCALES: [f32; 9] = [0.25, 0.4, 0.5, 0.67, 0.8, 1.0, 1.25, 1.5, 2.0];
/// Room for the tools drawn over the ground's top-left corner.
const START: Vec2 = vec2(16.0, 60.0);
/// One press of Alt with an arrow: two columns across, one line down.
const NUDGE: Vec2 = vec2(2.0, 1.0);
const REVEAL_MARGIN: f32 = 24.0;

/// Where the cursor is: a header is `row: None`. Kept by path, an answer renumbers the nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cursor {
    node: String,
    row: Option<usize>,
}

/// What the ground keeps between frames.
#[derive(Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four asks of the next frame, each on its own"
)]
pub(super) struct Ground {
    /// `None` until moved: the start, which the size of the ground does not change.
    pan: Option<Vec2>,
    /// `None` is 100 %.
    zoom: Option<f32>,
    cursor: Option<Cursor>,
    /// The node under a drag and where it is drawn, committed on release.
    dragged: Option<(String, Vec2)>,
    /// The cursor is brought into view at the next frame.
    pub(super) reveal: bool,
    /// The ground takes the keys at the next frame: the answer redrew what had them.
    pub(super) refocus: bool,
    /// Whether the ground had the keys at the last frame: egui lets go on Escape before it is read.
    focused: bool,
    fit: bool,
}

impl Ground {
    fn zoom(&self) -> f32 {
        self.zoom.unwrap_or(1.0)
    }

    fn pan(&self) -> Vec2 {
        self.pan.unwrap_or(START)
    }

    pub(super) fn fit(&mut self) {
        self.fit = true;
    }

    /// Kept while it still points at the selection; moved onto it otherwise.
    fn follow(&mut self, graph: &JsonGraph, selected: Option<&str>) {
        if let Some(kept) = &self.cursor
            && path_of(graph, kept).as_deref() == selected
        {
            return;
        }
        let found = selected.and_then(|path| find(graph, path));
        self.cursor = match found {
            Some(super::Found::Node(node)) => Some(Cursor {
                node: node.path.clone(),
                row: None,
            }),
            Some(super::Found::Row { node, index, .. }) => Some(Cursor {
                node: node.path.clone(),
                row: Some(index),
            }),
            None => self
                .cursor
                .take()
                .filter(|kept| graph.nodes.iter().any(|node| node.path == kept.node)),
        };
    }

    fn zoom_by(&mut self, ground: Rect, step: i32) {
        let zoom = self.zoom();
        let at = SCALES
            .iter()
            .position(|scale| *scale >= zoom)
            .unwrap_or(SCALES.len() - 1);
        let next = at
            .saturating_add_signed(step as isize)
            .min(SCALES.len() - 1);
        self.zoom_to(ground, SCALES[next]);
    }

    /// Around the middle of the ground, which is what stays put under the eye.
    fn zoom_to(&mut self, ground: Rect, scale: f32) {
        let middle = ground.size() / 2.0;
        let ratio = scale / self.zoom();
        self.pan = Some(middle - (middle - self.pan()) * ratio);
        self.zoom = Some(scale);
    }
}

fn path_of(graph: &JsonGraph, cursor: &Cursor) -> Option<String> {
    let node = graph.nodes.iter().find(|node| node.path == cursor.node)?;
    match cursor.row {
        None => Some(node.path.clone()),
        Some(row) => node.rows.get(row).map(|row| row.path.clone()),
    }
}

/// The plane as drawn this frame: where a point of the layout lands on screen.
struct Plane {
    origin: Pos2,
    column: f32,
    zoom: f32,
}

impl Plane {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.column, y * ROW_PX) * self.zoom
    }

    fn rect(&self, node: &JsonNode, offset: Vec2) -> Rect {
        let (x, y) = (as_f32(node.x) + offset.x, as_f32(node.y) + offset.y);
        Rect::from_min_max(
            self.at(x, y),
            self.at(x + as_f32(node.width), y + as_f32(node.height)),
        )
    }
}

fn as_f32(units: u32) -> f32 {
    f32::from(u16::try_from(units).unwrap_or(u16::MAX))
}

/// The ground and its nodes; the path picked this frame, if any.
pub(super) fn show(
    ui: &mut Ui,
    explorer: &mut Explorer,
    view: &JsonView,
    language: Language,
) -> Option<String> {
    let graph = &view.graph;
    let selected = explorer.selected_path(view);
    explorer.ground.follow(graph, selected.as_deref());
    let rect = ui.max_rect();
    let ground = widgets::region(ui, rect, "json-graph", Sense::click_and_drag());
    ground.widget_info(|| {
        WidgetInfo::labeled(
            Role::Group,
            true,
            i18n::translate(language, keys::json::TREE, &[]),
        )
    });
    address(ui, &ground, "json-graph");
    paint_ground(ui, rect);
    let column = ui
        .fonts_mut(|fonts| fonts.glyph_width(&egui::FontId::new(FONT, FontFamily::Monospace), '0'));
    if std::mem::take(&mut explorer.ground.fit) {
        fit(&mut explorer.ground, graph, &explorer.offsets, rect, column);
    }
    if ground.dragged() && explorer.ground.dragged.is_none() {
        explorer.ground.pan = Some(explorer.ground.pan() + ground.drag_delta());
    }
    if ground.hovered() {
        let scrolled = ui.input(|input| input.smooth_scroll_delta);
        if scrolled != Vec2::ZERO {
            explorer.ground.pan = Some(explorer.ground.pan() + scrolled);
        }
    }
    if ground.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if ground.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    let plane = Plane {
        origin: rect.min + explorer.ground.pan(),
        column,
        zoom: explorer.ground.zoom(),
    };
    let painter = ui.painter_at(rect);
    let lit = lit(graph, &explorer.ground);
    edges(&painter, graph, explorer, &plane, &lit);
    let mut picked = None;
    for node in &graph.nodes {
        let placed = plane.rect(node, placed_offset(explorer, &node.path));
        if !placed.intersects(rect) {
            continue;
        }
        let drawn = Drawn {
            ground: rect,
            plane: &plane,
            lit: &lit,
            language,
        };
        if let Some(chosen) = draw_node(ui, &drawn, explorer, view, (node, placed)) {
            picked = Some(chosen);
            ground.request_focus();
        }
    }
    if ground.clicked() {
        ground.request_focus();
    }
    overlays(ui, explorer, rect, language);
    if std::mem::take(&mut explorer.ground.refocus) {
        ground.request_focus();
    }
    let had_focus = explorer.ground.focused;
    explorer.ground.focused = widgets::takes_keys(ui, &ground);
    let escaped = had_focus && ui.input(|input| input.key_pressed(Key::Escape));
    if (explorer.ground.focused || escaped)
        && let Some(chosen) = keys_of(ui, explorer, view, escaped)
    {
        picked = Some(chosen);
        ground.request_focus();
        explorer.ground.reveal = true;
    }
    if std::mem::take(&mut explorer.ground.reveal)
        && let Some(target) = cursor_rect(explorer, graph, &plane)
    {
        reveal(&mut explorer.ground, rect, target);
    }
    picked
}

fn paint_ground(ui: &Ui, rect: Rect) {
    let p = palette(ui.ctx());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, p.bg_0);
    let step = 20.0;
    let mut y = rect.top() + step / 2.0;
    while y < rect.bottom() {
        let mut x = rect.left() + step / 2.0;
        while x < rect.right() {
            painter.circle_filled(pos2(x, y), 1.0, p.line);
            x += step;
        }
        y += step;
    }
}

/// Where a node is drawn: where it was moved, or under the drag.
fn placed_offset(explorer: &Explorer, path: &str) -> Vec2 {
    match &explorer.ground.dragged {
        Some((dragged, offset)) if dragged == path => *offset,
        _ => explorer.offsets.get(path).copied().unwrap_or(Vec2::ZERO),
    }
}

/// The nodes from the root down to the one the cursor is in, by index.
fn lit(graph: &JsonGraph, ground: &Ground) -> Vec<u32> {
    let Some(cursor) = &ground.cursor else {
        return Vec::new();
    };
    let Some(node) = graph.nodes.iter().find(|node| node.path == cursor.node) else {
        return Vec::new();
    };
    lineage(graph, node)
        .into_iter()
        .map(|node| node.id)
        .collect()
}

fn edges(
    painter: &egui::Painter,
    graph: &JsonGraph,
    explorer: &Explorer,
    plane: &Plane,
    lit: &[u32],
) {
    let p = palette(painter.ctx());
    for node in &graph.nodes {
        let from = placed_offset(explorer, &node.path);
        for (index, row) in node.rows.iter().enumerate() {
            let Some(child) = child(graph, row) else {
                continue;
            };
            let to = placed_offset(explorer, &child.path);
            let x1 = as_f32(node.x) + from.x + as_f32(node.width);
            let y1 = as_f32(node.y) + from.y + 1.5 + as_f32(u32::try_from(index).unwrap_or(0));
            let x2 = as_f32(child.x) + to.x;
            let y2 = as_f32(child.y) + to.y + 0.5;
            let bend = (x2 - x1) / 2.0;
            let lit = lit.contains(&node.id) && lit.contains(&child.id);
            let colour = if lit { p.amber_edge } else { p.line };
            painter.add(CubicBezierShape::from_points_stroke(
                [
                    plane.at(x1, y1),
                    plane.at(x1 + bend, y1),
                    plane.at(x2 - bend, y2),
                    plane.at(x2, y2),
                ],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.6, colour),
            ));
        }
    }
}

/// What every node of a frame is drawn with.
struct Drawn<'a> {
    ground: Rect,
    plane: &'a Plane,
    lit: &'a [u32],
    language: Language,
}

/// A node, its header and rows, drawn at `placed`; the path a click picked.
fn draw_node(
    ui: &mut Ui,
    drawn: &Drawn<'_>,
    explorer: &mut Explorer,
    view: &JsonView,
    (node, placed): (&JsonNode, Rect),
) -> Option<String> {
    let p = palette(ui.ctx());
    let ground = drawn.ground;
    let line = ROW_PX * drawn.plane.zoom;
    // The whole node moves it; its header and rows, drawn over, take the clicks.
    let body = widgets::region(ui, placed.intersect(ground), &node.path, Sense::DRAG);
    drag(ui, explorer, node, &body, drawn.plane);

    let corner = CornerRadius::same(round(9.0 * drawn.plane.zoom));
    let painter = ui.painter_at(placed.intersect(ground));
    let dragged = explorer.ground.dragged.as_ref();
    if dragged.is_some_and(|(path, _)| *path == node.path) {
        ui.painter_at(ground)
            .add(p.shadow_dropdown.as_shape(placed, corner));
    }
    painter.rect_filled(placed, corner, p.bg_1);

    let head = Rect::from_min_size(placed.min, vec2(placed.width(), line));
    let mut picked = draw_head(ui, drawn, explorer, node, head);
    for (index, row) in node.rows.iter().enumerate() {
        let top = head.bottom() + line * as_f32(u32::try_from(index).unwrap_or(0));
        let rect = Rect::from_min_size(pos2(placed.left(), top), vec2(placed.width(), line));
        if !rect.intersects(ground) {
            continue;
        }
        if index > 0 {
            painter.hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line_soft));
        }
        if draw_row(ui, drawn, explorer, (node, index), rect) && explorer.ground.dragged.is_none() {
            explorer.ground.cursor = Some(Cursor {
                node: node.path.clone(),
                row: Some(index),
            });
            explorer.toggle(view, &row.path, row.opens, row.child.is_some());
            picked = Some(row.path.clone());
        }
    }
    if node.hidden_rows > 0 {
        let top = head.bottom() + line * as_f32(u32::try_from(node.rows.len()).unwrap_or(0));
        let words = i18n::translate(
            drawn.language,
            keys::json::HIDDEN_ROWS,
            &[("count", node.hidden_rows.into())],
        );
        let size = FONT * drawn.plane.zoom;
        let more = text(ui, &words, size, 400.0, FontFamily::Monospace, p.text_2);
        let at = pos2(
            head.left() + 10.0 * drawn.plane.zoom,
            top + (line - more.size().y) / 2.0,
        );
        painter.galley(at, more, p.text_2);
    }
    let current = is_cursor(&explorer.ground, node, None);
    let edge = if current || node.hit {
        p.amber_edge
    } else if drawn.lit.contains(&node.id) {
        p.amber_dim
    } else {
        p.line
    };
    painter.rect_stroke(placed, corner, Stroke::new(1.0, edge), StrokeKind::Inside);
    if current {
        ui.painter_at(ground).rect_stroke(
            placed.expand(4.0),
            corner,
            Stroke::new(4.0, p.amber_fill.gamma_multiply(0.15)),
            StrokeKind::Inside,
        );
    }
    picked
}

fn is_cursor(ground: &Ground, node: &JsonNode, row: Option<usize>) -> bool {
    ground
        .cursor
        .as_ref()
        .is_some_and(|cursor| cursor.node == node.path && cursor.row == row)
}

/// The label and the size; a click selects the node.
fn draw_head(
    ui: &mut Ui,
    drawn: &Drawn<'_>,
    explorer: &mut Explorer,
    node: &JsonNode,
    head: Rect,
) -> Option<String> {
    let p = palette(ui.ctx());
    let zoom = drawn.plane.zoom;
    let current = is_cursor(&explorer.ground, node, None);
    let painter = ui.painter_at(head.intersect(drawn.ground));
    let corner = round(9.0 * zoom);
    let fill = if current {
        p.amber_fill.gamma_multiply(0.14)
    } else {
        p.bg_2
    };
    let top_corners = CornerRadius {
        nw: corner,
        ne: corner,
        sw: 0,
        se: 0,
    };
    painter.rect_filled(head, top_corners, fill);
    painter.hline(head.x_range(), head.bottom(), Stroke::new(1.0, p.line_soft));
    let salt = (&node.path, "head");
    let response = widgets::region(ui, head.intersect(drawn.ground), salt, Sense::CLICK);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &node.label));
    address(ui, &response, "json-node-head");
    describe(ui, &response, &node.path, None);
    let pad = 10.0 * zoom;
    let ink = if current || drawn.lit.contains(&node.id) {
        p.amber_text
    } else {
        p.text_0
    };
    let size_words = format!("{} {}", braces(node.kind), node.size);
    let size = text(
        ui,
        &size_words,
        10.0 * zoom,
        400.0,
        FontFamily::Monospace,
        p.text_2,
    );
    let size_at = pos2(
        head.right() - pad - size.size().x,
        head.center().y - size.size().y / 2.0,
    );
    let name = text(
        ui,
        &node.label,
        FONT * zoom,
        700.0,
        FontFamily::Monospace,
        ink,
    );
    let name_at = pos2(head.left() + pad, head.center().y - name.size().y / 2.0);
    painter.galley(name_at, name, ink);
    painter.galley(size_at, size, p.text_2);
    if !response.clicked() || explorer.ground.dragged.is_some() {
        return None;
    }
    explorer.ground.cursor = Some(Cursor {
        node: node.path.clone(),
        row: None,
    });
    explorer.select(&node.path);
    Some(node.path.clone())
}

/// A row: its key on the left, its value on the right in its kind's colour. `true` once clicked.
fn draw_row(
    ui: &mut Ui,
    drawn: &Drawn<'_>,
    explorer: &Explorer,
    (node, index): (&JsonNode, usize),
    rect: Rect,
) -> bool {
    let p = palette(ui.ctx());
    let row = &node.rows[index];
    let zoom = drawn.plane.zoom;
    let shown = rect.intersect(drawn.ground);
    let response = widgets::region(ui, shown, (&node.path, index), Sense::CLICK);
    let words = format!("{} {}", row.key, row.value);
    let open = row.child.is_some();
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &words));
    address(ui, &response, "json-row");
    describe(ui, &response, &row.path, row.opens.then_some(open));
    let painter = ui.painter_at(shown);
    let fill = if row.hit {
        Some(p.amber_fill.gamma_multiply(0.18))
    } else if is_cursor(&explorer.ground, node, Some(index)) {
        Some(p.amber_fill.gamma_multiply(0.1))
    } else if response.hovered() {
        Some(p.bg_2)
    } else {
        None
    };
    if let Some(fill) = fill {
        painter.rect_filled(rect.shrink2(vec2(1.0, 0.5)), CornerRadius::ZERO, fill);
    }
    let pad = 10.0 * zoom;
    let middle = |height: f32| rect.center().y - height / 2.0;
    let key = text(
        ui,
        &row.key,
        FONT * zoom,
        400.0,
        FontFamily::Monospace,
        p.text_1,
    );
    let left = rect.left() + pad + key.size().x + 8.0 * zoom;
    painter.galley(pos2(rect.left() + pad, middle(key.size().y)), key, p.text_1);
    let mut right = rect.right() - pad;
    if row.opens {
        let chevron = if open { "‹" } else { "›" };
        let mark = text(
            ui,
            chevron,
            FONT * zoom,
            400.0,
            FontFamily::Monospace,
            p.text_2,
        );
        right -= mark.size().x;
        painter.galley(pos2(right, middle(mark.size().y)), mark, p.text_2);
        right -= 8.0 * zoom;
    }
    let ink = kind_colour(p, row.kind);
    let value = text(
        ui,
        &row.value,
        FONT * zoom,
        400.0,
        FontFamily::Monospace,
        ink,
    );
    let x = (right - value.size().x).max(left);
    let room = Rect::from_x_y_ranges(left..=right.max(left), rect.y_range());
    painter
        .with_clip_rect(room.intersect(painter.clip_rect()))
        .galley(pos2(x, middle(value.size().y)), value, ink);
    response.clicked()
}

fn round(value: f32) -> u8 {
    // A corner of a few points, never past a byte.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped first"
    )]
    let rounded = value.clamp(0.0, 255.0).round() as u8;
    rounded
}

fn braces(kind: JsonKind) -> &'static str {
    if kind == JsonKind::Object {
        "{ }"
    } else {
        "[ ]"
    }
}

/// The path, which a test and the e2e driver find an entry by — no translation touches it — and
/// whether a container is open.
pub(super) fn describe(ui: &Ui, response: &Response, path: &str, open: Option<bool>) {
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_description(path.to_owned());
        if let Some(open) = open {
            node.set_expanded(open);
        }
    });
}

/// A press that travels moves the node; let go, it is written down.
fn drag(ui: &Ui, explorer: &mut Explorer, node: &JsonNode, body: &Response, plane: &Plane) {
    let units = vec2(plane.column, ROW_PX) * plane.zoom;
    if body.drag_started() {
        let from = explorer
            .offsets
            .get(&node.path)
            .copied()
            .unwrap_or(Vec2::ZERO);
        explorer.ground.dragged = Some((node.path.clone(), from));
    }
    if body.dragged()
        && let Some((path, offset)) = &mut explorer.ground.dragged
        && *path == node.path
    {
        *offset += body.drag_delta() / units;
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    }
    if body.drag_stopped()
        && let Some((path, offset)) = explorer.ground.dragged.take()
    {
        explorer.move_node(&path, offset);
    }
}

/// The reset, the zoom: drawn over the ground's bottom corners.
fn overlays(ui: &mut Ui, explorer: &mut Explorer, ground: Rect, language: Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    if !explorer.offsets.is_empty() {
        let at = Rect::from_min_size(
            pos2(ground.left() + 16.0, ground.bottom() - 16.0 - TOOL_HEIGHT),
            vec2(200.0, TOOL_HEIGHT),
        );
        let mut corner = ui.new_child(egui::UiBuilder::new().id_salt("json-reset").max_rect(at));
        if widgets::Button::new(&tr(keys::json::RESET_LAYOUT), "json-reset-layout")
            .show(&mut corner)
            .clicked()
        {
            explorer.offsets.clear();
        }
    }
    let width = 120.0;
    let at = Rect::from_min_size(
        pos2(
            ground.right() - 16.0 - width,
            ground.bottom() - 16.0 - TOOL_HEIGHT,
        ),
        vec2(width, TOOL_HEIGHT),
    );
    let mut corner = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("json-zoom")
            .max_rect(at)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    corner.spacing_mut().item_spacing.x = 2.0;
    let p = palette(ui.ctx());
    corner.painter().rect(
        at,
        CornerRadius::same(8),
        p.bg_2,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    if widgets::IconButton::new(
        crate::theme::icons::Icon::Plus,
        &tr(keys::json::ZOOM_IN),
        "json-zoom-in",
    )
    .show(&mut corner)
    .clicked()
    {
        explorer.ground.zoom_by(ground, 1);
    }
    let level = format!("{} %", (explorer.ground.zoom() * 100.0).round());
    if widgets::Button::new(&level, "json-zoom-reset")
        .kind(widgets::Kind::Quiet)
        .show(&mut corner)
        .clicked()
    {
        explorer.ground.zoom_to(ground, 1.0);
    }
    if widgets::IconButton::new(
        crate::theme::icons::Icon::Minus,
        &tr(keys::json::ZOOM_OUT),
        "json-zoom-out",
    )
    .show(&mut corner)
    .clicked()
    {
        explorer.ground.zoom_by(ground, -1);
    }
}

/// The whole graph in the ground, moved nodes included, never past 100 %.
fn fit(
    ground: &mut Ground,
    graph: &JsonGraph,
    offsets: &std::collections::HashMap<String, Vec2>,
    rect: Rect,
    column: f32,
) {
    let placed = graph.nodes.iter().map(|node| {
        let offset = offsets.get(&node.path).copied().unwrap_or(Vec2::ZERO);
        let x = as_f32(node.x) + offset.x;
        let y = as_f32(node.y) + offset.y;
        Rect::from_min_max(
            pos2(x, y),
            pos2(x + as_f32(node.width), y + as_f32(node.height)),
        )
    });
    let Some(bounds) = placed.reduce(Rect::union) else {
        return;
    };
    let size = vec2(bounds.width() * column, bounds.height() * ROW_PX);
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let room = vec2(
        rect.width() - 2.0 * START.x,
        rect.height() - START.y - START.x,
    );
    let scale = (room.x / size.x).min(room.y / size.y).clamp(SCALES[0], 1.0);
    ground.zoom = Some(scale);
    ground.pan = Some(vec2(
        (rect.width() - size.x * scale) / 2.0 - bounds.left() * column * scale,
        START.y + (room.y - size.y * scale) / 2.0 - bounds.top() * ROW_PX * scale,
    ));
}

fn cursor_rect(explorer: &Explorer, graph: &JsonGraph, plane: &Plane) -> Option<Rect> {
    let cursor = explorer.ground.cursor.as_ref()?;
    let node = graph.nodes.iter().find(|node| node.path == cursor.node)?;
    let placed = plane.rect(node, placed_offset(explorer, &node.path));
    let line = ROW_PX * plane.zoom;
    let row = cursor
        .row
        .map_or(0.0, |row| as_f32(u32::try_from(row + 1).unwrap_or(0)));
    Some(Rect::from_min_size(
        placed.min + vec2(0.0, row * line),
        vec2(placed.width(), line),
    ))
}

/// Pans just enough to bring the cursor inside the ground.
fn reveal(ground: &mut Ground, rect: Rect, target: Rect) {
    let mut moved = Vec2::ZERO;
    if target.right() > rect.right() - REVEAL_MARGIN {
        moved.x = rect.right() - REVEAL_MARGIN - target.right();
    }
    if target.left() + moved.x < rect.left() + REVEAL_MARGIN {
        moved.x = rect.left() + REVEAL_MARGIN - target.left();
    }
    if target.bottom() > rect.bottom() - REVEAL_MARGIN {
        moved.y = rect.bottom() - REVEAL_MARGIN - target.bottom();
    }
    if target.top() + moved.y < rect.top() + START.y {
        moved.y = rect.top() + START.y - target.top();
    }
    if moved != Vec2::ZERO {
        ground.pan = Some(ground.pan() + moved);
    }
}

/// The ground's keys; the path the cursor moved to. `escaped`: egui let go on Escape this frame.
fn keys_of(ui: &Ui, explorer: &mut Explorer, view: &JsonView, escaped: bool) -> Option<String> {
    let graph = &view.graph;
    if super::fold_keys(ui, explorer, view) {
        explorer.ground.refocus = true;
        return None;
    }
    let Some(cursor) = explorer.ground.cursor.clone() else {
        let arrow = ui.input(|input| {
            [
                Key::ArrowDown,
                Key::ArrowUp,
                Key::ArrowLeft,
                Key::ArrowRight,
            ]
            .iter()
            .any(|key| input.key_pressed(*key))
        });
        let root = graph.nodes.first()?;
        return arrow.then(|| move_to(explorer, root, None));
    };
    let node = graph.nodes.iter().find(|node| node.path == cursor.node)?;
    let pressed =
        |modifiers: Modifiers, key: Key| ui.input_mut(|input| input.consume_key(modifiers, key));
    if escaped {
        if cursor.row.is_none() && node.parent.is_none() {
            return None;
        }
        ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        return go_up(explorer, graph, node, cursor.row);
    }
    for (key, step) in [
        (Key::ArrowLeft, vec2(-NUDGE.x, 0.0)),
        (Key::ArrowRight, vec2(NUDGE.x, 0.0)),
        (Key::ArrowUp, vec2(0.0, -NUDGE.y)),
        (Key::ArrowDown, vec2(0.0, NUDGE.y)),
    ] {
        if pressed(Modifiers::ALT, key) {
            let from = explorer
                .offsets
                .get(&node.path)
                .copied()
                .unwrap_or(Vec2::ZERO);
            explorer.move_node(&node.path, from + step);
            explorer.ground.reveal = true;
            return None;
        }
    }
    let row = cursor.row.and_then(|at| node.rows.get(at));
    if pressed(Modifiers::NONE, Key::ArrowDown) {
        let next = cursor.row.map_or(0, |at| at + 1);
        return (next < node.rows.len()).then(|| move_to(explorer, node, Some(next)));
    }
    if pressed(Modifiers::NONE, Key::ArrowUp) {
        let previous = cursor.row.and_then(|at| at.checked_sub(1));
        return cursor.row.map(|_| move_to(explorer, node, previous));
    }
    if pressed(Modifiers::NONE, Key::ArrowRight) {
        if let Some(child) = row.and_then(|row| child(graph, row)) {
            return Some(move_to(explorer, child, None));
        }
        if let Some(row) = row.filter(|row| row.opens) {
            explorer.toggle(view, &row.path, true, false);
            explorer.ground.refocus = true;
            return Some(row.path.clone());
        }
        if row.is_none() && !node.rows.is_empty() {
            return Some(move_to(explorer, node, Some(0)));
        }
        return None;
    }
    if pressed(Modifiers::NONE, Key::ArrowLeft) {
        return go_up(explorer, graph, node, cursor.row);
    }
    if (pressed(Modifiers::NONE, Key::Enter) || pressed(Modifiers::NONE, Key::Space))
        && let Some(row) = row
    {
        explorer.toggle(view, &row.path, row.opens, row.child.is_some());
        explorer.ground.refocus = true;
        return Some(row.path.clone());
    }
    None
}

fn move_to(explorer: &mut Explorer, node: &JsonNode, row: Option<usize>) -> String {
    explorer.ground.cursor = Some(Cursor {
        node: node.path.clone(),
        row,
    });
    let path = row
        .and_then(|at| node.rows.get(at))
        .map_or_else(|| node.path.clone(), |row| row.path.clone());
    explorer.select(&path);
    path
}

/// From a row to its header, from a header to the row that opened it.
fn go_up(
    explorer: &mut Explorer,
    graph: &JsonGraph,
    node: &JsonNode,
    row: Option<usize>,
) -> Option<String> {
    if row.is_some() {
        return Some(move_to(explorer, node, None));
    }
    let up = parent(graph, node)?;
    let index = up.rows.iter().position(|row| row.child == Some(node.id));
    Some(move_to(explorer, up, index))
}
