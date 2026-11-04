//! A todo list's items (the reference's `checklist-editor`): ticked, typed, split by Enter,
//! joined by Backspace, pasted line by line, and reordered by the grip — dragged with the pointer
//! or moved by Alt+↑/↓, which every drag has as its twin. Every change but typing is written at
//! once; typing is written when the row is left.

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    Align, CornerRadius, Event, FontFamily, FontId, Id, Key, Layout, Modifiers, Rect, Role, Sense,
    Stroke, TextEdit, Ui, Vec2, WidgetInfo,
};
use engine::notes::checklist::ChecklistItem;
use i18n::{Language, keys};

use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, IconButton, address, interact};

const ROW: f32 = 30.0;

/// What is typed, and where the next frame puts the caret.
#[derive(Debug, Default)]
pub(super) struct ChecklistView {
    pub(super) items: Vec<ChecklistItem>,
    /// The row a pointer drags, and the row it would land on.
    dragging: Option<(usize, usize)>,
    /// The row to focus at the next frame, and its caret.
    focus: Option<(usize, usize)>,
}

impl ChecklistView {
    pub(super) fn of(items: &[ChecklistItem]) -> Self {
        Self {
            items: items.to_vec(),
            ..Self::default()
        }
    }

    fn move_item(&mut self, from: usize, to: usize) -> bool {
        if to >= self.items.len() || from == to {
            return false;
        }
        let moved = self.items.remove(from);
        self.items.insert(to, moved);
        self.focus = Some((to, usize::MAX));
        true
    }
}

/// What a row's keys asked, read before its field draws.
enum RowKey {
    Split(usize),
    Join,
    Remove,
    Paste(Vec<String>),
    Move(isize),
}

/// The list; `true` once something is to be written.
pub(super) fn show(ui: &mut Ui, view: &mut ChecklistView, language: Language) -> bool {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut changed = false;
    ui.spacing_mut().item_spacing = Vec2::new(8.0, 2.0);
    progress(ui, &view.items, language);
    ui.add_space(10.0);
    let rows_top = ui.cursor().top();
    let mut asked = None;
    // The rows keep their places while one is dragged — a row moved under the pointer would
    // be another widget to egui — and a line shows where it would land.
    for index in 0..view.items.len() {
        #[expect(clippy::cast_precision_loss, reason = "a list's length")]
        let top = rows_top + index as f32 * (ROW + 2.0);
        let rect = Rect::from_min_size(
            egui::pos2(ui.max_rect().left(), top),
            Vec2::new(ui.available_width(), ROW),
        );
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("checklist-row-ui", index))
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let dragged = view.dragging.is_some_and(|(from, _)| from == index);
        if dragged || row_ui.rect_contains_pointer(rect) {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(6),
                if dragged { p.bg_3 } else { p.bg_2 },
            );
        }
        let (outcome, grip) = row(&mut row_ui, view, index, language);
        if let Some(outcome) = outcome {
            asked = Some((index, outcome));
        }
        if let Some(dropped) = drag(ui, view, index, &grip, rows_top) {
            asked = Some((index, dropped));
        }
    }
    if let Some((from, to)) = view.dragging.filter(|(from, to)| from != to) {
        // Below the row it lands on when it goes down, above it when it goes up.
        let slot = if to > from { to + 1 } else { to };
        #[expect(clippy::cast_precision_loss, reason = "a list's length")]
        let y = rows_top + slot as f32 * (ROW + 2.0) - 1.0;
        let x = ui.max_rect().x_range();
        ui.painter().hline(x, y, Stroke::new(2.0, p.amber_text));
    }
    #[expect(clippy::cast_precision_loss, reason = "a list's length")]
    let height = view.items.len() as f32 * (ROW + 2.0);
    ui.allocate_space(Vec2::new(ui.available_width(), height));
    if let Some((index, outcome)) = asked {
        let count = view.items.len();
        changed |= apply(view, index, outcome);
        if view.items.len() != count || view.focus.is_some() {
            settle_rows(ui, view, index);
        }
    }
    ui.add_space(8.0);
    let add = tr(keys::editor::ADD_ITEM);
    if widgets::Button::new(&add, "checklist-add")
        .kind(widgets::Kind::Quiet)
        .icon(Icon::Plus)
        .show(ui)
        .clicked()
    {
        view.items.push(ChecklistItem {
            text: String::new(),
            done: false,
        });
        view.focus = Some((view.items.len() - 1, 0));
        changed = true;
    }
    changed
}

