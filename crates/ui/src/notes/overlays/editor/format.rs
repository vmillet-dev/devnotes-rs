//! Formatting a snippet (the reference's `format-button`): the button and the library's
//! settings beside it, Shift+Alt+F from anywhere in the editor, and what a format says once
//! done — the lines it changed marked in the gutter until the next keystroke, and a notice
//! whose « Undo » is the field's own step. The core's answer is applied only to the text it
//! was asked about: typing meanwhile wins.

use egui::{
    Align, CornerRadius, Key, Layout, Modifiers, Popup, PopupCloseBehavior, Rect, Stroke,
    StrokeKind, Ui, UiBuilder, pos2, vec2,
};
use engine::notes::language::Language;
use engine::notes::model::DisplayNote;
use i18n::keys;
use text::format::{Answer, Request};

use super::{EditorView, editor};
use crate::action::Actions;
use crate::notes::Editing;
use crate::notes::language;
use crate::notes::prettier::{Indentation, clamp_width};
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, Checkbox, IconButton, Segmented, TextField};

/// How long a notice stays; the marked lines stay until the next keystroke.
const NOTICE_FOR: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notice {
    /// How many lines it changed.
    Formatted(usize),
    Unchanged,
    Syntax {
        line: u32,
        column: u32,
    },
    Fields,
    Failed,
}

impl Notice {
    /// What the e2e driver reads: no translation touches it.
    fn kind(self) -> &'static str {
        match self {
            Notice::Formatted(_) => "formatted",
            Notice::Unchanged => "unchanged",
            Notice::Syntax { .. } => "syntax",
            Notice::Fields => "fields",
            Notice::Failed => "failed",
        }
    }
}

#[derive(Default)]
pub(super) struct FormatView {
    /// The last answer applied.
    seen: u64,
    /// The text sent: an answer about another one is dropped.
    sent: Option<String>,
    notice: Option<(Notice, f64)>,
    /// The width as it is typed, applied when the field is left.
    width: Option<String>,
}

impl FormatView {
    /// Typed into: the notice goes, as the marks do.
    pub(super) fn typed(&mut self) {
        self.notice = None;
    }
}

/// Whether the snippet's code can be formatted now: a language Prettier reads, and the code on
/// screen, not the filled preview or the explorer.
fn ready(state: &AppState, (session, note): (u64, &DisplayNote), editing: &EditorView) -> bool {
    language::formatter(note.language).is_some()
        && !state.formatting.busy(session)
        && !editing.fields.previewing
        && editing
            .exploring
            .as_ref()
            .is_none_or(super::json_view::Exploring::shows_code)
}

/// The button and its chevron, in the toolbar drawn from the right.
pub(super) fn button(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    labelled: bool,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let settings_label = tr(keys::editor::format::SETTINGS);
    let chevron =
        IconButton::new(Icon::ChevronDown, &settings_label, "editor-format-settings").show(ui);
    let hint = if language::formatter(note.language).is_some() {
        tr(keys::editor::format::HINT)
    } else {
        i18n::translate(
            language,
            keys::editor::format::UNSUPPORTED,
            &[("language", language::name(note.language).into())],
        )
    };
    let enabled = ready(state, (session, note), editing);
    let label = if labelled {
        tr(keys::editor::format::LABEL)
    } else {
        String::new()
    };
    let clicked = Button::new(&label, "editor-format")
        .icon(Icon::Format)
        .enabled(enabled)
        .show(ui);
    // Its icon alone in a narrow toolbar: the name stays.
    let name = tr(keys::editor::format::LABEL);
    ui.ctx()
        .accesskit_node_builder(clicked.id, |node| node.set_label(name));
    if widgets::tooltip(clicked, &hint).clicked() {
        ask(state, (session, note), editing, actions);
    }
    Popup::from_toggle_button_response(&chevron)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(widgets::menu_frame(ui.ctx()))
        .show(|ui| panel(ui, state, &mut editing.format.width, actions));
}

/// Shift+Alt+F from anywhere in the editor.
pub(super) fn shortcut(
    ui: &Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let pressed =
        ui.input_mut(|input| input.consume_key(Modifiers::SHIFT | Modifiers::ALT, Key::F));
    if pressed && ready(state, (session, note), editing) {
        ask(state, (session, note), editing, actions);
    }
}

fn ask(
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let (Some(code), Some(parser)) = (&editing.code, language::formatter(note.language)) else {
        return;
    };
    let text = code.text();
    let request = Request {
        text: text.clone(),
        cursor: code.editor.buffer.selection().head,
        parser,
        options: state.prettier.options(language::indent_unit(note.language)),
    };
    editing.format.sent = Some(text);
    actions.push(editor(Editing::Format { session, request }));
}

