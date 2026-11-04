//! The body under the header, by kind, and what the footer counts of it.

use egui::{FontFamily, FontId, TextEdit, Ui};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;
use i18n::{Language, keys};

use crate::state::AppState;
use crate::state::editor::Body;
use crate::theme::palette;
use crate::widgets;

/// What precedes a done task, one a line, until the list has its own editor.
const DONE: &str = "[x] ";

/// The body as the field holds it: a list's tasks one a line.
pub(super) fn text_of(note: &DisplayNote) -> String {
    if note.kind != NoteKind::Checklist {
        return note.content.clone();
    }
    note.items
        .iter()
        .map(|item| {
            if item.done {
                format!("{DONE}{}", item.text)
            } else {
                item.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The field read back; `None` for a drawing, whose scene is not text.
pub(super) fn body_of(note: &DisplayNote, text: &str) -> Option<Body> {
    match note.kind {
        NoteKind::Drawing => None,
        NoteKind::Checklist => Some(Body::Items(items_of(text))),
        NoteKind::Snippet | NoteKind::Note | NoteKind::Calc => Some(Body::Content(text.to_owned())),
    }
}

fn items_of(text: &str) -> Vec<ChecklistItem> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.strip_prefix(DONE) {
            Some(text) => ChecklistItem {
                text: text.to_owned(),
                done: true,
            },
            None => ChecklistItem {
                text: line.to_owned(),
                done: false,
            },
        })
        .collect()
}

/// The field for the note's kind; `true` once it is left.
pub(super) fn show(ui: &mut Ui, state: &AppState, note: &DisplayNote, body: &mut String) -> bool {
    if note.kind == NoteKind::Drawing {
        return false;
    }
    let p = palette(ui.ctx());
    let language = state.language;
    let (font, hint) = match note.kind {
        NoteKind::Note => (
            FontId::new(14.0, FontFamily::Proportional),
            keys::editor::EMPTY_RICH,
        ),
        _ => (
            FontId::new(13.0, FontFamily::Monospace),
            keys::editor::EMPTY_BODY,
        ),
    };
    let label = i18n::translate(language, keys::editor::CONTENT_LABEL, &[]);
    let size = ui.available_size();
    let response = egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_sized(
                size,
                TextEdit::multiline(body)
                    .font(font)
                    .frame(egui::Frame::new())
                    .text_color(p.text_0)
                    .hint_text(i18n::translate(language, hint, &[]))
                    .desired_width(f32::INFINITY),
            )
        })
        .inner;
    // Its name only: the field's own node carries what it holds.
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(label.clone()));
    widgets::address(ui, &response, "editor-body");
    response.lost_focus()
}

/// The footer's words: tasks done, words, or the language, lines, bytes and line ending.
pub(super) fn stats(note: &DisplayNote, typed: &str, language: Language) -> String {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    match note.kind {
        NoteKind::Checklist => {
            let items = items_of(typed);
            let done = items.iter().filter(|item| item.done).count();
            i18n::translate(
                language,
                keys::editor::CHECKLIST_STATS,
                &[
                    ("done", count(done).into()),
                    ("total", count(items.len()).into()),
                ],
            )
        }
        NoteKind::Note => i18n::translate(
            language,
            keys::editor::NOTE_STATS,
            &[("count", count(typed.split_whitespace().count()).into())],
        ),
        NoteKind::Drawing => String::new(),
        NoteKind::Snippet | NoteKind::Calc => {
            let head = if note.kind == NoteKind::Calc {
                i18n::translate(language, keys::notes::kind_tag::CALC, &[])
            } else {
                crate::notes::language::label(note.language).to_owned()
            };
            let eol = if typed.contains("\r\n") { "CRLF" } else { "LF" };
            let lines = i18n::translate(
                language,
                keys::editor::FOOTER_STATS,
                &[
                    ("lines", count(typed.split('\n').count()).into()),
                    ("bytes", count(typed.len()).into()),
                    ("eol", eol.into()),
                ],
            );
            format!("{head} · {lines}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_one_task_a_line_a_done_one_marked() {
        let items = items_of("milk\n[x] bread\n\n  \neggs");
        let read: Vec<_> = items
            .iter()
            .map(|item| (item.text.as_str(), item.done))
            .collect();
        assert_eq!(read, [("milk", false), ("bread", true), ("eggs", false)]);
    }
}