/// What one row did.
enum Outcome {
    Toggled,
    Removed,
    Left,
    Key(RowKey),
    /// The grip let go over the row it was moved to.
    Dropped(usize),
}

/// A pointer on the grip: the row follows it, and lands where it is let go.
fn drag(
    ui: &Ui,
    view: &mut ChecklistView,
    index: usize,
    grip: &egui::Response,
    rows_top: f32,
) -> Option<Outcome> {
    if grip.drag_started() {
        view.dragging = Some((index, index));
    }
    let (from, _) = view.dragging.filter(|(from, _)| *from == index)?;
    if let Some(pointer) = ui.ctx().pointer_latest_pos() {
        let slot = ((pointer.y - rows_top) / (ROW + 2.0)).floor().max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a row"
        )]
        let to = (slot as usize).min(view.items.len() - 1);
        view.dragging = Some((from, to));
    }
    if !grip.drag_stopped() {
        return None;
    }
    let to = view.dragging.map_or(from, |(_, to)| to);
    view.dragging = None;
    Some(Outcome::Dropped(to))
}

fn row(
    ui: &mut Ui,
    view: &mut ChecklistView,
    index: usize,
    language: Language,
) -> (Option<Outcome>, egui::Response) {
    let p = palette(ui.ctx());
    let text = view.items[index].text.clone();
    let done = view.items[index].done;
    let named = |key| i18n::translate(language, key, &[("text", text.clone().into())]);
    let mut outcome = None;
    let grip_label = named(keys::editor::MOVE_ITEM);
    let grip = interact(
        ui,
        Vec2::new(16.0, ROW),
        Sense::click_and_drag(),
        "checklist-grip",
        || WidgetInfo::labeled(Role::Button, true, &grip_label),
        CornerRadius::same(4),
    );
    let dots = egui::Rect::from_center_size(grip.rect.center(), Vec2::splat(12.0));
    icons::paint(ui, dots, Icon::Ellipsis, p.text_2);
    if widgets::takes_keys(ui, &grip)
        && let Some(by) = alt_arrows(ui)
    {
        outcome = Some(Outcome::Key(RowKey::Move(by)));
    }
    if tick(ui, done, &named(keys::editor::TOGGLE_ITEM)).clicked() {
        outcome = Some(Outcome::Toggled);
    }
    let id = row_id(index);
    if let Some(key) = row_keys(ui, id, &view.items[index].text) {
        outcome = Some(Outcome::Key(key));
    }
    let remove_room = 28.0;
    let field_width = (ui.available_width() - remove_room).max(60.0);
    let label = i18n::translate(
        language,
        keys::editor::ITEM_LABEL,
        &[(
            "position",
            u32::try_from(index + 1).unwrap_or(u32::MAX).into(),
        )],
    );
    let hint = i18n::translate(language, keys::editor::ITEM_PLACEHOLDER, &[]);
    let field = ui.add_sized(
        Vec2::new(field_width, ROW - 6.0),
        TextEdit::singleline(&mut view.items[index].text)
            .id(id)
            .frame(egui::Frame::new())
            .hint_text(hint)
            .text_color(if done { p.text_2 } else { p.text_0 })
            .font(FontId::new(13.5, FontFamily::Proportional)),
    );
    ui.ctx()
        .accesskit_node_builder(field.id, |node| node.set_label(label.clone()));
    address(ui, &field, "checklist-text");
    if done {
        strike(ui, &field, &view.items[index].text);
    }
    if let Some((_, caret)) = view.focus.filter(|(row, _)| *row == index) {
        field.request_focus();
        place_caret(ui, id, caret.min(view.items[index].text.chars().count()));
        view.focus = None;
    }
    if field.lost_focus() && outcome.is_none() {
        outcome = Some(Outcome::Left);
    }
    let remove = named(keys::editor::REMOVE_ITEM);
    if IconButton::new(Icon::X, &remove, "checklist-remove")
        .show(ui)
        .clicked()
    {
        outcome = Some(Outcome::Removed);
    }
    (outcome, grip)
}

