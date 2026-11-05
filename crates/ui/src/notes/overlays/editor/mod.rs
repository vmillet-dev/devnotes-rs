//! The note editor over the canvas (the reference's `note-editor-overlay`): the toolbar with the
//! title, the header's bands — or their summary, folded — the body, and the footer. Every field
//! is a draft of this view, keyed on the opening's session, and written when it is left and on
//! every closing path, none of which leaves a field on its own.

mod attachments;
mod body;
mod calc;
mod checklist;
mod code;
mod drawing;
mod fields;
mod format;
mod header;
mod json_view;
mod revisions;
mod rich;
mod search_bar;

use std::sync::Arc;

use egui::{Align, CornerRadius, Id, Layout, Margin, Rect, Stroke, Ui, UiBuilder, Vec2};
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::{Action as Asked, Editing};
use crate::state::AppState;
use crate::state::editor::{Body, Edit, Opened};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, IconButton, Kind};

/// The reference's `--dialog-width` and `--dialog-height`.
const SIZE: Vec2 = Vec2::new(880.0, 640.0);
/// Under it, the toolbar's buttons keep their icon alone (the reference's container query).
const LABELS_FROM: f32 = 1_000.0;

/// What is being typed, for the opening it began in.
#[derive(Default)]
pub(crate) struct EditorView {
    session: u64,
    /// A kind changed in the same opening (code pasted into an empty Note) draws another body.
    kind: NoteKind,
    title: String,
    source: String,
    body: String,
    checklist: checklist::ChecklistView,
    /// A snippet's code, in the code editor.
    code: Option<code::CodeView>,
    /// A snippet whose language has a reader: its explorer.
    exploring: Option<json_view::Exploring>,
    /// A Note, formatted.
    rich: Option<rich::RichEditor>,
    /// A Calcul sheet, in its own editor.
    sheet: Option<calc::CalcView>,
    /// A Dessin's editor: its scene is no text.
    drawing: Option<drawing::DrawingView>,
    fields: fields::FieldsView,
    format: format::FormatView,
    /// The attachment whose cross was clicked once.
    removing: Option<String>,
    tag: String,
    /// The title takes the focus once, as the editor opens.
    focus: bool,
    /// Two steps rather than a dialog: the second click deletes.
    confirming_delete: bool,
}

impl EditorView {
    fn of(open: &Opened, note: &DisplayNote) -> Self {
        Self {
            session: open.session,
            kind: note.kind,
            title: note.title.clone(),
            source: note.source.clone(),
            body: body::text_of(note),
            checklist: checklist::ChecklistView::of(&note.items),
            code: (note.kind == NoteKind::Snippet)
                .then(|| code::CodeView::of(&note.content, note.language)),
            exploring: json_view::Exploring::of(note),
            rich: (note.kind == NoteKind::Note).then(|| rich::RichEditor::of(&note.content)),
            sheet: (note.kind == NoteKind::Calc).then(|| calc::CalcView::of(&note.content)),
            drawing: (note.kind == NoteKind::Drawing)
                .then(|| drawing::DrawingView::open(&note.content)),
            fields: fields::FieldsView::of(note),
            format: format::FormatView::default(),
            removing: None,
            tag: String::new(),
            focus: true,
            confirming_delete: false,
        }
    }

    /// What differs from the note as last read: the title, the context, the body, a tag typed.
    fn edit(&self, note: &DisplayNote) -> Edit {
        let mut tags = None;
        if !self.tag.trim().is_empty() {
            let mut more = note.tags.clone();
            more.push(self.tag.clone());
            tags = Some(more);
        }
        Edit {
            title: Some(self.title.clone()),
            source: Some(self.source.clone()),
            body: match &self.drawing {
                Some(drawing) => drawing.body().map(Body::Content),
                None => body::body_of(note, &self.body, &self.checklist.items),
            },
            tags,
            ..Edit::default()
        }
    }
}

pub(crate) use json_view::Layout as ExplorerLayout;

