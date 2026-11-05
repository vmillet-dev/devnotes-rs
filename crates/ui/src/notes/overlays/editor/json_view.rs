//! A snippet whose language has a reader (the reference's `json-view` and `side-by-side`): the
//! band of Code / Graph / Tree, the layout and whether the draft parses, then the explorer over
//! the body, alone or beside the code. Side by side the two select in step: a node chosen selects
//! its text in the code without taking the keys, and the caret moved selects what it is in.

use std::ops::Range;

use egui::{Align, Layout as Flow, Rect, Ui, UiBuilder, pos2};
use engine::json::model::{JsonQuery, JsonView};
use engine::json::parse::Span;
use engine::json::read::StructureFormat;
use engine::notes::model::DisplayNote;
use i18n::{Language, keys};

use super::code::{CodeView, Shown};
use crate::action::Actions;
use crate::api;
use crate::notes::Editing;
use crate::state::AppState;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::json_explorer::{self, Explorer, ExplorerView, Mode};
use crate::widgets::{self, Button, Kind};

/// A draft typed into is explored once the typing pauses: the document is parsed whole.
const EXPLORE_AFTER: f64 = 0.25;
/// Neither pane is squeezed under this; under their sum and a margin, side by side does not fit.
const CODE_FLOOR: f32 = 240.0;
const EXPLORER_FLOOR: f32 = 360.0;
const SIDE_BY_SIDE_FROM: f32 = CODE_FLOOR + EXPLORER_FLOOR + 100.0;
/// The code's share of the width until the line is moved.
const CODE_SHARE: f32 = 0.4;

/// Side by side or one at a time, the code hidden or not, and where the line between them
/// stands: the application session's, written nowhere.
#[derive(Debug)]
pub(crate) struct Layout {
    side_by_side: bool,
    code_hidden: bool,
    share: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            side_by_side: false,
            code_hidden: false,
            share: CODE_SHARE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Code,
    Graph,
    Tree,
}

/// One snippet's exploring, for the opening it began in.
pub(super) struct Exploring {
    format: StructureFormat,
    tab: Tab,
    explorer: Explorer,
    /// When the draft is sent next; at once on opening.
    due: Option<f64>,
    /// The text last sent.
    text: String,
    /// What the core was last asked: a frame that asks the same asks nothing.
    asked: Option<JsonQuery>,
    /// What the caret selected: revealed again, it would move the selection under the typing.
    caret_path: Option<String>,
}

impl Exploring {
    /// For a snippet whose language has a reader.
    pub(super) fn of(note: &DisplayNote) -> Option<Self> {
        let format = api::json::explorable_format(note.language)?;
        Some(Self {
            format,
            tab: Tab::Code,
            explorer: Explorer::default(),
            due: Some(0.0),
            text: String::new(),
            asked: None,
            caret_path: None,
        })
    }

    /// The code is on screen, alone or beside the explorer.
    pub(super) fn shows_code(&self) -> bool {
        self.tab == Tab::Code
    }

    fn mode(&self) -> Option<Mode> {
        match self.tab {
            Tab::Code => None,
            Tab::Graph => Some(Mode::Graph),
            Tab::Tree => Some(Mode::Tree),
        }
    }
}

/// Another language picked, another reader, or none.
pub(super) fn follow(exploring: &mut Option<Exploring>, note: &DisplayNote) {
    let format = api::json::explorable_format(note.language);
    if exploring.as_ref().map(|each| each.format) != format {
        *exploring = Exploring::of(note);
    }
}

