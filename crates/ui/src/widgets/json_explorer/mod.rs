//! The JSON visualiser (the reference's `json-explorer`, `json-graph` and `json-tree`): the graph
//! or the tree of a `JsonView`, the search over them, what is selected and the document. The
//! model is the core's and the state the caller's [`Explorer`]: two documents can be explored at
//! once, a note's and later an HTTP answer's. What is open is read back from each answer: the
//! paths sent are the nodes drawn, plus or minus the one clicked, and the core decides the rest.

mod graph;
mod reads;
mod tree;

use std::collections::HashMap;

use egui::{
    Align, Color32, CornerRadius, FontFamily, Key, Layout, Modifiers, Rect, Role, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};
use engine::json::model::{JsonKind, JsonOpening, JsonQuery, JsonView};
use engine::json::parse::{JsonErrorReason, Span};
use engine::json::read::{JsonNotice, StructureFormat};
use i18n::{Key as Words, Language, keys};

use reads::{Found, find, lineage, preview, shows, trail_of};
pub use reads::{entry_at, span_of, utf16_slice};

use super::{Button, IconButton, SearchField, address, text};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::theme::tokens::Palette;

/// The search is sent once the typing pauses.
const SEARCH_AFTER: f64 = 0.15;
const ASIDE_WIDTH: f32 = 260.0;
/// Narrower, the panel goes under the graph with the path and its actions alone.
const WIDE_FROM: f32 = 600.0;
const NARROW_ASIDE: f32 = 92.0;
const CRUMBS_HEIGHT: f32 = 36.0;
const TOOL_HEIGHT: f32 = 32.0;
const SEARCH_WIDTH: f32 = 260.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Graph,
    Tree,
}

/// One explorer's state, kept by whoever shows the document.
#[derive(Debug)]
pub struct Explorer {
    search: String,
    /// What was last sent: the search once the typing paused.
    searched: String,
    search_due: Option<f64>,
    opening: JsonOpening,
    selected: Option<String>,
    /// The selection and what held it when the view last folded: where it falls back to.
    trail: Vec<String>,
    /// Among the matches, `None` before the first step.
    match_index: Option<usize>,
    /// By path, in the layout's units, so an edited document keeps the place of every node
    /// whose path survives.
    offsets: HashMap<String, Vec2>,
    ground: graph::Ground,
    tree: tree::Lines,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            search: String::new(),
            searched: String::new(),
            search_due: None,
            opening: JsonOpening::Initial,
            selected: None,
            trail: Vec::new(),
            match_index: None,
            offsets: HashMap::new(),
            ground: graph::Ground::default(),
            tree: tree::Lines::default(),
        }
    }
}

impl Explorer {
    /// What the core is asked of `text`.
    pub fn query(&self, text: String, format: StructureFormat) -> JsonQuery {
        JsonQuery {
            text,
            format,
            search: self.searched.trim().to_owned(),
            opening: self.opening.clone(),
        }
    }

    /// The selection while the view draws it; folded away, the nearest of what held it.
    pub fn selected_path(&self, view: &JsonView) -> Option<String> {
        let path = self.selected.as_deref()?;
        if shows(view, path) {
            return Some(path.to_owned());
        }
        let nearest = self.trail.iter().rev().find(|each| shows(view, each));
        Some(nearest.map_or(path, String::as_str).to_owned())
    }

    pub fn select(&mut self, path: &str) {
        self.selected = Some(path.to_owned());
        self.trail.clear();
    }

    /// A container opens or closes, and is the selection either way.
    fn toggle(&mut self, view: &JsonView, path: &str, opens: bool, open: bool) {
        self.select(path);
        if !opens {
            return;
        }
        let mut paths = open_paths(view);
        if open {
            paths.retain(|each| each != path);
        } else {
            paths.push(path.to_owned());
        }
        self.opening = JsonOpening::Paths {
            paths,
            reveal: None,
        };
    }

    fn open_all(&mut self) {
        self.opening = JsonOpening::All;
    }