pub(crate) fn show(
    ui: &Ui,
    state: &AppState,
    view: &mut Option<EditorView>,
    layout: &mut ExplorerLayout,
    actions: &mut Actions,
) {
    let Some((open, note)) = state
        .editor
        .opened()
        .and_then(|open| open.note().map(|note| (open, note)))
    else {
        *view = None;
        return;
    };
    if view
        .as_ref()
        .is_none_or(|view| view.session != open.session || view.kind != note.kind)
    {
        *view = Some(EditorView::of(open, note));
    }
    let Some(editing) = view else { return };
    let ctx = ui.ctx();
    let p = palette(ctx);
    let fullscreen = state.settings.editor_fullscreen;
    let room = ctx.content_rect();
    let size = if fullscreen {
        room.size() - Vec2::splat(24.0)
    } else {
        SIZE.min(room.size() * 0.92)
    };
    // Escape first leaves the field being typed in; only a second one closes. egui drops the
    // focus before the frame is drawn, so the field is the one that had it at the last frame.
    let typing_id = Id::unique("note-editor.typing");
    let typing = ctx.data(|data| data.get_temp::<bool>(typing_id).unwrap_or(false));
    let escaped = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(if fullscreen { 10 } else { 14 }))
        .shadow(p.shadow_overlay);
    let shown = egui::Modal::new(Id::unique("note-editor"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_size(size);
            ui.set_max_size(size);
            panel(ui, state, (open.session, note), editing, layout, actions)
        });
    arrivals(ui, actions);
    let lightbox = state.attachments.zoomed().is_some();
    let dismissed = !lightbox && shown.should_close() && !(escaped && typing);
    // A field being typed in asks for the input method; a focused button asks for nothing.
    let focused = ctx.output(|output| output.ime.is_some());
    ctx.data_mut(|data| data.insert_temp(typing_id, focused));
    if shown.inner || dismissed {
        pictures(ui, open.session, editing, actions);
        missing_pictures(ui, open.session, note, editing, actions);
        close(open.session, note, editing, actions);
    }
}

/// The editor's column: toolbar, header, body, footer. `true` once the close button is clicked.
fn panel(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    layout: &mut ExplorerLayout,
    actions: &mut Actions,
) -> bool {
    let whole = ui.max_rect();
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    if editing.code.is_some() {
        format::adopt(ui, state, session, editing);
        format::shortcut(ui, state, (session, note), editing, actions);
    }
    let closed = band(ui, Margin::symmetric(20, 12), true, |ui| {
        toolbar(ui, state, (session, note), editing, actions)
    });
    if state.settings.editor_header_folded {
        band(ui, Margin::symmetric(20, 8), true, |ui| {
            header::summary(ui, state, note, actions);
        });
    } else {
        header::bands(ui, state, (session, note), editing, actions);
    }
    fields::show(ui, state, (session, note), editing, actions);
    if editing.code.is_some() {
        json_view::follow(&mut editing.exploring, note);
    }
    if let Some(exploring) = &mut editing.exploring {
        json_view::band(ui, state, session, exploring, layout, whole.width() - 40.0);
    }
    let footer_height = 34.0;
    let history = revisions::height(state);
    let body_rect = Rect::from_min_max(
        egui::pos2(whole.left(), ui.cursor().top()),
        egui::pos2(whole.right(), whole.bottom() - footer_height - history),
    );
    let mut body_ui =
        ui.new_child(UiBuilder::new().max_rect(body_rect.shrink2(Vec2::new(20.0, 14.0))));
    let filled = state
        .fill
        .preview(session)
        .filter(|_| editing.fields.previewing);
    let left = if let Some(filled) = filled {
        let lines: Vec<&str> = filled.lines().collect();
        widgets::code_viewer(&mut body_ui, "editor-preview", &lines);
        false
    } else if let Some(view) = &mut editing.drawing {
        let shown = drawing::show(&mut body_ui, view, &note.title, state.language);
        if let Some(format) = shown.export {
            let export = view.export(&note.title, format, state.language, palette(ui.ctx()));
            actions.push(editor(Editing::Export(export)));
        }
        if shown.left {
            pictures(ui, session, editing, actions);
        }
        shown.save
    } else if let Some(sheet) = &mut editing.sheet {
        let shown = calc::show(&mut body_ui, sheet, state, session, actions);
        if shown.changed {
            editing.body = sheet.text();
        }
        shown.left
    } else if let Some(code) = &mut editing.code {
        let shown = match &mut editing.exploring {
            Some(exploring) => {
                let at = (session, note);
                json_view::body(&mut body_ui, state, at, exploring, code, layout, actions)
            }
            None => code::show(&mut body_ui, code, note.language, state.language),
        };
        if shown.changed {
            editing.format.typed();
            editing.body = code.text();
        }
        shown.left
    } else if let Some(view) = &mut editing.rich {
        rich_body(
            &mut body_ui,
            state,
            (session, view),
            &mut editing.body,
            actions,
        )
    } else if note.kind == NoteKind::Checklist {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(&mut body_ui, |ui| {
                checklist::show(ui, &mut editing.checklist, state.language)
            })
            .inner
    } else {
        body::show(&mut body_ui, state, note, &mut editing.body)
    };
    if left {
        save(session, note, editing, actions);
    }
    format::notice(ui, state, body_rect, editing);
    under_body(ui, state, note, editing, (body_rect, history), actions);
    ui.advance_cursor_after_rect(whole);
    closed
}