/// The band over the body.
pub(super) fn band(
    ui: &mut Ui,
    state: &AppState,
    session: u64,
    exploring: &mut Exploring,
    layout: &mut Layout,
    body_width: f32,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let fits = body_width >= SIDE_BY_SIDE_FROM;
    super::band(ui, egui::Margin::symmetric(20, 8), true, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tab, key, author_id) in [
                (Tab::Code, keys::json::tabs::CODE, "editor-view-code"),
                (Tab::Graph, keys::json::tabs::GRAPH, "editor-view-graph"),
                (Tab::Tree, keys::json::tabs::TREE, "editor-view-tree"),
            ] {
                let on = exploring.tab == tab;
                let response = Button::new(&tr(key), author_id)
                    .kind(Kind::Quiet)
                    .armed(on)
                    .show(ui);
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_selected(on));
                if response.clicked() {
                    exploring.tab = tab;
                }
            }
            ui.add_space(12.0);
            let hint = if fits {
                keys::json::side_by_side::HINT
            } else {
                keys::json::side_by_side::NARROW
            };
            let toggle = Button::new(&tr(keys::json::side_by_side::TOGGLE), "editor-side-by-side")
                .kind(Kind::Quiet)
                .armed(layout.side_by_side)
                .enabled(fits)
                .show(ui);
            if widgets::tooltip(toggle, &tr(hint)).clicked() {
                layout.side_by_side = !layout.side_by_side;
                if layout.side_by_side && exploring.tab == Tab::Code {
                    exploring.tab = Tab::Graph;
                }
            }
            if layout.side_by_side && fits && exploring.tab != Tab::Code {
                let hide =
                    Button::new(&tr(keys::json::side_by_side::HIDE_CODE), "editor-hide-code")
                        .kind(Kind::Quiet)
                        .armed(layout.code_hidden)
                        .show(ui);
                if hide.clicked() {
                    layout.code_hidden = !layout.code_hidden;
                }
            }
            if layout.side_by_side && !fits {
                let p = palette(ui.ctx());
                let said = ui.label(
                    egui::RichText::new(tr(keys::json::side_by_side::NARROW))
                        .size(11.5)
                        .color(p.text_2),
                );
                widgets::address(ui, &said, "editor-side-by-side-narrow");
            }
            if let Some((view, _)) = state.explored.of(session) {
                ui.with_layout(Flow::right_to_left(Align::Center), |ui| {
                    validity(ui, view, exploring.format, language);
                });
            }
        });
    });
}

/// « ✓ JSON valide », or where the draft stops parsing.
fn validity(ui: &mut Ui, view: &JsonView, format: StructureFormat, language: Language) {
    let p = palette(ui.ctx());
    let name = json_explorer::format_name(format);
    let (mark, words, colour) = match &view.error {
        None => (
            Icon::Check,
            i18n::translate(language, keys::json::VALID, &[("format", name.into())]),
            p.green,
        ),
        Some(error) => (
            Icon::X,
            i18n::translate(
                language,
                keys::json::INVALID,
                &[("format", name.into()), ("line", error.line.into())],
            ),
            p.red,
        ),
    };
    let said = ui.label(egui::RichText::new(words).size(11.5).color(colour));
    let (glyph, _) = ui.allocate_exact_size(egui::Vec2::splat(14.0), egui::Sense::hover());
    icons::paint(ui, glyph, mark, colour);
    let invalid = view.error.is_some();
    ui.ctx().accesskit_node_builder(said.id, |node| {
        if invalid {
            node.set_invalid(egui::accesskit::Invalid::True);
        }
    });
    widgets::address(ui, &said, "editor-json-validity");
}

/// The body of an explorable snippet: the code, the explorer, or both.
pub(super) fn body(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    exploring: &mut Exploring,
    code: &mut CodeView,
    layout: &mut Layout,
    actions: &mut Actions,
) -> Shown {
    let now = ui.input(|input| input.time);
    let rect = ui.max_rect();
    let fits = rect.width() >= SIDE_BY_SIDE_FROM;
    let mode = exploring.mode();
    let beside = mode.is_some() && layout.side_by_side && fits && !layout.code_hidden;
    let explored = state.explored.of(session);
    let mut shown = Shown {
        changed: false,
        left: false,
    };
    match mode {
        None => {
            let drawn = super::code::show(ui, code, note.language, state.language);
            (shown.changed, shown.left) = (drawn.changed, drawn.left);
        }
        Some(mode) if beside => {
            let split = rect.left() + (rect.width() * layout.share).max(CODE_FLOOR);
            let split = split.min(rect.right() - EXPLORER_FLOOR);
            let code_rect = Rect::from_min_max(rect.min, pos2(split - 4.0, rect.bottom()));
            let mut code_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("side-by-side-code")
                    .max_rect(code_rect),
            );
            let pane = widgets::region(
                &mut code_ui,
                code_rect,
                "side-by-side-code",
                egui::Sense::hover(),
            );
            widgets::address(&code_ui, &pane, "side-by-side-code");
            let before = code.editor.buffer.selection().head;
            code.lit = lit_lines(code, explored, &exploring.explorer);
            let drawn = super::code::show(&mut code_ui, code, note.language, state.language);
            (shown.changed, shown.left) = (drawn.changed, drawn.left);
            if code.editor.buffer.selection().head != before && !drawn.changed {
                caret_moved(exploring, code, explored);
            }
            let handle_rect = Rect::from_min_max(
                pos2(split - 4.0, rect.top()),
                pos2(split + 4.0, rect.bottom()),
            );
            let mut handle_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("side-by-side-handle")
                    .max_rect(handle_rect),
            );
            let moved = widgets::split_handle(&mut handle_ui, "side-by-side-handle");
            if moved != 0.0 {
                layout.share = ((split + moved - rect.left()) / rect.width()).clamp(0.1, 0.9);
            }
            let explorer_rect = Rect::from_min_max(pos2(split + 4.0, rect.top()), rect.max);
            let mut explorer_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("side-by-side")
                    .max_rect(explorer_rect),
            );
            let pane = widgets::region(
                &mut explorer_ui,
                explorer_rect,
                "side-by-side",
                egui::Sense::hover(),
            );
            widgets::address(&explorer_ui, &pane, "side-by-side");
            explore(
                &mut explorer_ui,
                state,
                exploring,
                code,
                explored,
                (mode, true),
            );
        }
        Some(mode) => {
            code.lit = 0..0;
            explore(ui, state, exploring, code, explored, (mode, false));
        }
    }
    ui.advance_cursor_after_rect(rect);
    if shown.changed {
        exploring.due = Some(now + EXPLORE_AFTER);
    }
    ask(ui, session, exploring, code, actions);
    shown
}

