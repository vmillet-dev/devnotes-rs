//! The run's order: dragged by its grip, or moved with Alt + ↑ ↓ on a row; Space leaves a row out.

use egui::{
    CornerRadius, FontFamily, Key, Modifiers, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2,
};
use i18n::keys;

use super::{Ask, ask, badge_of};
use crate::action::Actions;
use crate::http::badge_hue;
use crate::state::AppState;
use crate::state::http::runner::Row;
use crate::theme::palette;
use crate::widgets::{self, Checkbox, interact, virtual_list};

const ROW: f32 = 32.0;
const HINT: f32 = 40.0;
const GRIP: f32 = 24.0;

/// A drag under way: the row taken, and where it would land.
#[derive(Clone, Copy)]
struct Dragging {
    from: usize,
    to: usize,
}

#[derive(Default)]
pub(super) struct OrderView {
    dragging: Option<Dragging>,
    /// Where a row moved by the keys went: a widget's id follows its place, the focus does not.
    focus: Option<usize>,
}

pub(super) fn show(ui: &mut Ui, state: &AppState, view: &mut OrderView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let runner = state.http.runner();
    let rows = runner.rows();
    let caption = i18n::translate(
        language,
        keys::http::runner::ORDER,
        &[
            ("running", runner.enabled_count().into()),
            (
                "count",
                u32::try_from(rows.len()).unwrap_or(u32::MAX).into(),
            ),
        ],
    );
    let caption = egui::RichText::new(caption).size(12.0).color(p.text_2);
    let said = ui.add(egui::Label::new(caption));
    widgets::address(ui, &said, "http-runner-order");
    let room = ui.available_rect_before_wrap();
    let list = Rect::from_min_max(room.min, pos2(room.right(), room.bottom() - HINT));
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("http-runner-rows")
            .max_rect(list),
    );
    inside.spacing_mut().item_spacing.y = 0.0;
    let disabled = runner.is_running();
    // Taken before the rows: one set while they are drawn is for the order of the next frame.
    let focus = view.focus.take();
    virtual_list(
        &mut inside,
        "http-runner-rows",
        ROW,
        rows.len(),
        |ui, at| {
            let id = rows[at].id.as_str();
            ui.push_id(id, |ui| {
                let response = row(ui, (rows, at, disabled), view, language, actions);
                if focus == Some(at) {
                    response.request_focus();
                }
            });
        },
    );
    ui.advance_cursor_after_rect(list);
    let hint = i18n::translate(language, keys::http::runner::ORDER_HINT, &[]);
    let hint = egui::RichText::new(hint).size(11.5).color(p.text_2);
    ui.add(egui::Label::new(hint).wrap());
}

fn row(
    ui: &mut Ui,
    (rows, at, disabled): (&[Row], usize, bool),
    view: &mut OrderView,
    language: i18n::Language,
    actions: &mut Actions,
) -> egui::Response {
    let p = palette(ui.ctx());
    let shown = &rows[at];
    let size = Vec2::new(ui.available_width(), ROW);
    let name = shown.name.as_str();
    let response = interact(
        ui,
        size,
        Sense::click(),
        "http-runner-row",
        || WidgetInfo::labeled(Role::ListItem, true, name),
        CornerRadius::same(5),
    );
    if response.clicked() {
        response.request_focus();
    }
    if !disabled {
        keys_of(ui, &response, (rows.len(), at), view, actions);
    }
    let rect = response.rect;
    if view
        .dragging
        .is_some_and(|drag| drag.to == at && drag.from != at)
    {
        let line = if view.dragging.is_some_and(|drag| drag.from < at) {
            rect.bottom()
        } else {
            rect.top()
        };
        ui.painter()
            .hline(rect.x_range(), line, Stroke::new(2.0, p.amber_edge));
    }
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inside.spacing_mut().item_spacing.x = 4.0;
    let label = i18n::translate(
        language,
        keys::http::runner::INCLUDE,
        &[("name", name.into())],
    );
    let mut enabled = shown.enabled;
    let checked = Checkbox::new(&mut enabled, &label, "http-runner-row-enabled")
        .enabled(!disabled)
        .show(&mut inside);
    if checked.changed() {
        ask(actions, Ask::Toggle(at));
    }
    grip(&mut inside, (rows.len(), at, disabled), view, rect, actions);
    words(&mut inside, shown);
    response
}

fn keys_of(
    ui: &mut Ui,
    row: &egui::Response,
    (count, at): (usize, usize),
    view: &mut OrderView,
    actions: &mut Actions,
) {
    if !widgets::takes_keys(ui, row) {
        return;
    }
    let arrows = egui::EventFilter {
        vertical_arrows: true,
        ..Default::default()
    };
    ui.memory_mut(|memory| memory.set_focus_lock_filter(row.id, arrows));
    let (space, up, down) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::Space),
            input.consume_key(Modifiers::ALT, Key::ArrowUp),
            input.consume_key(Modifiers::ALT, Key::ArrowDown),
        )
    });
    if space {
        ask(actions, Ask::Toggle(at));
    }
    if up && at > 0 {
        view.focus = Some(at - 1);
        ask(
            actions,
            Ask::Move {
                from: at,
                to: at - 1,
            },
        );
    }
    if down && at + 1 < count {
        view.focus = Some(at + 1);
        ask(
            actions,
            Ask::Move {
                from: at,
                to: at + 1,
            },
        );
    }
}

fn grip(
    ui: &mut Ui,
    (count, at, disabled): (usize, usize, bool),
    view: &mut OrderView,
    row: Rect,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::new(GRIP, ROW), Sense::drag());
    widgets::address(ui, &response, "http-runner-row-grip");
    for dot in 0..6_u8 {
        let x = rect.center().x + if dot % 2 == 0 { -2.5 } else { 2.5 };
        let y = rect.center().y + (f32::from(dot / 2) - 1.0) * 5.0;
        ui.painter().circle_filled(pos2(x, y), 1.2, p.text_2);
    }
    if disabled {
        return;
    }
    if response.drag_started() {
        view.dragging = Some(Dragging { from: at, to: at });
    }
    if let Some(y) = response
        .interact_pointer_pos()
        .map(|pointer| pointer.y)
        .filter(|_| response.dragged())
        && let Some(drag) = &mut view.dragging
    {
        // Rows are one height: where the pointer is, counted from this row.
        let moved = ((y - row.top()) / ROW).floor();
        #[expect(clippy::cast_possible_truncation, reason = "a few rows")]
        let moved = moved as isize;
        let to = at.saturating_add_signed(moved).min(count.saturating_sub(1));
        drag.to = to;
    }
    if response.drag_stopped()
        && let Some(drag) = view.dragging.take()
        && drag.from != drag.to
    {
        ask(
            actions,
            Ask::Move {
                from: drag.from,
                to: drag.to,
            },
        );
    }
}

fn words(ui: &mut Ui, shown: &Row) {
    let p = palette(ui.ctx());
    let ink = if shown.enabled { p.text_0 } else { p.text_2 };
    let badge = badge_of(shown.kind, shown.method);
    let hue = badge_hue(ui, badge);
    let mark = widgets::text(ui, badge.short(), 11.0, 600.0, FontFamily::Monospace, hue);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(32.0, mark.size().y), Sense::hover());
    ui.painter().galley(rect.min, mark, hue);
    let mut name = shown.path.iter().fold(String::new(), |mut name, folder| {
        name.push_str(folder);
        name.push_str(" › ");
        name
    });
    name.push_str(&shown.name);
    let name = egui::RichText::new(name).size(12.5).color(ink);
    ui.add(egui::Label::new(name).truncate().selectable(false));
}