/// The history, when open, then the footer: from under the body to the panel's bottom.
fn under_body(
    ui: &mut Ui,
    state: &AppState,
    note: &DisplayNote,
    editing: &EditorView,
    (body_rect, history): (Rect, f32),
    actions: &mut Actions,
) {
    let whole = ui.max_rect();
    let history_rect = Rect::from_min_max(
        egui::pos2(whole.left(), body_rect.bottom()),
        egui::pos2(whole.right(), body_rect.bottom() + history),
    );
    if history > 0.0 {
        let mut history_ui = ui.new_child(UiBuilder::new().max_rect(history_rect));
        revisions::show(&mut history_ui, state, actions);
    }
    let footer_rect =
        Rect::from_min_max(egui::pos2(whole.left(), history_rect.bottom()), whole.max);
    let mut footer_ui = ui.new_child(UiBuilder::new().max_rect(footer_rect));
    footer(&mut footer_ui, state, note, editing);
}

/// A Note's body: its Markdown kept once it changed, a link followed, code pasted into it empty
/// made a snippet. `true` once it is left.
fn rich_body(
    ui: &mut Ui,
    state: &AppState,
    (session, view): (u64, &mut rich::RichEditor),
    body: &mut String,
    actions: &mut Actions,
) -> bool {
    let shown = rich::show(ui, view, state.language, state.clipboard.as_ref());
    if let Some(code) = shown.code_paste {
        actions.push(editor(Editing::Convert {
            session,
            into: NoteKind::Snippet,
            edit: Edit {
                body: Some(Body::Content(code)),
                ..Edit::default()
            },
        }));
    }
    if let Some(markdown) = view.markdown().filter(|_| shown.changed) {
        *body = markdown;
    }
    if let Some(url) = shown.link {
        actions.push(editor(Editing::OpenLink(url)));
    }
    shown.left
}

/// A strip of the editor, a soft line under it.
pub(super) fn band<R>(
    ui: &mut Ui,
    margin: Margin,
    ruled: bool,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = palette(ui.ctx());
    let shown = egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_width(ui.available_width());
        content(ui)
    });
    if ruled {
        let rect = shown.response.rect;
        ui.painter()
            .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
    }
    shown.inner
}

fn toolbar(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) -> bool {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let labelled = ui.available_width() >= LABELS_FROM - 40.0;
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let close = tr(keys::editor::CLOSE);
            closed = IconButton::new(Icon::X, &close, "editor-close")
                .show(ui)
                .clicked();
            let fullscreen = state.settings.editor_fullscreen;
            let (icon, words) = if fullscreen {
                (Icon::Minimize, tr(keys::editor::EXIT_FULLSCREEN))
            } else {
                (Icon::Maximize, tr(keys::editor::FULLSCREEN))
            };
            if IconButton::new(icon, &words, "editor-fullscreen")
                .pressed(fullscreen)
                .show(ui)
                .clicked()
            {
                actions.push(editor(Editing::Fullscreen(!fullscreen)));
            }
            let folded = state.settings.editor_header_folded;
            let (icon, words) = if folded {
                (Icon::Unfold, tr(keys::editor::header::UNFOLD))
            } else {
                (Icon::Fold, tr(keys::editor::header::FOLD))
            };
            if IconButton::new(icon, &words, "editor-fold-header")
                .show(ui)
                .clicked()
            {
                if !folded {
                    save(session, note, editing, actions);
                    editing.tag.clear();
                }
                actions.push(editor(Editing::FoldHeader(!folded)));
            }
            delete(ui, state, editing, labelled, actions);
            if note.kind != NoteKind::Drawing {
                copy(ui, state, (session, note), editing, labelled, actions);
            }
            priority(ui, state, note, actions);
            pin(ui, state, note, labelled, actions);
            if note.kind.has_language() {
                format::button(ui, state, (session, note), editing, labelled, actions);
                language_menu(ui, state, note, actions);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                title(ui, state, (session, note), editing, actions);
            });
        });
    });
    closed
}