/// The draft once due, and every change of what the explorer asks.
fn ask(ui: &Ui, session: u64, exploring: &mut Exploring, code: &CodeView, actions: &mut Actions) {
    let now = ui.input(|input| input.time);
    if let Some(due) = exploring.due {
        if now >= due {
            exploring.due = None;
            exploring.text = code.text();
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(due - now));
        }
    }
    let query = exploring
        .explorer
        .query(exploring.text.clone(), exploring.format);
    if exploring.asked.as_ref() != Some(&query) {
        exploring.asked = Some(query.clone());
        actions.push(super::editor(Editing::Explore { session, query }));
    }
}

fn explore(
    ui: &mut Ui,
    state: &AppState,
    exploring: &mut Exploring,
    code: &mut CodeView,
    answer: Option<(&JsonView, &str)>,
    (mode, beside): (Mode, bool),
) {
    let Some((view, text)) = answer else {
        return;
    };
    let output = ExplorerView::new(&mut exploring.explorer, view, text)
        .mode(mode)
        .format(exploring.format)
        .language(state.language)
        .show(ui);
    if output.tree_proposed {
        exploring.tab = Tab::Tree;
    }
    if let Some(span) = output.show_in_code {
        if !beside {
            exploring.tab = Tab::Code;
        }
        code.editor.select(chars(code, span));
        code.editor.focus();
    }
    if beside
        && let Some(path) = output.chosen
        && exploring.caret_path.as_ref() != Some(&path)
    {
        exploring.caret_path = None;
        if let Some(span) = json_explorer::span_of(view, &path) {
            code.editor.select(chars(code, span));
        }
    }
}

/// The span's UTF-16 units as the code's characters, within the text as it is now.
fn chars(code: &CodeView, span: Span) -> Range<usize> {
    let rope = code.editor.buffer.rope();
    let units = rope.len_utf16_cu();
    let at = |unit: u32| rope.utf16_cu_to_char(usize::try_from(unit).unwrap_or(units).min(units));
    at(span.start)..at(span.end)
}

/// The lines of the explorer's selection, lit in the code beside it.
fn lit_lines(
    code: &CodeView,
    answer: Option<(&JsonView, &str)>,
    explorer: &Explorer,
) -> Range<usize> {
    let Some((view, _)) = answer else {
        return 0..0;
    };
    let span = explorer
        .selected_path(view)
        .and_then(|path| json_explorer::span_of(view, &path));
    let Some(span) = span else {
        return 0..0;
    };
    let range = chars(code, span);
    let buffer = &code.editor.buffer;
    buffer.line_of(range.start)..buffer.line_of(range.end) + 1
}

/// The caret moved by a key or a click: what it is in is selected.
fn caret_moved(exploring: &mut Exploring, code: &CodeView, answer: Option<(&JsonView, &str)>) {
    let Some((view, _)) = answer else {
        return;
    };
    let rope = code.editor.buffer.rope();
    let head = code.editor.buffer.selection().head.min(rope.len_chars());
    let offset = u32::try_from(rope.char_to_utf16_cu(head)).unwrap_or(u32::MAX);
    let Some(path) = json_explorer::entry_at(view, offset) else {
        return;
    };
    if exploring.explorer.selected_path(view).as_deref() == Some(path) {
        return;
    }
    exploring.caret_path = Some(path.to_owned());
    exploring.explorer.select(path);
}

#[cfg(test)]
mod tests;