/// The box: amber and ticked when the task is done.
fn tick(ui: &mut Ui, done: bool, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let check = interact(
        ui,
        Vec2::splat(20.0),
        Sense::click(),
        "checklist-check",
        || WidgetInfo::selected(Role::CheckBox, true, done, label),
        CornerRadius::same(4),
    );
    let (fill, edge) = if done {
        (p.amber_fill, p.amber_text)
    } else {
        (egui::Color32::TRANSPARENT, p.text_2)
    };
    let tick = egui::Rect::from_center_size(check.rect.center(), Vec2::splat(14.0));
    ui.painter().rect(
        tick,
        CornerRadius::same(3),
        fill,
        Stroke::new(1.2, edge),
        egui::StrokeKind::Inside,
    );
    if done {
        icons::paint(ui, tick.shrink(1.5), Icon::Check, p.amber_ink);
    }
    check
}

/// A done task's words, struck through.
fn strike(ui: &Ui, field: &egui::Response, text: &str) {
    let p = palette(ui.ctx());
    let font = FontId::new(13.5, FontFamily::Proportional);
    let width = ui
        .fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font, p.text_2))
        .size()
        .x;
    let y = field.rect.center().y;
    ui.painter().hline(
        field.rect.left()..=field.rect.left() + width,
        y,
        Stroke::new(1.0, p.text_2),
    );
}

/// Enter, Backspace at the start, a paste of several lines and Alt+↑/↓, taken from the field's
/// keys before it reads them.
fn row_keys(ui: &Ui, id: Id, text: &str) -> Option<RowKey> {
    if !ui.memory(|memory| memory.has_focus(id)) {
        return None;
    }
    let caret = TextEditState::load(ui.ctx(), id)
        .and_then(|state| state.cursor.char_range())
        .unwrap_or_else(|| CCursorRange::one(CCursor::new(text.chars().count())));
    let (start, end) = (
        caret.primary.index.min(caret.secondary.index),
        caret.primary.index.max(caret.secondary.index),
    );
    if let Some(by) = alt_arrows(ui) {
        return Some(RowKey::Move(by));
    }
    ui.ctx().input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::Enter) {
            return Some(RowKey::Split(start.0));
        }
        if start.0 == 0 && end.0 == 0 && input.key_pressed(Key::Backspace) {
            input.consume_key(Modifiers::NONE, Key::Backspace);
            return Some(if text.is_empty() {
                RowKey::Remove
            } else {
                RowKey::Join
            });
        }
        let pasted = input
            .events
            .iter()
            .position(|event| matches!(event, Event::Paste(text) if text.trim().contains('\n')))?;
        let Event::Paste(lines) = input.events.remove(pasted) else {
            return None;
        };
        Some(RowKey::Paste(
            lines
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect(),
        ))
    })
}

fn alt_arrows(ui: &Ui) -> Option<isize> {
    ui.ctx().input_mut(|input| {
        if input.consume_key(Modifiers::ALT, Key::ArrowUp) {
            Some(-1)
        } else if input.consume_key(Modifiers::ALT, Key::ArrowDown) {
            Some(1)
        } else {
            None
        }
    })
}

/// The rows moved under their fields: what egui kept of the fields from `from` on is
/// forgotten, and the row to type in takes the focus now rather than at the next frame.
fn settle_rows(ui: &Ui, view: &ChecklistView, from: usize) {
    let rows = view.items.len() + 2;
    ui.ctx().data_mut(|data| {
        for index in from.saturating_sub(1)..rows {
            data.remove::<TextEditState>(row_id(index));
        }
    });
    match view.focus {
        Some((row, _)) => ui.memory_mut(|memory| memory.request_focus(row_id(row))),
        None => ui.memory_mut(egui::Memory::stop_text_input),
    }
}

fn row_id(index: usize) -> Id {
    Id::unique("checklist-row").with(index)
}

fn place_caret(ui: &Ui, id: Id, at: usize) {
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
    state.store(ui.ctx(), id);
}