fn title(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let name = i18n::translate(language, keys::editor::TITLE_LABEL, &[]);
    let untitled = i18n::translate(language, keys::notes::UNTITLED, &[]);
    let response = ui.add(
        egui::TextEdit::singleline(&mut editing.title)
            .frame(egui::Frame::new())
            .margin(Margin::ZERO)
            .hint_text(untitled)
            .text_color(p.text_0)
            .font(egui::FontId::new(16.0, egui::FontFamily::Proportional))
            .desired_width(ui.available_width().max(120.0)),
    );
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.clone()));
    widgets::address(ui, &response, "editor-title");
    if std::mem::take(&mut editing.focus) {
        response.request_focus();
    }
    if response.lost_focus() {
        save(session, note, editing, actions);
    }
}

fn pin(ui: &mut Ui, state: &AppState, note: &DisplayNote, labelled: bool, actions: &mut Actions) {
    let key = if note.pinned {
        keys::editor::PINNED
    } else {
        keys::editor::PIN
    };
    let words = i18n::translate(state.language, key, &[]);
    let clicked = if labelled {
        let kind = if note.pinned {
            Kind::Filled
        } else {
            Kind::Outline
        };
        Button::new(&words, "editor-pin")
            .kind(kind)
            .icon(Icon::Pin)
            .show(ui)
            .clicked()
    } else {
        IconButton::new(Icon::Pin, &words, "editor-pin")
            .pressed(note.pinned)
            .show(ui)
            .clicked()
    };
    if clicked {
        actions.push(editor(Editing::Pin(!note.pinned)));
    }
}

fn priority(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    use engine::notes::priority::Priority;
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let names: Vec<(Priority, String)> = Priority::ALL
        .into_iter()
        .map(|level| {
            let name = if level == Priority::None {
                tr(keys::editor::NO_PRIORITY)
            } else {
                tr(crate::notes::priority_section(level))
            };
            (level, name)
        })
        .collect();
    let options: Vec<(Priority, &str)> = names
        .iter()
        .map(|(level, name)| (*level, name.as_str()))
        .collect();
    let mut chosen = note.priority;
    let label = tr(keys::notes::PRIORITY_LABEL);
    if ChoiceMenu::new(&label, "editor-priority", &options)
        .show(ui, &mut chosen)
        .changed()
    {
        actions.push(editor(Editing::Prioritise(chosen)));
    }
}

fn language_menu(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    use engine::notes::language::Language;
    let options: Vec<(Language, &str)> = Language::ALL
        .into_iter()
        .map(|language| (language, crate::notes::language::label(language)))
        .collect();
    let mut chosen = note.language;
    let label = i18n::translate(state.language, keys::editor::LANGUAGE_LABEL, &[]);
    if ChoiceMenu::new(&label, "editor-language", &options)
        .show(ui, &mut chosen)
        .changed()
    {
        actions.push(editor(Editing::Save {
            session: state.editor.opened().map_or(0, |open| open.session),
            edit: Edit {
                language: Some(chosen),
                ..Edit::default()
            },
        }));
    }
}

/// What is typed goes first: the copy is read from the note as written.
fn copy(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &EditorView,
    labelled: bool,
    actions: &mut Actions,
) {
    // A note with fields copies filled, from the values in the panel; « copy as is » stays there.
    let (icon, key, author_id) = if note.placeholders.is_empty() {
        (Icon::Copy, keys::notes::COPY, "editor-copy")
    } else {
        (Icon::Zap, keys::placeholders::COPY, "editor-copy-filled")
    };
    let words = i18n::translate(state.language, key, &[]);
    let clicked = if labelled {
        Button::new(&words, author_id).icon(icon).show(ui).clicked()
    } else {
        IconButton::new(icon, &words, author_id).show(ui).clicked()
    };
    if clicked {
        save(session, note, editing, actions);
        if note.placeholders.is_empty() {
            actions.push(editor(Editing::Copy));
        } else {
            actions.push(editor(Editing::Fields(crate::notes::Fields::Copy {
                content: editing.body.clone(),
                values: editing.fields.values(note),
            })));
        }
    }
}

fn delete(
    ui: &mut Ui,
    state: &AppState,
    editing: &mut EditorView,
    labelled: bool,
    actions: &mut Actions,
) {
    let confirming = editing.confirming_delete;
    let key = if confirming {
        keys::editor::DELETE_CONFIRM
    } else {
        keys::editor::DELETE
    };
    let words = i18n::translate(state.language, key, &[]);
    let clicked = if labelled || confirming {
        Button::new(&words, "editor-delete")
            .kind(Kind::Destructive)
            .armed(confirming)
            .icon(Icon::Trash)
            .show(ui)
            .clicked()
    } else {
        Button::new("", "editor-delete")
            .kind(Kind::Destructive)
            .icon(Icon::Trash)
            .show(ui)
            .on_hover_text(&words)
            .clicked()
    };
    if clicked {
        if confirming {
            actions.push(editor(Editing::Delete));
        }
        editing.confirming_delete = !confirming;
    }
}

