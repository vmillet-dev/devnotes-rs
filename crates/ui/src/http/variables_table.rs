//! A table of variables: a request's, and later the environments' and the globals'. A row to tick,
//! its name, value and kind; one blank row at the end becomes a row once typed in. A secret stays
//! masked until asked for, and what cannot be used is said under the table.

use std::collections::HashSet;

use egui::{Id, Rect, Stroke, Ui, UiBuilder, Vec2, pos2};
use engine::http::environments::{FilledBy, Variable, VariableFault, VariableKind};
use i18n::keys;

use super::request::table::{self, CHECK, REMOVE, ROW};
use crate::api;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, IconButton};

const KIND: f32 = 120.0;

/// The rows whose secret is shown, for as long as the table is.
pub(crate) type Revealed = HashSet<usize>;

/// What names a table.
pub(crate) struct Table<'a> {
    /// `request`, `environment`: its address is `http-<kind>-variables`.
    pub(crate) kind: &'a str,
    pub(crate) label: &'a str,
}

/// What a frame did to the table.
pub(crate) enum Changed {
    Rows(Vec<Variable>),
    /// The request that fills a variable, asked to open.
    Filler(FilledBy),
}

enum Row {
    Edited(Variable),
    Removed,
    Filler(FilledBy),
}

struct Columns {
    check: Rect,
    name: Rect,
    value: Rect,
    kind: Rect,
    remove: Rect,
}

impl Columns {
    fn of(frame: Rect) -> Self {
        let inner = (frame.width() - CHECK - KIND - REMOVE).max(120.0);
        let name = inner * 0.4;
        let column = |from: f32, width: f32| {
            Rect::from_min_max(
                pos2(frame.left() + from, frame.top()),
                pos2(frame.left() + from + width, frame.bottom()),
            )
        };
        Self {
            check: column(0.0, CHECK),
            name: column(CHECK, name),
            value: column(CHECK + name, inner - name),
            kind: column(CHECK + inner, KIND),
            remove: column(CHECK + inner + KIND, REMOVE),
        }
    }

    fn at(column: Rect, top: f32) -> Rect {
        Rect::from_min_max(pos2(column.left(), top), pos2(column.right(), top + ROW))
    }
}

pub(crate) fn show(
    ui: &mut Ui,
    table: &Table<'_>,
    rows: &[Variable],
    revealed: &mut Revealed,
    language: i18n::Language,
) -> Option<Changed> {
    let p = palette(ui.ctx());
    let id = ui.scope_id().with(("http-variables", table.kind));
    let address = format!("http-{}-variables", table.kind);
    let frame = table::frame(ui, rows.len(), table.label, &address);
    let columns = Columns::of(frame);
    for (column, key) in [
        (columns.name, keys::http::variables::NAME),
        (columns.value, keys::http::variables::VALUE),
        (columns.kind, keys::http::variables::KIND),
    ] {
        table::heading(ui, Columns::at(column, frame.top()), key, language);
    }
    let faults = api::http::variable_faults(rows);
    let mut changed = None;
    for at in 0..=rows.len() {
        #[expect(clippy::cast_precision_loss, reason = "rows of a table")]
        let top = frame.top() + (at + 1) as f32 * ROW;
        ui.painter()
            .hline(frame.x_range(), top + 0.5, Stroke::new(1.0, p.line_soft));
        let cell = Cell {
            id: id.with(at),
            top,
            at,
            blank: at == rows.len(),
            fault: faults.get(at).copied().flatten(),
        };
        let row = rows.get(at).cloned().unwrap_or_default();
        if let Some(row) = row_of(ui, &columns, &cell, row, revealed, language) {
            changed = Some(applied(rows, at, row, revealed));
        }
    }
    said(ui, &faults, language);
    changed
}

fn applied(rows: &[Variable], at: usize, row: Row, revealed: &mut Revealed) -> Changed {
    let mut all = rows.to_vec();
    match row {
        Row::Edited(row) if at == rows.len() => all.push(row),
        Row::Edited(row) => all[at] = row,
        Row::Removed => {
            all.remove(at);
            revealed.clear();
        }
        Row::Filler(filler) => return Changed::Filler(filler),
    }
    Changed::Rows(all)
}

struct Cell {
    id: Id,
    top: f32,
    at: usize,
    blank: bool,
    fault: Option<VariableFault>,
}

