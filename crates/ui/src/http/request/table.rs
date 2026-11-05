//! A request's parameters or headers: a row to tick, its key, value and description, and one
//! blank row at the end that becomes a row as soon as something is typed in it.
//!
//! As headers, a row that cannot be sent is marked with why, and `Name: value` typed or pasted in
//! a name is split into its two cells. Both are Rust's to say: a parameter takes any character.

use egui::{
    Color32, CornerRadius, Event, FontFamily, FontId, Id, Margin, Rect, Role, Sense, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, pos2,
};
use engine::http::model::{HeaderFault, KeyValue};
use i18n::keys;

use crate::api;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Checkbox, MIN_TARGET, interact};

const ROW: f32 = 32.0;
const CHECK: f32 = 36.0;
const REMOVE: f32 = 32.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Key,
    Value,
    Description,
}

/// What names a table and its rows.
pub(crate) struct Table<'a> {
    /// `params`, `headers`: the table's address is `http-<kind>-table`.
    pub(crate) kind: &'a str,
    pub(crate) label: &'a str,
    /// The rows are headers: their names are HTTP tokens, and a name may carry its value.
    pub(crate) headers: bool,
}

/// The rows as edited this frame, when they changed.
pub(crate) fn show(
    ui: &mut Ui,
    table: &Table<'_>,
    rows: &[KeyValue],
    language: i18n::Language,
) -> Option<Vec<KeyValue>> {
    let p = palette(ui.ctx());
    let id = ui.scope_id().with(("http-table", table.kind));
    let faults = if table.headers {
        api::http::header_faults(rows)
    } else {
        Vec::new()
    };
    let mut edited: Option<Vec<KeyValue>> = None;
    let width = ui.available_width();
    #[expect(clippy::cast_precision_loss, reason = "rows of a table")]
    let height = (rows.len() + 2) as f32 * ROW;
    let (frame, region) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    region.widget_info(|| WidgetInfo::labeled(Role::Table, true, table.label));
    widgets::address(ui, &region, &format!("http-{}-table", table.kind));
    let corner = CornerRadius::same(10);
    ui.painter().rect(
        frame,
        corner,
        p.bg_1,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let columns = Columns::of(frame);
    head(ui, &columns, frame.top(), language);
    for at in 0..=rows.len() {
        let blank = at == rows.len();
        let row = rows.get(at).cloned().unwrap_or_default();
        #[expect(clippy::cast_precision_loss, reason = "rows of a table")]
        let top = frame.top() + (at + 1) as f32 * ROW;
        ui.painter()
            .hline(frame.x_range(), top + 0.5, Stroke::new(1.0, p.line_soft));
        let fault = faults.get(at).copied().flatten();
        let cell = Cell {
            id: id.with(at),
            top,
            blank,
            fault,
            headers: table.headers,
        };
        if let Some(changed) = row_of(ui, &columns, &cell, row, language) {
            let mut all = edited.take().unwrap_or_else(|| rows.to_vec());
            match changed {
                Changed::Row(row) if blank => all.push(row),
                Changed::Row(row) => all[at] = row,
                Changed::Removed => {
                    all.remove(at);
                }
                Changed::Pasted(read) => {
                    // A row already written keeps its place: the lines pasted go after it.
                    let kept = !blank && (!all[at].key.is_empty() || !all[at].value.is_empty());
                    let from = if kept { at + 1 } else { at };
                    let to = if blank || kept { from } else { at + 1 };
                    all.splice(from..to.min(all.len()), read);
                }
            }
            edited = Some(all);
        }
    }
    said(ui, &faults, language);
    if table.headers
        && let Some(after) = &edited
        && let Some((split, at)) = split_headers(rows, after)
    {
        // The caret goes with the value, as a pasted header's does.
        ui.memory_mut(|memory| memory.request_focus(id.with(at).with(Field::Value as u8)));
        edited = Some(split);
    }
    edited
}

/// Where each column stands in the table's frame.
struct Columns {
    check: Rect,
    key: Rect,
    value: Rect,
    description: Rect,
    remove: Rect,
}

impl Columns {
    fn of(frame: Rect) -> Self {
        let inner = (frame.width() - CHECK - REMOVE).max(90.0);
        let third = inner / 3.0;
        let x = |offset: f32| frame.left() + offset;
        let column = |from: f32, width: f32| {
            Rect::from_min_max(
                pos2(x(from), frame.top()),
                pos2(x(from + width), frame.bottom()),
            )
        };
        Self {
            check: column(0.0, CHECK),
            key: column(CHECK, third),
            value: column(CHECK + third, third),
            description: column(CHECK + 2.0 * third, third),
            remove: column(CHECK + 3.0 * third, REMOVE),
        }
    }

    fn at(column: Rect, top: f32) -> Rect {
        Rect::from_min_max(pos2(column.left(), top), pos2(column.right(), top + ROW))
    }
}

fn head(ui: &Ui, columns: &Columns, top: f32, language: i18n::Language) {
    let p = palette(ui.ctx());
    for (column, key) in [
        (columns.key, keys::http::table::KEY),
        (columns.value, keys::http::table::VALUE),
        (columns.description, keys::http::table::DESCRIPTION),
    ] {
        let words = i18n::translate(language, key, &[]);
        let galley = widgets::text(ui, &words, 11.5, 600.0, FontFamily::Proportional, p.text_2);
        let rect = Columns::at(column, top);
        ui.painter().galley(
            pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0),
            galley,
            p.text_2,
        );
    }
}