/// What a row asked, done to the list; `true` when it is to be written.
fn apply(view: &mut ChecklistView, index: usize, outcome: Outcome) -> bool {
    match outcome {
        Outcome::Toggled => {
            view.items[index].done = !view.items[index].done;
            true
        }
        Outcome::Removed => {
            view.items.remove(index);
            // The row above, not the next: that is where the caret was.
            view.focus = (!view.items.is_empty()).then(|| (index.saturating_sub(1), usize::MAX));
            true
        }
        Outcome::Left => true,
        Outcome::Dropped(to) => view.move_item(index, to),
        Outcome::Key(RowKey::Move(by)) => {
            let to = index.checked_add_signed(by);
            to.is_some_and(|to| view.move_item(index, to))
        }
        Outcome::Key(RowKey::Split(at)) => {
            let text = view.items[index].text.clone();
            let cut = text
                .char_indices()
                .nth(at)
                .map_or(text.len(), |(byte, _)| byte);
            text[..cut].clone_into(&mut view.items[index].text);
            view.items.insert(
                index + 1,
                ChecklistItem {
                    text: text[cut..].to_owned(),
                    done: false,
                },
            );
            view.focus = Some((index + 1, 0));
            true
        }
        Outcome::Key(RowKey::Join) => {
            if index == 0 {
                return false;
            }
            let joined = view.items.remove(index);
            let above = &mut view.items[index - 1];
            let caret = above.text.chars().count();
            above.text.push_str(&joined.text);
            view.focus = Some((index - 1, caret));
            true
        }
        Outcome::Key(RowKey::Remove) => {
            view.items.remove(index);
            view.focus = (!view.items.is_empty()).then(|| (index.saturating_sub(1), usize::MAX));
            true
        }
        Outcome::Key(RowKey::Paste(lines)) => {
            paste(view, index, &lines);
            true
        }
    }
}

/// Several lines pasted are several items from this one on: a task's box or a list marker is
/// read, what the row held goes before the first.
fn paste(view: &mut ChecklistView, index: usize, lines: &[String]) {
    let mut pasted: Vec<ChecklistItem> = lines.iter().map(|line| pasted_item(line)).collect();
    let Some(first) = pasted.first_mut() else {
        return;
    };
    let before = view.items[index].text.clone();
    if !before.trim().is_empty() {
        first.text = format!("{before}{}", first.text);
        first.done = view.items[index].done;
    }
    let last = index + pasted.len() - 1;
    let caret = pasted.last().map_or(0, |item| item.text.chars().count());
    view.items.splice(index..=index, pasted);
    view.focus = Some((last, caret));
}

/// The reference's `pastedItem`: `- [x] ship` is a ticked task, `1. tag` a task, a line a task.
fn pasted_item(line: &str) -> ChecklistItem {
    let trimmed = line.trim_start();
    let unmarked = trimmed
        .strip_prefix(['-', '*', '+'])
        .or_else(|| {
            let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
            (digits > 0)
                .then(|| &trimmed[digits..])
                .and_then(|rest| rest.strip_prefix(['.', ')']))
        })
        .map_or(trimmed, str::trim_start);
    let boxed = unmarked
        .strip_prefix("[ ]")
        .map(|rest| (rest, false))
        .or_else(|| {
            unmarked
                .strip_prefix("[x]")
                .or_else(|| unmarked.strip_prefix("[X]"))
                .map(|rest| (rest, true))
        });
    match boxed {
        Some((rest, done)) if rest.starts_with(char::is_whitespace) => ChecklistItem {
            text: rest.trim().to_owned(),
            done,
        },
        _ => ChecklistItem {
            text: if unmarked.len() < trimmed.len() {
                unmarked
            } else {
                trimmed
            }
            .trim_end()
            .to_owned(),
            done: false,
        },
    }
}

/// The bar and « 2/5 tâches ».
fn progress(ui: &mut Ui, items: &[ChecklistItem], language: Language) {
    let p = palette(ui.ctx());
    let done = items.iter().filter(|item| item.done).count();
    let total = items.len();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let words = i18n::translate(
        language,
        keys::editor::CHECKLIST_STATS,
        &[("done", count(done).into()), ("total", count(total).into())],
    );
    ui.horizontal(|ui| {
        let galley = widgets::text(ui, &words, 11.0, 400.0, FontFamily::Monospace, p.text_2);
        let width = ui.available_width() - galley.size().x - 10.0;
        let (track, _) = ui.allocate_exact_size(Vec2::new(width.max(40.0), 6.0), Sense::hover());
        ui.painter()
            .rect_filled(track, CornerRadius::same(3), p.bg_3);
        if total > 0 {
            #[expect(clippy::cast_precision_loss, reason = "a list's length")]
            let share = done as f32 / total as f32;
            let fill =
                Rect::from_min_size(track.min, Vec2::new(track.width() * share, track.height()));
            ui.painter()
                .rect_filled(fill, CornerRadius::same(3), p.amber_fill);
        }
        let response = ui.label(galley);
        address(ui, &response, "checklist-progress");
    });
}

#[cfg(test)]
mod tests;