/// The answer that arrived since the last frame, applied through the field's own history.
pub(super) fn adopt(ui: &Ui, state: &AppState, session: u64, editing: &mut EditorView) {
    let Some(answered) = state.formatting.answered(session) else {
        return;
    };
    if answered.seq <= editing.format.seen {
        return;
    }
    editing.format.seen = answered.seq;
    let Some(code) = &mut editing.code else {
        return;
    };
    let now = ui.input(|input| input.time);
    let text = code.text();
    if answered.quiet {
        // Formatted on save, and written so: the field follows unless it was typed into since.
        if let Answer::Formatted {
            text: formatted, ..
        } = &answered.answer
            && *formatted != text
            && editing.body == text
        {
            code.editor.replace_all(formatted);
            code.edited(0);
            editing.body.clone_from(formatted);
        }
        return;
    }
    if editing.format.sent.take().as_ref() != Some(&text) {
        return;
    }
    let notice = match &answered.answer {
        Answer::Formatted {
            text: formatted,
            cursor,
            changed_lines,
        } => {
            code.editor.replace_all(formatted);
            code.edited(0);
            code.editor.select(*cursor..*cursor);
            code.marked.clone_from(changed_lines);
            editing.body.clone_from(formatted);
            Notice::Formatted(changed_lines.len())
        }
        Answer::Unchanged => Notice::Unchanged,
        Answer::Syntax { line, column } => {
            let at = offset_of(&code.editor.buffer, *line, *column);
            code.editor.select(at..at);
            Notice::Syntax {
                line: *line,
                column: *column,
            }
        }
        Answer::Fields => Notice::Fields,
        Answer::Failed => Notice::Failed,
    };
    editing.format.notice = Some((notice, now + NOTICE_FOR));
}

/// Prettier's one-based line and column, as a `char` offset into the text.
fn offset_of(buffer: &text::Buffer, line: u32, column: u32) -> usize {
    let line = usize::try_from(line.saturating_sub(1)).unwrap_or(0);
    if line >= buffer.line_count() {
        return buffer.len();
    }
    let column = usize::try_from(column.saturating_sub(1)).unwrap_or(0);
    (buffer.line_start(line) + column).min(buffer.line_end(line))
}