struct Cell {
    id: Id,
    top: f32,
    blank: bool,
    fault: Option<HeaderFault>,
    headers: bool,
}

enum Changed {
    Row(KeyValue),
    Removed,
    /// Lines pasted in a name, a row each.
    Pasted(Vec<KeyValue>),
}

fn row_of(
    ui: &mut Ui,
    columns: &Columns,
    cell: &Cell,
    mut row: KeyValue,
    language: i18n::Language,
) -> Option<Changed> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut changed = false;
    if !cell.blank {
        let rect = Columns::at(columns.check, cell.top);
        let mut check_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        ));
        let label = i18n::translate(
            language,
            keys::http::table::ENABLE,
            &[("key", row.key.as_str().into())],
        );
        changed |= Checkbox::new(&mut row.enabled, &label, "http-kv-enabled")
            .show(&mut check_ui)
            .changed();
    }
    let ink = if row.enabled || cell.blank {
        p.text_0
    } else {
        p.text_2
    };
    if let Some(pasted) = pasted_lines(ui, cell.id.with(Field::Key as u8)) {
        return Some(Changed::Pasted(pasted));
    }
    let hint = if cell.blank {
        tr(keys::http::table::NEW_KEY)
    } else {
        String::new()
    };
    let key_ink = if cell.fault == Some(HeaderFault::Name) {
        p.red
    } else {
        ink
    };
    let key = text_cell(
        ui,
        (
            Columns::at(columns.key, cell.top),
            cell.id.with(Field::Key as u8),
        ),
        (&mut row.key, &tr(keys::http::table::KEY), "http-kv-key"),
        (&hint, key_ink, true),
    );
    changed |= key.changed();
    let was_empty = row.value.is_empty();
    let value_ink = if cell.fault == Some(HeaderFault::Value) {
        p.red
    } else {
        ink
    };
    let value = text_cell(
        ui,
        (
            Columns::at(columns.value, cell.top),
            cell.id.with(Field::Value as u8),
        ),
        (
            &mut row.value,
            &tr(keys::http::table::VALUE),
            "http-kv-value",
        ),
        ("", value_ink, true),
    );
    // The space typed after `Name:` lands here once the split has moved the caret: a header's
    // value starts at its first character, as a pasted one does.
    if cell.headers && was_empty && row.value.starts_with(char::is_whitespace) {
        row.value = row.value.trim_start().to_string();
    }
    changed |= value.changed();
    let description = text_cell(
        ui,
        (
            Columns::at(columns.description, cell.top),
            cell.id.with(Field::Description as u8),
        ),
        (
            &mut row.description,
            &tr(keys::http::table::DESCRIPTION),
            "http-kv-description",
        ),
        ("", ink, false),
    );
    changed |= description.changed();
    if !cell.blank && remove(ui, Columns::at(columns.remove, cell.top), &row, language) {
        return Some(Changed::Removed);
    }
    changed.then_some(Changed::Row(row))
}