    /// Only the root open, which the core keeps open whatever is asked.
    fn collapse_all(&mut self, view: &JsonView) {
        self.trail = self
            .selected_path(view)
            .map(|path| trail_of(view, &path))
            .unwrap_or_default();
        self.opening = JsonOpening::Paths {
            paths: Vec::new(),
            reveal: None,
        };
    }

    /// To the next match or the previous one, opening what it sits in.
    fn step(&mut self, view: &JsonView, forward: bool) -> Option<String> {
        let count = view.matches.len();
        if count == 0 {
            return None;
        }
        let index = match (self.match_index, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(at), true) => (at + 1) % count,
            (Some(at), false) => (at + count - 1) % count,
        };
        let path = view.matches[index].clone();
        self.match_index = Some(index);
        self.select(&path);
        self.opening = JsonOpening::Paths {
            paths: open_paths(view),
            reveal: Some(path.clone()),
        };
        self.ground.reveal = true;
        self.tree.reveal = true;
        Some(path)
    }

    /// Back where the core placed it at no offset.
    fn move_node(&mut self, path: &str, offset: Vec2) {
        if offset == Vec2::ZERO {
            self.offsets.remove(path);
        } else {
            self.offsets.insert(path.to_owned(), offset);
        }
    }

    /// The search sent once due; while it waits, how long is left.
    fn send_search(&mut self, now: f64) -> Option<f64> {
        let due = self.search_due?;
        if now < due {
            return Some(due - now);
        }
        self.search_due = None;
        self.searched.clone_from(&self.search);
        self.match_index = None;
        None
    }
}

fn open_paths(view: &JsonView) -> Vec<String> {
    view.graph
        .nodes
        .iter()
        .map(|node| node.path.clone())
        .collect()
}

/// Shift with an arrow, on the graph or the tree: everything opens, or folds to the root.
fn fold_keys(ui: &Ui, explorer: &mut Explorer, view: &JsonView) -> bool {
    let (open, fold) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::SHIFT, Key::ArrowRight),
            input.consume_key(Modifiers::SHIFT, Key::ArrowLeft),
        )
    });
    if open {
        explorer.open_all();
    } else if fold {
        explorer.collapse_all(view);
    }
    open || fold
}

/// A value's colour by its kind, as the reference's rows have it.
fn kind_colour(p: &Palette, kind: JsonKind) -> Color32 {
    match kind {
        JsonKind::String => p.green,
        JsonKind::Number => p.red,
        JsonKind::Boolean => p.purple,
        JsonKind::Datetime => p.blue,
        JsonKind::Object | JsonKind::Array | JsonKind::Null => p.text_2,
    }
}

/// How a format names itself in a sentence: « YAML valide ».
pub fn format_name(format: StructureFormat) -> &'static str {
    match format {
        StructureFormat::Json => "JSON",
        StructureFormat::Yaml => "YAML",
        StructureFormat::Toml => "TOML",
        StructureFormat::Xml => "XML",
    }
}

pub fn reason(reason: JsonErrorReason) -> Words {
    use keys::json::reasons;
    match reason {
        JsonErrorReason::UnexpectedEnd => reasons::UNEXPECTED_END,
        JsonErrorReason::UnexpectedCharacter => reasons::UNEXPECTED_CHARACTER,
        JsonErrorReason::InvalidNumber => reasons::INVALID_NUMBER,
        JsonErrorReason::InvalidEscape => reasons::INVALID_ESCAPE,
        JsonErrorReason::ControlCharacter => reasons::CONTROL_CHARACTER,
        JsonErrorReason::TrailingCharacters => reasons::TRAILING_CHARACTERS,
        JsonErrorReason::TooDeep => reasons::TOO_DEEP,
        JsonErrorReason::Malformed => reasons::MALFORMED,
        JsonErrorReason::TooLarge => reasons::TOO_LARGE,
    }
}