/// The notice over the bottom of the body, with its « Undo » after a format.
pub(super) fn notice(ui: &mut Ui, state: &AppState, body: Rect, editing: &mut EditorView) {
    let Some((notice, until)) = editing.format.notice else {
        return;
    };
    let now = ui.input(|input| input.time);
    if now >= until {
        editing.format.notice = None;
        return;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(until - now));
    let p = palette(ui.ctx());
    let language = state.language;
    let words = match notice {
        Notice::Formatted(changed) => format!(
            "{} · {}",
            i18n::translate(language, keys::editor::format::DONE, &[]),
            i18n::translate(
                language,
                keys::editor::format::CHANGED,
                &[("count", u32::try_from(changed).unwrap_or(u32::MAX).into())],
            )
        ),
        Notice::Unchanged => i18n::translate(language, keys::editor::format::UNCHANGED, &[]),
        Notice::Syntax { line, column } => i18n::translate(
            language,
            keys::editor::format::SYNTAX,
            &[("line", line.into()), ("column", column.into())],
        ),
        Notice::Fields => i18n::translate(language, keys::editor::format::FIELDS, &[]),
        Notice::Failed => i18n::translate(language, keys::editor::format::FAILED, &[]),
    };
    let failed = matches!(
        notice,
        Notice::Syntax { .. } | Notice::Fields | Notice::Failed
    );
    let undo = i18n::translate(language, keys::editor::format::UNDO, &[]);
    let galley = widgets::text(
        ui,
        &words,
        12.5,
        400.0,
        egui::FontFamily::Proportional,
        p.text_0,
    );
    let undo_room = if matches!(notice, Notice::Formatted(_)) {
        Button::width(ui, &undo) + 12.0
    } else {
        0.0
    };
    let size = vec2(galley.size().x + undo_room + 28.0, 40.0);
    let rect = Rect::from_center_size(
        pos2(body.center().x, body.bottom() - 16.0 - size.y / 2.0),
        size,
    );
    let edge = if failed {
        p.red.gamma_multiply(0.5)
    } else {
        p.line
    };
    ui.painter()
        .add(p.shadow_overlay.as_shape(rect, CornerRadius::same(10)));
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        p.bg_3,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let mut inner = ui.new_child(
        UiBuilder::new()
            .id_salt("editor-format-notice")
            .max_rect(rect.shrink2(vec2(14.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    inner.spacing_mut().item_spacing.x = 12.0;
    let said = inner.label(galley);
    widgets::address(&inner, &said, "editor-format-notice");
    let kind = notice.kind();
    inner
        .ctx()
        .accesskit_node_builder(said.id, |node| node.set_description(kind));
    if matches!(notice, Notice::Formatted(_))
        && Button::new(&undo, "editor-format-undo")
            .kind(widgets::Kind::Quiet)
            .show(&mut inner)
            .clicked()
        && let Some(code) = &mut editing.code
        && let Some(line) = code.editor.undo()
    {
        code.edited(line);
        editing.body = code.text();
        editing.format.notice = None;
    }
}

/// The library's settings, applied as they are chosen: the panel has no button to confirm.
fn panel(ui: &mut Ui, state: &AppState, width: &mut Option<String>, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let now = state.prettier;
    let mut next = now;
    ui.set_width(300.0);
    ui.spacing_mut().item_spacing.y = 10.0;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Prettier").strong().color(p.text_0));
        ui.label(
            egui::RichText::new(tr(keys::editor::format::SCOPE))
                .size(11.5)
                .color(p.text_2),
        );
    });
    setting(ui, &tr(keys::editor::format::PRINT_WIDTH), |ui| {
        let typed = width.get_or_insert_with(|| now.print_width.to_string());
        let field = TextField::new(
            typed,
            &tr(keys::editor::format::PRINT_WIDTH),
            "prettier-width",
        )
        .compact()
        .show(ui);
        if field.lost_focus() {
            if let Some(clamped) = typed.trim().parse::<f64>().ok().and_then(clamp_width) {
                next.print_width = clamped;
            }
            *width = None;
        }
    });
    let indentations: Vec<(Indentation, String)> = Indentation::ALL
        .into_iter()
        .map(|each| (each, tr(indentation_key(each))))
        .collect();
    let options: Vec<(Indentation, &str)> = indentations
        .iter()
        .map(|(each, name)| (*each, name.as_str()))
        .collect();
    setting(ui, &tr(keys::editor::format::INDENTATION), |ui| {
        Segmented::new(&options, "prettier-indentation").show(ui, &mut next.indentation);
    });
    let single = tr(keys::editor::format::QUOTES_SINGLE);
    let double = tr(keys::editor::format::QUOTES_DOUBLE);
    let quotes = [(true, single.as_str()), (false, double.as_str())];
    setting(ui, &tr(keys::editor::format::QUOTES), |ui| {
        Segmented::new(&quotes, "prettier-quotes").show(ui, &mut next.single_quote);
    });
    for (key, author_id, value) in [
        (
            keys::editor::format::SEMICOLONS,
            "prettier-semicolons",
            &mut next.semicolons,
        ),
        (
            keys::editor::format::TRAILING_COMMAS,
            "prettier-trailingCommas",
            &mut next.trailing_commas,
        ),
        (
            keys::editor::format::FORMAT_ON_SAVE,
            "prettier-formatOnSave",
            &mut next.format_on_save,
        ),
    ] {
        Checkbox::new(value, &tr(key), author_id).show(ui);
    }
    let formatted: Vec<&str> = Language::ALL
        .into_iter()
        .filter(|each| language::formatter(*each).is_some())
        .map(language::name)
        .collect();
    let list = i18n::translate(
        language,
        keys::editor::format::FORMATTED_LIST,
        &[("list", formatted.join(", ").into())],
    );
    note(ui, &tr(keys::editor::format::FORMATTED_TITLE), &list);
    note(
        ui,
        &tr(keys::editor::format::NOT_FORMATTED_TITLE),
        &tr(keys::editor::format::NOT_FORMATTED_LIST),
    );
    if next != now {
        actions.push(editor(Editing::Prettier(next)));
    }
}

fn indentation_key(indentation: Indentation) -> i18n::Key {
    use keys::editor::format::indentations;
    match indentation {
        Indentation::Editor => indentations::EDITOR,
        Indentation::Two => indentations::TWO,
        Indentation::Four => indentations::FOUR,
        Indentation::Tab => indentations::TAB,
    }
}

fn setting(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.5).color(p.text_1));
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}

fn note(ui: &mut Ui, title: &str, words: &str) {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::proportional(11.5);
    job.append(
        &format!("{title} "),
        0.0,
        egui::TextFormat::simple(font.clone(), p.text_1),
    );
    job.append(words, 0.0, egui::TextFormat::simple(font, p.text_2));
    ui.add(egui::Label::new(job).wrap());
}

#[cfg(test)]
mod tests;
