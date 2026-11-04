//! The two bubbles the caller fills (plan § 3.8): the completion list at the caret — ↑ and ↓ to
//! move, Tab or Enter to accept, Escape to close until the word changes — and the tooltip for
//! the position under the pointer.

use egui::{Area, Id, Key, LayerId, Modifiers, Order, Pos2, Rect, Response, Role, Ui, WidgetInfo};
use text::Selection;

use super::Editor;

/// What the caller proposes for the word at the caret: the entries, and where that word starts.
pub struct Completion<'a> {
    pub from: usize,
    pub items: &'a [String],
}

/// Which entry is lit, and the word whose list Escape closed.
#[derive(Debug, Default)]
pub(crate) struct Choosing {
    selected: usize,
    /// Where the word the list was lit for starts: another word starts at the first entry.
    word: usize,
    dismissed: Option<(usize, String)>,
}

impl Editor {
    /// The word ending at the caret — letters, digits, `_` — and where it starts: what a list is
    /// filtered by. Empty with a selection.
    pub fn word_at_caret(&self) -> (usize, String) {
        let selection = self.buffer.selection();
        if !selection.is_empty() {
            return (selection.head, String::new());
        }
        let head = selection.head;
        let line = self.buffer.line_of(head);
        let start = self.buffer.line_start(line);
        let before: Vec<char> = self.buffer.line(line).chars().take(head - start).collect();
        let word: String = before
            .iter()
            .rev()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        (head - word.chars().count(), word)
    }
}

/// Whether the list is shown: entries to show, and not closed on this very word.
pub(crate) fn open(editor: &Editor, completion: Option<&Completion<'_>>) -> bool {
    let Some(completion) = completion.filter(|completion| !completion.items.is_empty()) else {
        return false;
    };
    let word = editor.word_at_caret();
    editor.choosing.dismissed.as_ref() != Some(&(completion.from, word.1))
}

/// The list's keys, taken before the editor reads them; the first line an acceptance changed.
pub(crate) fn keys(ui: &Ui, editor: &mut Editor, completion: &Completion<'_>) -> Option<usize> {
    let count = completion.items.len();
    let choosing = &mut editor.choosing;
    if choosing.word != completion.from {
        choosing.word = completion.from;
        choosing.selected = 0;
    }
    choosing.selected = choosing.selected.min(count - 1);
    let (up, down, accept, escape) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::ArrowUp),
            input.consume_key(Modifiers::NONE, Key::ArrowDown),
            input.consume_key(Modifiers::NONE, Key::Tab)
                || input.consume_key(Modifiers::NONE, Key::Enter),
            input.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    if up {
        choosing.selected = (choosing.selected + count - 1) % count;
    }
    if down {
        choosing.selected = (choosing.selected + 1) % count;
    }
    if escape {
        let (_, word) = editor.word_at_caret();
        editor.choosing.dismissed = Some((completion.from, word));
        return None;
    }
    accept.then(|| accept_entry(editor, completion, editor.choosing.selected))
}

/// The word at the caret replaced by entry `at`.
fn accept_entry(editor: &mut Editor, completion: &Completion<'_>, at: usize) -> usize {
    let head = editor.buffer.selection().head;
    editor.buffer.select(Selection {
        anchor: completion.from,
        head,
    });
    let edited = editor.buffer.insert(&completion.items[at]);
    editor.choosing = Choosing::default();
    edited.first_line
}

fn layer(owner: Id) -> LayerId {
    LayerId::new(Order::Foreground, owner.with("completion"))
}

/// Whether the pointer is over the list drawn for the editor `owner`.
pub(crate) fn pointer_on_list(ui: &Ui, owner: Id) -> bool {
    let ctx = ui.ctx();
    ctx.pointer_latest_pos()
        .and_then(|pos| ctx.layer_id_at(pos))
        .is_some_and(|layer_id| layer_id == layer(owner))
}

/// The list under the caret; the first line a click on an entry changed.
pub(crate) fn list(
    ui: &Ui,
    editor: &mut Editor,
    completion: &Completion<'_>,
    caret: Rect,
    owner: Id,
) -> Option<usize> {
    let selected = editor.choosing.selected;
    let shown = Area::new(layer(owner).id)
        .order(Order::Foreground)
        .fixed_pos(Pos2::new(caret.left(), caret.bottom() + 2.0))
        .show(ui.ctx(), |ui| {
            crate::widgets::menu_frame(ui.ctx())
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut clicked = None;
                    for (at, item) in completion.items.iter().enumerate().take(50) {
                        let entry = option(ui, item, at == selected);
                        if at == selected {
                            entry.scroll_to_me(None);
                        }
                        if entry.clicked() {
                            clicked = Some(at);
                        }
                    }
                    clicked
                })
                .inner
        });
    let response = shown.response;
    response.widget_info(|| WidgetInfo::labeled(Role::ListBox, true, "completion"));
    crate::widgets::address(ui, &response, "editor-completion");
    let accepted = shown.inner.map(|at| accept_entry(editor, completion, at));
    if accepted.is_some() {
        ui.memory_mut(|memory| memory.request_focus(owner));
    }
    accepted
}

/// An entry: amber while it is the one Enter takes.
fn option(ui: &mut Ui, item: &str, selected: bool) -> Response {
    let entry =
        crate::widgets::menu_entry(ui, item, "editor-completion-option", false, selected, false);
    entry.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, selected, item));
    entry
}

/// The caller's words for the position under the pointer, as a tooltip.
pub(crate) fn tooltip(
    ui: &Ui,
    response: &Response,
    editor: &Editor,
    frame: &super::geometry::Frame,
    hover: &dyn Fn(usize) -> Option<String>,
) {
    let Some(words) = response
        .hover_pos()
        .map(|pos| super::geometry::hit(ui, &editor.buffer, &editor.lines, pos, frame))
        .and_then(hover)
    else {
        return;
    };
    response.clone().on_hover_ui_at_pointer(|ui| {
        let label = ui.label(words);
        crate::widgets::address(ui, &label, "editor-hover");
    });
}

#[cfg(test)]
mod tests;