fn kind_words(kind: JsonKind) -> Words {
    use keys::json::kinds;
    match kind {
        JsonKind::Object => kinds::OBJECT,
        JsonKind::Array => kinds::ARRAY,
        JsonKind::String => kinds::STRING,
        JsonKind::Number => kinds::NUMBER,
        JsonKind::Boolean => kinds::BOOLEAN,
        JsonKind::Null => kinds::NULL,
        JsonKind::Datetime => kinds::DATETIME,
    }
}

/// What a frame of the explorer asked of whoever shows it.
#[derive(Debug, Default)]
pub struct Output {
    /// « Voir dans le code »: the span, in the explored text's UTF-16 units.
    pub show_in_code: Option<Span>,
    /// The graph is too large and proposes the tree.
    pub tree_proposed: bool,
    /// Picked by a click or a key this frame.
    pub chosen: Option<String>,
}

/// What the selection panel shows of the entry selected.
struct Selected {
    path: String,
    kind: JsonKind,
    size: u32,
    preview: String,
    span: Span,
    /// A string's text, unquoted: copied as it is written otherwise.
    text: Option<String>,
}

fn selection(explorer: &Explorer, view: &JsonView) -> Option<Selected> {
    let path = explorer
        .selected_path(view)
        .or_else(|| view.graph.nodes.first().map(|node| node.path.clone()))?;
    match find(&view.graph, &path) {
        Some(Found::Node(node)) => Some(Selected {
            path: node.path.clone(),
            kind: node.kind,
            size: node.size,
            preview: preview(node),
            span: node.span,
            text: None,
        }),
        Some(Found::Row { row, .. }) => Some(Selected {
            path: row.path.clone(),
            kind: row.kind,
            size: row.size,
            preview: row.value.clone(),
            span: row.span,
            text: row.text.clone(),
        }),
        None => view
            .tree
            .iter()
            .find(|line| line.path == path)
            .map(|line| Selected {
                path: line.path.clone(),
                kind: line.kind,
                size: line.size,
                preview: line.value.clone(),
                span: line.span,
                text: line.text.clone(),
            }),
    }
}

/// What a frame draws: the explorer over `view`, made of `text`.
pub struct ExplorerView<'a> {
    explorer: &'a mut Explorer,
    view: &'a JsonView,
    text: &'a str,
    mode: Mode,
    format: StructureFormat,
    language: Language,
    /// A code beside it to show a node in: not under an answer, which is read, not edited.
    with_code: bool,
}

impl<'a> ExplorerView<'a> {
    pub fn new(explorer: &'a mut Explorer, view: &'a JsonView, text: &'a str) -> Self {
        Self {
            explorer,
            view,
            text,
            mode: Mode::Graph,
            format: StructureFormat::Json,
            language: Language::Fr,
            with_code: true,
        }
    }

