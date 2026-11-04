//! The same document as an indented tree (the reference's `json-tree`): the view for a large
//! one, a line per value, open or not by the same set as the graph's nodes. The list is the one
//! tab stop; the arrows walk it, → and ← open and close.

use egui::{
    CornerRadius, FontFamily, Key, Modifiers, Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2,
};
use engine::json::model::{JsonKind, JsonLine, JsonView};
use i18n::{Language, keys};

use super::graph::describe;
use super::{Explorer, kind_colour};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, address, text};

const LINE: f32 = 24.0;
const FONT: f32 = 12.0;
/// Room for the tools drawn over the top of the list.
const TOOLS_ROOM: f32 = 56.0;
const INDENT: f32 = 18.0;

/// What the tree keeps between frames.
#[derive(Debug, Default)]
pub(super) struct Lines {
    /// The scroll and the height last drawn, to bring the cursor into view.
    offset: f32,
    height: f32,
    pub(super) reveal: bool,
    pub(super) refocus: bool,
}

/// The lines; the path picked this frame, if any.
pub(super) fn show(
    ui: &mut Ui,
    explorer: &mut Explorer,
    view: &JsonView,
    language: Language,
) -> Option<String> {
    let lines = &view.tree;
    let selected = explorer.selected_path(view);
    let cursor = lines
        .iter()
        .position(|line| Some(&line.path) == selected.as_ref())
        .unwrap_or(0);
    let rect = ui.max_rect();
    let list = widgets::region(ui, rect, "json-tree", Sense::focusable_noninteractive());
    list.widget_info(|| {
        WidgetInfo::labeled(
            Role::Tree,
            true,
            i18n::translate(language, keys::json::TREE, &[]),
        )
    });
    address(ui, &list, "json-tree");
    let p = palette(ui.ctx());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.bg_0);

    let mut area = egui::ScrollArea::vertical()
        .id_salt("json-tree-scroll")
        .auto_shrink([false, false]);
    if std::mem::take(&mut explorer.tree.reveal) {
        let top = LINE * index_f32(cursor);
        let lines = &explorer.tree;
        if top < lines.offset {
            area = area.vertical_scroll_offset(top);
        } else if top + LINE > lines.offset + lines.height {
            area = area.vertical_scroll_offset(top + LINE - lines.height);
        }
    }
    let mut picked = None;
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("json-tree-rows")
            .max_rect(rect.with_min_y(rect.top() + TOOLS_ROOM)),
    );
    let shown = area.show_rows(
        &mut inner,
        LINE,
        lines.len() + usize::from(view.tree_truncated),
        |ui, range| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for index in range {
                let Some(line) = lines.get(index) else {
                    let note = i18n::translate(language, keys::json::TREE_TRUNCATED, &[]);
                    ui.label(
                        egui::RichText::new(note)
                            .size(12.0)
                            .italics()
                            .color(p.text_2),
                    );
                    continue;
                };
                let response = draw_line(ui, line, index == cursor);
                if response.clicked() {
                    picked = Some(choose(explorer, view, line));
                    list.request_focus();
                }
            }
        },
    );
    explorer.tree.offset = shown.state.clamped_offset().y;
    explorer.tree.height = shown.inner_rect.height();

    if std::mem::take(&mut explorer.tree.refocus) {
        list.request_focus();
    }
    if widgets::takes_keys(ui, &list)
        && let Some(path) = keys_of(ui, explorer, view, cursor)
    {
        picked = Some(path);
        list.request_focus();
        explorer.tree.reveal = true;
    }
    picked
}

fn index_f32(index: usize) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(u16::MAX))
}

fn draw_line(ui: &mut Ui, line: &JsonLine, current: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), LINE), Sense::CLICK);
    let words = format!("{} {}", line.key, line.value);
    let (opens, open) = (line.opens, line.open);
    response.widget_info(|| WidgetInfo::labeled(Role::TreeItem, true, &words));
    address(ui, &response, "json-line");
    describe(ui, &response, &line.path, opens.then_some(open));
    let fill = if line.hit {
        Some(p.amber_fill.gamma_multiply(0.18))
    } else if current {
        Some(p.amber_fill.gamma_multiply(0.1))
    } else if response.hovered() {
        Some(p.bg_2)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    }
    let depth = f32::from(u16::try_from(line.depth).unwrap_or(u16::MAX));
    let mut x = rect.left() + 12.0 + depth * INDENT;
    let middle = rect.center().y;
    let twist = match (line.opens, line.open) {
        (false, _) => None,
        (true, true) => Some(Icon::ChevronDown),
        (true, false) => Some(Icon::ChevronRight),
    };
    if let Some(icon) = twist {
        let glyph = egui::Rect::from_center_size(pos2(x + 6.0, middle), vec2(12.0, 12.0));
        icons::paint(ui, glyph, icon, p.text_2);
    }
    x += 18.0;
    let mut paint = |words: &str, colour, gap: f32| {
        let galley = text(ui, words, FONT, 400.0, FontFamily::Monospace, colour);
        let width = galley.size().x;
        ui.painter().with_clip_rect(rect).galley(
            pos2(x, middle - galley.size().y / 2.0),
            galley,
            colour,
        );
        x += width + gap;
    };
    paint(&line.key, p.text_1, 10.0);
    if line.opens {
        let braces = if line.kind == JsonKind::Object {
            "{ }"
        } else {
            "[ ]"
        };
        paint(&format!("{braces} {}", line.size), p.text_2, 0.0);
    } else {
        paint(&line.value, kind_colour(p, line.kind), 0.0);
    }
    response
}

/// A container opens or closes; a value is selected.
fn choose(explorer: &mut Explorer, view: &JsonView, line: &JsonLine) -> String {
    explorer.toggle(view, &line.path, line.opens, line.open);
    explorer.tree.refocus = true;
    line.path.clone()
}

fn keys_of(ui: &Ui, explorer: &mut Explorer, view: &JsonView, cursor: usize) -> Option<String> {
    if super::fold_keys(ui, explorer, view) {
        explorer.tree.refocus = true;
        return None;
    }
    let lines = &view.tree;
    let line = lines.get(cursor)?;
    let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    let at = if pressed(Key::ArrowDown) {
        (cursor + 1).min(lines.len() - 1)
    } else if pressed(Key::ArrowUp) {
        cursor.saturating_sub(1)
    } else if pressed(Key::ArrowRight) {
        if line.opens && !line.open {
            return Some(choose(explorer, view, line));
        }
        if !line.open {
            return None;
        }
        cursor + 1
    } else if pressed(Key::ArrowLeft) {
        if line.open && cursor > 0 {
            return Some(choose(explorer, view, line));
        }
        parent_of(lines, cursor)
    } else if pressed(Key::Enter) || pressed(Key::Space) {
        return Some(choose(explorer, view, line));
    } else {
        return None;
    };
    let line = lines.get(at)?;
    explorer.select(&line.path);
    Some(line.path.clone())
}

fn parent_of(lines: &[JsonLine], index: usize) -> usize {
    let depth = lines[index].depth;
    lines[..index]
        .iter()
        .rposition(|line| line.depth < depth)
        .unwrap_or(index)
}
