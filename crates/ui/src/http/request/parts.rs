//! A multipart body's fields: a text each, or a file by its path, read when the request is sent.
//! One blank row at the end becomes a part as soon as something is typed or chosen in it.

use egui::{
    CornerRadius, FontFamily, Id, Rect, Role, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, pos2,
};
use engine::http::model::FormPart;
use i18n::keys;

use super::table::{self, CHECK, REMOVE, ROW};
use crate::theme::palette;
use crate::widgets::{self, interact};

const KIND: f32 = 96.0;

/// What a frame did to the table.
pub(super) enum Changed {
    Parts(Vec<FormPart>),
    /// A file asked for the row at that rank; the one after the last is the blank row.
    Pick(usize),
}

enum Row {
    Part(FormPart),
    Removed,
    Pick,
}

struct Columns {
    check: Rect,
    key: Rect,
    kind: Rect,
    value: Rect,
    remove: Rect,
}

impl Columns {
    fn of(frame: Rect) -> Self {
        let half = ((frame.width() - CHECK - KIND - REMOVE) / 2.0).max(60.0);
        let column = |from: f32, width: f32| {
            Rect::from_min_max(
                pos2(frame.left() + from, frame.top()),
                pos2(frame.left() + from + width, frame.bottom()),
            )
        };
        Self {
            check: column(0.0, CHECK),
            key: column(CHECK, half),
            kind: column(CHECK + half, KIND),
            value: column(CHECK + half + KIND, half),
            remove: column(CHECK + 2.0 * half + KIND, REMOVE),
        }
    }

    fn at(column: Rect, top: f32) -> Rect {
        Rect::from_min_max(pos2(column.left(), top), pos2(column.right(), top + ROW))
    }
}

pub(super) fn show(ui: &mut Ui, parts: &[FormPart], language: i18n::Language) -> Option<Changed> {
    let p = palette(ui.ctx());
    let id = ui.scope_id().with("http-parts");
    let label = i18n::translate(language, keys::http::body::kinds::MULTIPART, &[]);
    let frame = table::frame(ui, parts.len(), &label, "http-multipart-table");
    let columns = Columns::of(frame);
    for (column, key) in [
        (columns.key, keys::http::table::KEY),
        (columns.kind, keys::http::body::PART_KIND),
        (columns.value, keys::http::table::VALUE),
    ] {
        table::heading(ui, Columns::at(column, frame.top()), key, language);
    }
    let mut changed = None;
    for at in 0..=parts.len() {
        #[expect(clippy::cast_precision_loss, reason = "rows of a table")]
        let top = frame.top() + (at + 1) as f32 * ROW;
        ui.painter()
            .hline(frame.x_range(), top + 0.5, Stroke::new(1.0, p.line_soft));
        let blank = at == parts.len();
        let part = parts.get(at).cloned().unwrap_or_default();
        if let Some(row) = row_of(ui, &columns, (id.with(at), top, blank), part, language) {
            changed = Some(applied(parts, at, row));
        }
    }
    changed
}

fn applied(parts: &[FormPart], at: usize, row: Row) -> Changed {
    let mut all = parts.to_vec();
    match row {
        Row::Part(part) if at == parts.len() => all.push(part),
        Row::Part(part) => all[at] = part,
        Row::Removed => {
            all.remove(at);
        }
        Row::Pick => return Changed::Pick(at),
    }
    Changed::Parts(all)
}

fn row_of(
    ui: &mut Ui,
    columns: &Columns,
    (id, top, blank): (Id, f32, bool),
    mut part: FormPart,
    language: i18n::Language,
) -> Option<Row> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut changed = false;
    if !blank {
        let label = i18n::translate(
            language,
            keys::http::table::ENABLE,
            &[("key", part.key.as_str().into())],
        );
        let rect = Columns::at(columns.check, top);
        changed |= table::tick(ui, rect, &mut part.enabled, &label, "http-part-enabled");
    }
    let ink = if part.enabled || blank {
        p.text_0
    } else {
        p.text_2
    };
    let hint = if blank {
        tr(keys::http::table::NEW_KEY)
    } else {
        String::new()
    };
    changed |= table::text_cell(
        ui,
        (Columns::at(columns.key, top), id.with("key")),
        (&mut part.key, &tr(keys::http::table::KEY), "http-part-key"),
        (&hint, ink, true, false),
    )
    .changed();
    let kind = tr(if part.file {
        keys::http::body::PART_FILE
    } else {
        keys::http::body::PART_TEXT
    });
    let rect = Columns::at(columns.kind, top);
    if cell_button(
        ui,
        rect,
        (&kind, "http-part-kind"),
        FontFamily::Proportional,
    ) {
        // What was typed is no path, and a path is no text: it is let go.
        part.file = !part.file;
        part.value.clear();
        changed = true;
    }
    let rect = Columns::at(columns.value, top);
    if part.file {
        let chosen = if part.value.is_empty() {
            tr(keys::http::body::CHOOSE_FILE)
        } else {
            part.value.clone()
        };
        if cell_button(
            ui,
            rect,
            (&chosen, "http-part-choose"),
            FontFamily::Monospace,
        ) {
            return Some(Row::Pick);
        }
    } else {
        changed |= table::text_cell(
            ui,
            (rect, id.with("value")),
            (
                &mut part.value,
                &tr(keys::http::table::VALUE),
                "http-part-value",
            ),
            ("", ink, true, false),
        )
        .changed();
    }
    let rect = Columns::at(columns.remove, top);
    if !blank && table::remove(ui, rect, (&part.key, "http-part-remove"), language) {
        return Some(Row::Removed);
    }
    changed.then_some(Row::Part(part))
}

/// A cell that answers a click, its words cut at its edge.
fn cell_button(
    ui: &mut Ui,
    rect: Rect,
    (words, author_id): (&str, &str),
    family: FontFamily,
) -> bool {
    let p = palette(ui.ctx());
    let mut over = ui.new_child(UiBuilder::new().max_rect(rect));
    let response = interact(
        &mut over,
        rect.size(),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, words),
        CornerRadius::ZERO,
    );
    let ink = if response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, p.bg_2);
        p.text_0
    } else {
        p.text_1
    };
    let galley = widgets::text(ui, words, 12.0, 400.0, family, ink);
    let at = pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0);
    ui.painter_at(rect.shrink2(Vec2::new(8.0, 0.0)))
        .galley(at, galley, ink);
    response.clicked()
}
