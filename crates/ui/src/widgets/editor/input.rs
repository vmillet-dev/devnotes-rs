//! What the pointer and the keys do to the text. The keys the editor handles are taken out of the
//! frame's input (D14): an application shortcut never sees a Ctrl+Z meant for the text.

use egui::{Event, ImeEvent, Key, Modifiers, Pos2, Response, Ui};
use text::{Direction, Selection, Unit};

use super::Editor;
use super::geometry::{self, Frame};

/// What the keys need to know of the editor's options.
pub(crate) struct Keys<'a> {
    pub unit: &'a str,
    pub comment: Option<&'a str>,
    pub read_only: bool,
    /// The viewport's height: how far Page Up and Page Down go.
    pub page: f32,
}

/// A press puts the caret (Shift extends the selection), a drag extends it, a double click takes
/// the word, a triple click the line.
pub(crate) fn pointer(ui: &Ui, response: &Response, editor: &mut Editor, frame: &Frame) {
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let at = geometry::hit(ui, &editor.buffer, &editor.lines, pos, frame);
    let shift = ui.input(|input| input.modifiers.shift);
    if response.triple_clicked() {
        let line = editor.buffer.line_of(at);
        let end = if line + 1 < editor.buffer.line_count() {
            editor.buffer.line_start(line + 1)
        } else {
            editor.buffer.len()
        };
        editor.buffer.select(Selection {
            anchor: editor.buffer.line_start(line),
            head: end,
        });
    } else if response.double_clicked() {
        let start = editor.buffer.step(at, Direction::Backward, Unit::Word);
        let end = editor.buffer.step(start, Direction::Forward, Unit::Word);
        editor.buffer.select(Selection {
            anchor: start,
            head: end.max(at),
        });
    } else if ui.input(|input| input.pointer.primary_pressed())
        && response.is_pointer_button_down_on()
    {
        editor.buffer.move_to(at, shift);
        response.request_focus();
    } else if response.dragged() {
        editor.buffer.move_to(at, true);
    } else {
        return;
    }
    editor.goal_x = None;
    editor.reveal = true;
}

/// The frame's events for the focused editor; the first line an edit touched.
pub(crate) fn keyboard(
    ui: &Ui,
    editor: &mut Editor,
    keys: &Keys<'_>,
    frame: &Frame,
) -> Option<usize> {
    let events = ui.input(|input| input.events.clone());
    let mut first: Option<usize> = None;
    let mut handled = Vec::with_capacity(events.len());
    for event in &events {
        let edited = match event {
            Event::Text(text) if !keys.read_only => Some(editor.buffer.insert(text).first_line),
            Event::Paste(text) if !keys.read_only => Some(editor.buffer.insert(text).first_line),
            Event::Copy => {
                copy(ui, editor);
                None
            }
            Event::Cut => {
                copy(ui, editor);
                cut(editor, keys)
            }
            Event::Ime(ime) => ime_event(editor, ime, keys),
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => match key_event(ui, editor, (*key, *modifiers), keys, frame) {
                Handled::No => {
                    handled.push(false);
                    continue;
                }
                Handled::Moved => None,
                Handled::Edited(line) => line,
            },
            _ => {
                handled.push(false);
                continue;
            }
        };
        handled.push(true);
        if let Some(line) = edited {
            first = Some(first.map_or(line, |first| first.min(line)));
            editor.goal_x = None;
        }
        editor.reveal = true;
    }
    let mut index = 0;
    ui.ctx().input_mut(|input| {
        input.events.retain(|_| {
            let keep = !handled.get(index).copied().unwrap_or(false);
            index += 1;
            keep
        });
    });
    first
}

enum Handled {
    No,
    Moved,
    /// The text changed, from this line; `None` when the key found nothing to change.
    Edited(Option<usize>),
}

