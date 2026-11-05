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

/// The body as its field holds it; a list's items have a field each.
pub(super) fn text_of(note: &DisplayNote) -> String {
    note.content.clone()
}

/// The field read back: a list's items as its rows hold them; `None` for a drawing, whose scene
/// is not text.
pub(super) fn body_of(note: &DisplayNote, text: &str, items: &[ChecklistItem]) -> Option<Body> {
    match note.kind {
        NoteKind::Drawing => None,
        NoteKind::Checklist => Some(Body::Items(items.to_vec())),
        NoteKind::Snippet | NoteKind::Note | NoteKind::Calc => Some(Body::Content(text.to_owned())),
    }
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
pub(super) fn stats(
    note: &DisplayNote,
    typed: &str,
    items: &[ChecklistItem],
    shapes: usize,
    language: Language,
) -> String {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    match note.kind {
        NoteKind::Checklist => {
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
            &[("count", count(text::count_words(typed)).into())],
        ),
        NoteKind::Drawing => i18n::translate(
            language,
            keys::editor::DRAWING_STATS,
            &[("count", count(shapes).into())],
        ),
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