    #[must_use]
    pub fn with_code(mut self, with_code: bool) -> Self {
        self.with_code = with_code;
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// The messages name it, and a format other than JSON says what it drops.
    #[must_use]
    pub fn format(mut self, format: StructureFormat) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> Output {
        let mut output = Output::default();
        let now = ui.input(|input| input.time);
        if let Some(wait) = self.explorer.send_search(now) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
        let whole = ui.max_rect();
        let wide = whole.width() >= WIDE_FROM;
        let selected = selection(self.explorer, self.view);
        let (stage, aside) = if wide {
            let split = whole.right() - ASIDE_WIDTH;
            (
                Rect::from_min_max(whole.min, pos2(split, whole.bottom())),
                Rect::from_min_max(pos2(split, whole.top()), whole.max),
            )
        } else {
            let room = if selected.is_some() {
                NARROW_ASIDE
            } else {
                0.0
            };
            let split = whole.bottom() - room;
            (
                Rect::from_min_max(whole.min, pos2(whole.right(), split)),
                Rect::from_min_max(pos2(whole.left(), split), whole.max),
            )
        };
        let mut stage_ui = ui.new_child(UiBuilder::new().id_salt("json-stage").max_rect(stage));
        self.stage(&mut stage_ui, &mut output);
        if aside.height() > 0.0 {
            let mut aside_ui = ui.new_child(UiBuilder::new().id_salt("json-aside").max_rect(aside));
            self.aside(&mut aside_ui, wide, selected.as_ref(), &mut output);
        }
        ui.advance_cursor_after_rect(whole);
        output
    }

    fn tr(&self, key: Words) -> String {
        i18n::translate(self.language, key, &[])
    }

    fn stage(&mut self, ui: &mut Ui, output: &mut Output) {
        let rect = ui.max_rect();
        if let Some(error) = &self.view.error {
            self.invalid(ui, error, output);
            return;
        }
        let crumbs = self.mode == Mode::Graph && !self.view.graph.nodes.is_empty();
        let surface = if crumbs {
            Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - CRUMBS_HEIGHT))
        } else {
            rect
        };
        let mut surface_ui =
            ui.new_child(UiBuilder::new().id_salt("json-surface").max_rect(surface));
        let picked = match self.mode {
            Mode::Graph => graph::show(&mut surface_ui, self.explorer, self.view, self.language),
            Mode::Tree => tree::show(&mut surface_ui, self.explorer, self.view, self.language),
        };
        if picked.is_some() {
            output.chosen = picked;
        }
        self.tools(ui, surface, output);
        if self.mode == Mode::Graph && self.view.graph.folded {
            self.folded(ui, surface, output);
        }
        if crumbs {
            let bar = Rect::from_min_max(pos2(rect.left(), surface.bottom()), rect.max);
            self.crumbs(ui, bar, output);
        }
    }

    fn invalid(&self, ui: &mut Ui, error: &engine::json::parse::JsonError, output: &mut Output) {
        let p = palette(ui.ctx());
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, CornerRadius::ZERO, p.bg_0);
        let said = format!(
            "{} {}",
            i18n::translate(
                self.language,
                keys::json::INVALID_AT,
                &[
                    ("line", error.line.into()),
                    ("column", error.column.into()),
                    ("format", format_name(self.format).into()),
                ],
            ),
            self.tr(reason(error.reason)),
        );
        let mut inner = ui.new_child(
            UiBuilder::new()
                .id_salt("json-invalid")
                .max_rect(rect.shrink(24.0))
                .layout(Layout::top_down(Align::Center)),
        );
        inner.add_space((rect.height() / 2.0 - 40.0).max(0.0));
        let label = inner.label(egui::RichText::new(&said).size(13.0).color(p.text_1));
        label.widget_info(|| WidgetInfo::labeled(Role::Status, true, &said));
        address(&inner, &label, "json-invalid");
        inner.add_space(12.0);
        if self.with_code
            && Button::new(&self.tr(keys::json::SHOW_IN_CODE), "json-error-show")
                .show(&mut inner)
                .clicked()
        {
            output.show_in_code = Some(Span {
                start: error.offset,
                end: error.offset,
            });
        }
    }

    /// The search and the folding, over the surface's top-left corner.
    fn tools(&mut self, ui: &mut Ui, surface: Rect, output: &mut Output) {
        let at = Rect::from_min_size(
            surface.min + vec2(16.0, 12.0),
            vec2(surface.width() - 32.0, TOOL_HEIGHT),
        );
        let mut row = ui.new_child(
            UiBuilder::new()
                .id_salt("json-tools")
                .max_rect(at)
                .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
        );
        row.set_clip_rect(surface);
        row.spacing_mut().item_spacing = vec2(8.0, 8.0);
        let name = self.tr(keys::json::SEARCH);
        let hint = self.tr(keys::json::SEARCH_PLACEHOLDER);
        let count = self.view.match_count;
        let matched = match self.explorer.match_index {
            None => count.to_string(),
            Some(at) => format!("{} / {count}", at + 1),
        };
        let searching = !self.explorer.search.is_empty();
        let typed = self.explorer.search.clone();
        let searched = SearchField::new(&mut self.explorer.search, &name, "json-search")
            .hint(&hint)
            .matched(searching.then_some(matched.as_str()))
            .matched_id("json-match-count")
            .width(SEARCH_WIDTH.min(at.width() / 2.0))
            .show(&mut row);
        let field = searched.field;
        if self.explorer.search != typed {
            self.explorer.search_due = Some(row.input(|input| input.time) + SEARCH_AFTER);
        }
        if field.lost_focus() && row.input(|input| input.key_pressed(Key::Enter)) {
            let back = row.input(|input| input.modifiers.shift);
            output.chosen = self
                .explorer
                .step(self.view, !back)
                .or(output.chosen.take());
            field.request_focus();
        }
        if field.has_focus()
            && searching
            && row.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.explorer.search.clear();
            self.explorer.search_due = Some(row.input(|input| input.time));
        }
        if searching {
            if IconButton::new(
                Icon::ArrowUp,
                &self.tr(keys::json::PREVIOUS_MATCH),
                "json-match-previous",
            )
            .show(&mut row)
            .clicked()
            {
                output.chosen = self.explorer.step(self.view, false);
            }
            if IconButton::new(
                Icon::ArrowDown,
                &self.tr(keys::json::NEXT_MATCH),
                "json-match-next",
            )
            .show(&mut row)
            .clicked()
            {
                output.chosen = self.explorer.step(self.view, true);
            }
        }
        let open = Button::new(&self.tr(keys::json::OPEN_ALL), "json-open-all").show(&mut row);
        if super::tooltip(open, &self.tr(keys::json::OPEN_ALL_KEYS)).clicked() {
            self.explorer.open_all();
        }
        let collapse =
            Button::new(&self.tr(keys::json::COLLAPSE_ALL), "json-collapse-all").show(&mut row);
        if super::tooltip(collapse, &self.tr(keys::json::COLLAPSE_ALL_KEYS)).clicked() {
            self.explorer.collapse_all(self.view);
        }
        if self.mode == Mode::Graph
            && Button::new(&self.tr(keys::json::FIT), "json-fit")
                .show(&mut row)
                .clicked()
        {
            self.explorer.ground.fit();
        }
    }

    fn folded(&self, ui: &mut Ui, surface: Rect, output: &mut Output) {
        let p = palette(ui.ctx());
        let words = self.tr(keys::json::FOLDED);
        let show = self.tr(keys::json::SHOW_TREE);
        let width = text(ui, &words, 12.0, 400.0, FontFamily::Proportional, p.text_1)
            .size()
            .x
            + Button::width(ui, &show)
            + 40.0;
        let at = Rect::from_center_size(
            pos2(surface.center().x, surface.bottom() - 56.0 - 20.0),
            vec2(width, 40.0),
        );
        ui.painter().rect(
            at,
            CornerRadius::same(10),
            p.bg_3,
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        let mut inner = ui.new_child(
            UiBuilder::new()
                .id_salt("json-folded")
                .max_rect(at.shrink2(vec2(14.0, 4.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        inner.spacing_mut().item_spacing.x = 12.0;
        let said = inner.label(egui::RichText::new(&words).size(12.0).color(p.text_1));
        address(&inner, &said, "json-folded");
        if Button::new(&show, "json-show-tree")
            .show(&mut inner)
            .clicked()
        {
            output.tree_proposed = true;
        }
    }

    fn crumbs(&mut self, ui: &mut Ui, bar: Rect, output: &mut Output) {
        let p = palette(ui.ctx());
        ui.painter().rect_filled(bar, CornerRadius::ZERO, p.bg_1);
        ui.painter()
            .hline(bar.x_range(), bar.top(), Stroke::new(1.0, p.line_soft));
        let graph = &self.view.graph;
        let found = self
            .explorer
            .selected_path(self.view)
            .and_then(|path| find(graph, &path));
        let Some(node) = found.map(Found::node).or_else(|| graph.nodes.first()) else {
            return;
        };
        let mut chain: Vec<(&str, &str)> = lineage(graph, node)
            .into_iter()
            .map(|each| (each.label.as_str(), each.path.as_str()))
            .collect();
        if let Some(Found::Row { row, .. }) = found {
            chain.push((row.key.as_str(), row.path.as_str()));
        }
        let mut row = ui.new_child(
            UiBuilder::new()
                .id_salt("json-crumbs")
                .max_rect(bar.shrink2(vec2(16.0, 0.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.spacing_mut().item_spacing.x = 6.0;
        let response = super::region(&mut row, bar, "json-crumbs", Sense::hover());
        let all: Vec<&str> = chain.iter().map(|(label, _)| *label).collect();
        let named = all.join(" › ");
        response.widget_info(|| WidgetInfo::labeled(Role::Navigation, true, &named));
        address(&row, &response, "json-crumbs");
        let last = chain.len() - 1;
        for (index, (label, path)) in chain.iter().enumerate() {
            let ink = if index == last {
                p.amber_text
            } else {
                p.text_2
            };
            let galley = text(&row, label, 12.0, 400.0, FontFamily::Monospace, ink);
            let (rect, crumb) = row.allocate_exact_size(
                galley.size().max(vec2(0.0, super::MIN_TARGET)),
                Sense::click(),
            );
            crumb.widget_info(|| WidgetInfo::labeled(Role::Link, true, *label));
            let ink = if crumb.hovered() { p.text_0 } else { ink };
            row.painter().galley(
                pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
            if crumb.clicked() {
                self.explorer.select(path);
                output.chosen = Some((*path).to_owned());
            }
            if index != last {
                row.label(egui::RichText::new("›").size(12.0).color(p.line_no));
            }
        }
    }

    fn aside(&mut self, ui: &mut Ui, wide: bool, selected: Option<&Selected>, output: &mut Output) {
        let p = palette(ui.ctx());
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, CornerRadius::ZERO, p.bg_1);
        if wide {
            ui.painter()
                .vline(rect.left(), rect.y_range(), Stroke::new(1.0, p.line_soft));
        } else {
            ui.painter()
                .hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line_soft));
        }
        let mut inner = ui.new_child(
            UiBuilder::new()
                .id_salt("json-aside-inner")
                .max_rect(rect.shrink(if wide { 16.0 } else { 10.0 }))
                .layout(Layout::top_down(Align::Min)),
        );
        let response = super::region(&mut inner, rect, "json-selection", Sense::hover());
        let label = self.tr(keys::json::SELECTION_LABEL);
        response.widget_info(|| WidgetInfo::labeled(Role::Group, true, &label));
        address(&inner, &response, "json-selection");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut inner, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                if let Some(selected) = selected {
                    self.selected(ui, wide, selected, output);
                }
                if wide {
                    self.document(ui);
                }
            });
    }

    fn selected(&self, ui: &mut Ui, wide: bool, selected: &Selected, output: &mut Output) {
        let p = palette(ui.ctx());
        if wide {
            title(ui, &self.tr(keys::json::SELECTION));
        }
        egui::Frame::new()
            .fill(p.bg_0)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(egui::Margin::symmetric(10, if wide { 8 } else { 6 }))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let path = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&selected.path)
                            .size(12.0)
                            .family(FontFamily::Monospace)
                            .color(p.amber_text),
                    )
                    .wrap(),
                );
                address(ui, &path, "json-selected-path");
            });
        let copy_path = self.tr(keys::json::COPY_PATH);
        let copy_value = self.tr(keys::json::COPY_VALUE);
        let show = self.tr(keys::json::SHOW_IN_CODE);
        if wide {
            if Button::new(&copy_path, "json-copy-path")
                .kind(super::Kind::Filled)
                .show(ui)
                .clicked()
            {
                ui.ctx().copy_text(selected.path.clone());
            }
            let kind = i18n::translate(
                self.language,
                kind_words(selected.kind),
                &[("count", selected.size.into())],
            );
            let said = ui.label(egui::RichText::new(kind).size(12.5).color(p.text_1));
            address(ui, &said, "json-selected-kind");
            let previewed = ui.add(
                egui::Label::new(
                    egui::RichText::new(&selected.preview)
                        .size(12.0)
                        .family(FontFamily::Monospace)
                        .color(p.text_1),
                )
                .wrap(),
            );
            address(ui, &previewed, "json-selected-preview");
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if !wide
                && Button::new(&copy_path, "json-copy-path")
                    .kind(super::Kind::Filled)
                    .show(ui)
                    .clicked()
            {
                ui.ctx().copy_text(selected.path.clone());
            }
            if Button::new(&copy_value, "json-copy-value")
                .show(ui)
                .clicked()
            {
                let value = selected
                    .text
                    .clone()
                    .unwrap_or_else(|| utf16_slice(self.text, selected.span).to_owned());
                ui.ctx().copy_text(value);
            }
            if self.with_code && Button::new(&show, "json-show-in-code").show(ui).clicked() {
                output.show_in_code = Some(selected.span);
            }
        });
        if wide {
            ui.add_space(4.0);
            ui.painter().hline(
                ui.max_rect().x_range(),
                ui.cursor().top(),
                Stroke::new(1.0, p.line_soft),
            );
        }
    }

    fn document(&self, ui: &mut Ui) {
        let p = palette(ui.ctx());
        title(ui, &self.tr(keys::json::DOCUMENT));
        let stats = &self.view.stats;
        let size = size_words(stats.bytes, self.language);
        let rows = [
            (self.tr(keys::json::KEYS), stats.keys.to_string()),
            (self.tr(keys::json::DEPTH), stats.depth.to_string()),
            (self.tr(keys::json::SIZE), size),
        ];
        let figures = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (name, value) in &rows {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(name).size(12.5).color(p.text_2));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(value)
                                .size(12.5)
                                .family(FontFamily::Monospace)
                                .color(p.text_0),
                        );
                    });
                });
            }
        });
        let said: Vec<String> = rows
            .iter()
            .map(|(name, value)| format!("{name} {value}"))
            .collect();
        let said = said.join(" · ");
        figures
            .response
            .widget_info(|| WidgetInfo::labeled(Role::List, true, &said));
        address(ui, &figures.response, "json-stats");
        for notice in &self.view.notices {
            let words = match notice {
                JsonNotice::FirstDocumentOnly => self.tr(keys::json::notices::FIRST_DOCUMENT_ONLY),
            };
            note(ui, &words, p.amber_text, "json-notice");
        }
        let format_note = match self.format {
            StructureFormat::Json => None,
            StructureFormat::Yaml => Some(keys::json::format_notes::YAML),
            StructureFormat::Toml => Some(keys::json::format_notes::TOML),
            StructureFormat::Xml => Some(keys::json::format_notes::XML),
        };
        if let Some(key) = format_note {
            note(ui, &self.tr(key), p.text_2, "json-format-note");
        }
        note(
            ui,
            &self.tr(keys::json::LIMIT_NOTE),
            p.text_2,
            "json-limit-note",
        );
    }
}

fn title(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    ui.label(
        egui::RichText::new(words.to_uppercase())
            .size(11.0)
            .strong()
            .extra_letter_spacing(1.0)
            .color(p.text_2),
    );
}

fn note(ui: &mut Ui, words: &str, colour: Color32, author_id: &str) {
    let said = ui.add(egui::Label::new(egui::RichText::new(words).size(12.0).color(colour)).wrap());
    address(ui, &said, author_id);
}

/// The document's size, written the way the interface's language writes numbers.
fn size_words(bytes: u32, language: Language) -> String {
    let bytes = f64::from(bytes);
    let (key, value) = if bytes < 1024.0 {
        (keys::json::BYTES, bytes)
    } else if bytes < 1024.0 * 1024.0 {
        (keys::json::KILOBYTES, bytes / 1024.0)
    } else {
        (keys::json::MEGABYTES, bytes / 1024.0 / 1024.0)
    };
    let written = i18n::format::at_most(value, 1, language);
    i18n::translate(language, key, &[("value", written.into())])
}

#[cfg(test)]
mod tests;