fn key_event(
    ui: &Ui,
    editor: &mut Editor,
    (key, modifiers): (Key, Modifiers),
    keys: &Keys<'_>,
    frame: &Frame,
) -> Handled {
    let extend = modifiers.shift;
    let word = modifiers.command;
    let unit = if word { Unit::Word } else { Unit::Grapheme };
    let buffer = &mut editor.buffer;
    let head = buffer.selection().head;
    let collapse = |buffer: &text::Buffer, direction| {
        let range = buffer.selection().range();
        match direction {
            Direction::Backward => range.start,
            Direction::Forward => range.end,
        }
    };
    let edit = |line: Option<text::Edited>| Handled::Edited(line.map(|edited| edited.first_line));
    let writable = !keys.read_only;
    match key {
        Key::ArrowLeft | Key::ArrowRight if modifiers.alt => Handled::No,
        Key::ArrowLeft | Key::ArrowRight => {
            let direction = if key == Key::ArrowLeft {
                Direction::Backward
            } else {
                Direction::Forward
            };
            let to = if !extend && !word && !buffer.selection().is_empty() {
                collapse(buffer, direction)
            } else {
                buffer.step(head, direction, unit)
            };
            buffer.move_to(to, extend);
            editor.goal_x = None;
            Handled::Moved
        }
        Key::ArrowUp | Key::ArrowDown if modifiers.alt && writable => {
            let direction = if key == Key::ArrowUp {
                Direction::Backward
            } else {
                Direction::Forward
            };
            edit(buffer.move_lines(direction))
        }
        Key::ArrowUp | Key::ArrowDown if !modifiers.command => {
            let rows = if key == Key::ArrowUp { -1.0 } else { 1.0 };
            vertical(ui, editor, rows * frame.row_height, extend, frame);
            Handled::Moved
        }
        Key::PageUp | Key::PageDown => {
            let sign = if key == Key::PageUp { -1.0 } else { 1.0 };
            vertical(
                ui,
                editor,
                sign * keys.page.max(frame.row_height),
                extend,
                frame,
            );
            Handled::Moved
        }
        Key::Home | Key::End => {
            let direction = if key == Key::Home {
                Direction::Backward
            } else {
                Direction::Forward
            };
            let to = match (word, direction) {
                (true, Direction::Backward) => 0,
                (true, Direction::Forward) => buffer.len(),
                (false, _) => home_or_end(buffer, head, direction),
            };
            buffer.move_to(to, extend);
            editor.goal_x = None;
            Handled::Moved
        }
        Key::A if modifiers.command => {
            buffer.select_all();
            Handled::Moved
        }
        Key::Backspace if writable => edit(buffer.delete(Direction::Backward, unit)),
        Key::Delete if writable => edit(buffer.delete(Direction::Forward, unit)),
        Key::K if modifiers.command && modifiers.shift && writable => {
            edit(buffer.delete(Direction::Forward, Unit::Line))
        }
        Key::Enter if writable => {
            let line = buffer.line_of(head);
            let before: String = buffer
                .line(line)
                .chars()
                .take(head - buffer.line_start(line))
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();
            edit(Some(buffer.insert(&format!("\n{before}"))))
        }
        Key::Tab if writable && modifiers.shift => edit(buffer.outdent(keys.unit)),
        Key::Tab if writable && modifiers.is_none() => edit(Some(buffer.indent(keys.unit))),
        Key::Z if modifiers.command && modifiers.shift && writable => edit(buffer.redo()),
        Key::Z if modifiers.command && writable => edit(buffer.undo()),
        Key::Y if modifiers.command && writable => edit(buffer.redo()),
        Key::Slash if modifiers.command && writable => match keys.comment {
            Some(token) => edit(Some(buffer.toggle_comment(token))),
            None => Handled::No,
        },
        _ => Handled::No,
    }
}

/// Home goes to the first non-blank of the line, then to its very start.
fn home_or_end(buffer: &text::Buffer, head: usize, direction: Direction) -> usize {
    let line = buffer.line_of(head);
    let start = buffer.line_start(line);
    match direction {
        Direction::Forward => buffer.line_end(line),
        Direction::Backward => {
            let indent = buffer
                .line(line)
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();
            if head == start + indent {
                start
            } else {
                start + indent
            }
        }
    }
}

/// Up or down by `dy`, at the column the caret had when it started moving.
fn vertical(ui: &Ui, editor: &mut Editor, dy: f32, extend: bool, frame: &Frame) {
    let head = editor.buffer.selection().head;
    let caret = geometry::caret_rect(ui, &editor.buffer, &editor.lines, head, frame);
    let x = *editor.goal_x.get_or_insert(caret.left());
    let target = Pos2::new(x, caret.center().y + dy);
    let at = if target.y < frame.top {
        0
    } else if target.y > frame.top + editor.lines.total() {
        editor.buffer.len()
    } else {
        geometry::hit(ui, &editor.buffer, &editor.lines, target, frame)
    };
    editor.buffer.move_to(at, extend);
    editor.reveal = true;
    editor.goal_x = Some(x);
}

fn copy(ui: &Ui, editor: &Editor) {
    if !editor.buffer.selection().is_empty() {
        ui.ctx().copy_text(editor.buffer.selected_text());
    }
}

fn cut(editor: &mut Editor, keys: &Keys<'_>) -> Option<usize> {
    if keys.read_only || editor.buffer.selection().is_empty() {
        return None;
    }
    editor
        .buffer
        .delete(Direction::Backward, Unit::Grapheme)
        .map(|edited| edited.first_line)
}

/// A composition is drawn at the caret while it is built; only what it commits is typed.
fn ime_event(editor: &mut Editor, ime: &ImeEvent, keys: &Keys<'_>) -> Option<usize> {
    match ime {
        ImeEvent::Preedit { text, .. } => {
            editor.preedit.clone_from(text);
            None
        }
        ImeEvent::Commit(text) => {
            editor.preedit.clear();
            (!keys.read_only && !text.is_empty()).then(|| editor.buffer.insert(text).first_line)
        }
        ImeEvent::DeleteSurrounding {
            before_chars,
            after_chars,
        } if !keys.read_only => {
            let head = editor.buffer.selection().head;
            let from = head.saturating_sub(*before_chars);
            let to = (head + after_chars).min(editor.buffer.len());
            editor.buffer.select(Selection {
                anchor: from,
                head: to,
            });
            editor
                .buffer
                .delete(Direction::Backward, Unit::Grapheme)
                .map(|edited| edited.first_line)
        }
        _ => {
            editor.preedit.clear();
            None
        }
    }
}