fn row_of(
    ui: &mut Ui,
    columns: &Columns,
    cell: &Cell,
    mut row: Variable,
    revealed: &mut Revealed,
    language: i18n::Language,
) -> Option<Row> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut changed = false;
    if !cell.blank {
        let label = i18n::translate(
            language,
            keys::http::table::ENABLE,
            &[("key", row.name.as_str().into())],
        );
        let rect = Columns::at(columns.check, cell.top);
        changed |= table::tick(ui, rect, &mut row.enabled, &label, "http-variable-enabled");
    }
    let ink = if row.enabled || cell.blank {
        p.text_0
    } else {
        p.text_2
    };
    let faulty_name = matches!(
        cell.fault,
        Some(VariableFault::Name | VariableFault::Duplicate)
    );
    let hint = if cell.blank {
        tr(keys::http::variables::NEW_NAME)
    } else {
        String::new()
    };
    changed |= table::text_cell(
        ui,
        (Columns::at(columns.name, cell.top), cell.id.with("name")),
        (
            &mut row.name,
            &tr(keys::http::variables::NAME),
            "http-variable-name",
        ),
        (&hint, if faulty_name { p.red } else { ink }, true, false),
    )
    .changed();
    let secret = row.kind == VariableKind::Secret && !cell.blank;
    let hidden = secret && !revealed.contains(&cell.at);
    let mut value_rect = Columns::at(columns.value, cell.top);
    if secret {
        let eye = value_rect
            .split_left_right_at_x(value_rect.right() - 28.0)
            .1;
        value_rect.set_right(eye.left());
        reveal(ui, eye, (&row.name, cell.at), revealed, language);
    }
    if let Some(filled) = row.filled_by.clone()
        && filled_by(ui, &mut value_rect, &filled, language)
    {
        return Some(Row::Filler(filled));
    }
    let faulty_value = cell.fault == Some(VariableFault::Number);
    changed |= table::text_cell(
        ui,
        (value_rect, cell.id.with("value")),
        (
            &mut row.value,
            &tr(keys::http::variables::VALUE),
            "http-variable-value",
        ),
        ("", if faulty_value { p.red } else { ink }, true, hidden),
    )
    .changed();
    changed |= kind_cell(ui, columns, cell, &mut row, language);
    let rect = Columns::at(columns.remove, cell.top);
    if !cell.blank && table::remove(ui, rect, (&row.name, "http-variable-remove"), language) {
        return Some(Row::Removed);
    }
    changed.then_some(Row::Edited(row))
}

fn reveal(
    ui: &mut Ui,
    rect: Rect,
    (name, at): (&str, usize),
    revealed: &mut Revealed,
    language: i18n::Language,
) {
    let shown = revealed.contains(&at);
    let (key, icon) = if shown {
        (keys::http::variables::HIDE, Icon::EyeOff)
    } else {
        (keys::http::variables::REVEAL, Icon::Eye)
    };
    let label = i18n::translate(language, key, &[("name", name.into())]);
    let mut over = ui.new_child(UiBuilder::new().max_rect(rect));
    let button = IconButton::new(icon, &label, "http-variable-reveal").pressed(shown);
    if button.show(&mut over).clicked() && !revealed.remove(&at) {
        revealed.insert(at);
    }
}

/// « filled by Login », at the end of the value: a click opens that request. `true` on a click.
fn filled_by(ui: &mut Ui, value: &mut Rect, filled: &FilledBy, language: i18n::Language) -> bool {
    let label = i18n::translate(
        language,
        keys::http::variables::FILLED_BY,
        &[("name", filled.name.as_str().into())],
    );
    let width = Button::width(ui, &label);
    let rect = Rect::from_min_max(pos2(value.right() - width, value.top()), value.max);
    value.set_right(rect.left());
    let mut over = ui.new_child(UiBuilder::new().max_rect(rect));
    over.spacing_mut().interact_size = Vec2::new(width, ROW);
    Button::new(&label, "http-variable-filled")
        .enabled(filled.request_id.is_some())
        .show(&mut over)
        .clicked()
}

fn kind_cell(
    ui: &mut Ui,
    columns: &Columns,
    cell: &Cell,
    row: &mut Variable,
    language: i18n::Language,
) -> bool {
    let tr = |key| i18n::translate(language, key, &[]);
    let names = [
        tr(keys::http::variables::kinds::TEXT),
        tr(keys::http::variables::kinds::NUMBER),
        tr(keys::http::variables::kinds::SECRET),
    ];
    let options = [
        (VariableKind::Text, names[0].as_str()),
        (VariableKind::Number, names[1].as_str()),
        (VariableKind::Secret, names[2].as_str()),
    ];
    let label = i18n::translate(
        language,
        keys::http::variables::KIND_OF,
        &[("name", row.name.as_str().into())],
    );
    let rect = Columns::at(columns.kind, cell.top).shrink2(Vec2::new(8.0, 4.0));
    let mut over = ui.new_child(
        UiBuilder::new()
            .scope_id(cell.id.with("kind"))
            .max_rect(rect),
    );
    ChoiceMenu::new(&label, "http-variable-kind", &options)
        .show(&mut over, &mut row.kind)
        .changed()
}

/// The sentences under the table: one a fault, however many rows carry it.
fn said(ui: &mut Ui, faults: &[Option<VariableFault>], language: i18n::Language) {
    let p = palette(ui.ctx());
    for (fault, key) in [
        (VariableFault::Name, keys::http::variables::faults::NAME),
        (
            VariableFault::Duplicate,
            keys::http::variables::faults::DUPLICATE,
        ),
        (VariableFault::Number, keys::http::variables::faults::NUMBER),
    ] {
        if !faults.contains(&Some(fault)) {
            continue;
        }
        let words = egui::RichText::new(i18n::translate(language, key, &[]))
            .size(12.0)
            .color(p.red);
        let shown = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &shown, "http-variable-fault");
    }
}