fn text_cell(
    ui: &mut Ui,
    (rect, id): (Rect, Id),
    (text, name, author_id): (&mut String, &str, &str),
    (hint, ink, mono): (&str, Color32, bool),
) -> egui::Response {
    let family = if mono {
        FontFamily::Monospace
    } else {
        FontFamily::Proportional
    };
    let mut cell_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(4.0, 2.0))));
    let response = cell_ui.add_sized(
        cell_ui.available_size(),
        TextEdit::singleline(text)
            .id(id)
            .frame(egui::Frame::new())
            .margin(Margin::symmetric(8, 0))
            .align(egui::Align2::LEFT_CENTER)
            .hint_text(hint)
            .text_color(ink)
            .font(FontId::new(12.5, family)),
    );
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.to_string()));
    widgets::address(ui, &response, author_id);
    response
}

/// Lines pasted in a name — a browser's network tab copies headers so — fill a row each: the
/// field alone would take them on one line, the names lost in the first one's value.
fn pasted_lines(ui: &Ui, id: Id) -> Option<Vec<KeyValue>> {
    if !ui.memory(|memory| memory.has_focus(id)) {
        return None;
    }
    let text = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            Event::Paste(text) if text.trim().contains(['\n', '\r']) => Some(text.clone()),
            _ => None,
        })
    })?;
    let read = api::http::read_headers(&text);
    if read.is_empty() {
        return None;
    }
    ui.input_mut(|input| {
        input
            .events
            .retain(|event| !matches!(event, Event::Paste(_)));
    });
    Some(read)
}

fn remove(ui: &mut Ui, rect: Rect, row: &KeyValue, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let label = i18n::translate(
        language,
        keys::http::table::REMOVE_ROW,
        &[("key", row.key.as_str().into())],
    );
    let square = Rect::from_center_size(rect.center(), Vec2::splat(MIN_TARGET));
    let mut over = ui.new_child(egui::UiBuilder::new().max_rect(square));
    let response = interact(
        &mut over,
        square.size(),
        Sense::click(),
        "http-kv-remove",
        || WidgetInfo::labeled(Role::Button, true, &label),
        CornerRadius::same(5),
    );
    let ink = if response.hovered() {
        ui.painter()
            .rect_filled(square, CornerRadius::same(5), p.bg_3);
        p.text_0
    } else {
        p.text_2
    };
    icons::paint(
        ui,
        Rect::from_center_size(square.center(), Vec2::splat(14.0)),
        Icon::X,
        ink,
    );
    response.clicked()
}

/// The sentences under the table: one a fault, however many rows carry it.
fn said(ui: &mut Ui, faults: &[Option<HeaderFault>], language: i18n::Language) {
    let p = palette(ui.ctx());
    for (fault, key) in [
        (HeaderFault::Name, keys::http::table::faults::NAME),
        (HeaderFault::Value, keys::http::table::faults::VALUE),
    ] {
        if !faults.contains(&Some(fault)) {
            continue;
        }
        let words = egui::RichText::new(i18n::translate(language, key, &[]))
            .size(12.0)
            .color(p.red);
        let response = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &response, "http-kv-fault");
    }
}

/// `Name: value` in a header's name is what was meant as a name and its value: what follows the
/// first colon goes to the value, when there is none yet. `None` when nothing is to be split.
pub(crate) fn split_headers(
    before: &[KeyValue],
    after: &[KeyValue],
) -> Option<(Vec<KeyValue>, usize)> {
    let at = after
        .iter()
        .enumerate()
        .position(|(at, row)| before.get(at).is_none_or(|was| was.key != row.key))?;
    let row = &after[at];
    if !row.key.contains(':') || !row.value.is_empty() {
        return None;
    }
    let read = api::http::read_headers(&row.key).into_iter().next()?;
    let mut rows = after.to_vec();
    rows[at] = KeyValue {
        enabled: row.enabled,
        key: read.key,
        value: read.value,
        description: row.description.clone(),
    };
    Some((rows, at))
}