fn footer(ui: &mut Ui, state: &AppState, note: &DisplayNote, editing: &EditorView) {
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line_soft));
    let language = state.language;
    let shapes = editing
        .drawing
        .as_ref()
        .map_or(0, drawing::DrawingView::shapes);
    let counted = body::stats(
        note,
        &editing.body,
        &editing.checklist.items,
        shapes,
        language,
    );
    let modified = (!note.id.is_empty()).then(|| {
        let (said, _) = crate::services::time::relative(note.updated_at, state.clock.as_ref());
        format!(
            "{} {}",
            i18n::translate(language, keys::editor::MODIFIED_PREFIX, &[]),
            said.text(language)
        )
    });
    let mut inner = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(Vec2::new(20.0, 0.0))));
    inner.horizontal_centered(|ui| {
        let galley = widgets::text(
            ui,
            &counted,
            11.0,
            400.0,
            egui::FontFamily::Monospace,
            p.text_2,
        );
        let response = ui.label(galley);
        widgets::address(ui, &response, "editor-stats");
        if let Some(modified) = modified {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let galley = widgets::text(
                    ui,
                    &modified,
                    11.0,
                    400.0,
                    egui::FontFamily::Monospace,
                    p.text_2,
                );
                ui.label(galley);
            });
        }
    });
}

/// The fields as they are, written: on blur, before a copy, a fold or a conversion.
pub(super) fn save(session: u64, note: &DisplayNote, editing: &EditorView, actions: &mut Actions) {
    actions.push(editor(Editing::Save {
        session,
        edit: editing.edit(note),
    }));
}

/// ⚠️ The one closing path, and it writes the drafts first: Escape, the backdrop and the close
/// button leave no field on their own.
fn close(session: u64, note: &DisplayNote, editing: &mut EditorView, actions: &mut Actions) {
    save(session, note, editing, actions);
    editing.fields.keep(note, actions);
    actions.push(editor(Editing::Close));
}

/// What reaches the window while a note is open: a file dropped, an image pasted. A text pasted
/// is the field's.
fn arrivals(ui: &Ui, actions: &mut Actions) {
    use crate::state::attachments::Asked;
    let (dropped, pasted) = ui.input(|input| {
        let dropped: Vec<_> = input
            .raw
            .dropped_files
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect();
        let pasted: Vec<_> = input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::PasteImage(image) => Some(Arc::clone(image)),
                _ => None,
            })
            .collect();
        (dropped, pasted)
    });
    if !dropped.is_empty() {
        actions.push(editor(Editing::Attachment(Asked::Dropped(dropped))));
    }
    for image in pasted {
        actions.push(editor(Editing::Attachment(Asked::Pasted(image))));
    }
}

/// What was drawn since they were last taken: two pictures are dear, so only on leaving.
fn pictures(ui: &Ui, session: u64, editing: &mut EditorView, actions: &mut Actions) {
    let owed = editing.drawing.as_mut();
    if let Some(content) = owed.and_then(drawing::DrawingView::take_unpictured) {
        actions.push(editor(Editing::Pictures {
            session,
            content,
            palette: Arc::new(drawing::picture_palette(palette(ui.ctx()))),
            missing: false,
        }));
    }
}

/// A drawing that came by import has its scene and no picture: it takes them on being left,
/// and no date moves.
fn missing_pictures(
    ui: &Ui,
    session: u64,
    note: &DisplayNote,
    editing: &EditorView,
    actions: &mut Actions,
) {
    let drawn = editing.drawing.as_ref();
    let content = drawn.and_then(drawing::DrawingView::content_if_drawn);
    if let Some(content) = content.filter(|_| !note.id.is_empty()) {
        actions.push(editor(Editing::Pictures {
            session,
            content,
            palette: Arc::new(drawing::picture_palette(palette(ui.ctx()))),
            missing: true,
        }));
    }
}

pub(super) fn editor(editing: Editing) -> Action {
    Action::Notes(Asked::Editor(editing))
}

pub(in crate::notes::overlays) use attachments::{fitted, paint};

#[cfg(test)]
mod tests;
