//! The editor over the whole area: the way back and the keys, a toolbar with the steps and the
//! kind's own action, the body laid out for its kind, the details in a column beside it unless
//! put away, and the status line along the foot.

use egui::{Align, Layout, Margin, Rect, Stroke, Ui, UiBuilder, Vec2, pos2};
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;
use i18n::keys;

use super::{
    EditorView, ExplorerLayout, band, body, close_button, copy, drawing, editor, expand_button,
    fields, format, header, json_view, language_menu, more_menu, pin, revisions, save, title,
};
use crate::action::Actions;
use crate::notes::Editing;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind};

const DETAILS_WIDTH: f32 = 300.0;
/// The details give way rather than leave the body less: a JSON graph beside its code needs it.
const BODY_MIN: f32 = 720.0;
const STATUS_HEIGHT: f32 = 34.0;
/// A task list reads down a column, not across the screen.
const TASKS_WIDTH: f32 = 760.0;
/// A snippet with fields shows itself filled under its code, on this share of the height.
const FILLED_SHARE: f32 = 0.4;

/// The whole of it. `true` once the close button is clicked.
pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    (editing, layout): (&mut EditorView, &mut ExplorerLayout),
    actions: &mut Actions,
) -> bool {
    let whole = ui.max_rect();
    band(ui, Margin::symmetric(20, 8), true, |ui| {
        way_back(ui, state, note, actions);
    });
    let closed = band(ui, Margin::symmetric(20, 12), true, |ui| {
        toolbar(ui, state, (session, note), editing, actions)
    });
    // Too narrow for the column, the header's bands hold what it would.
    if !fits_details(whole.width()) {
        super::header(ui, state, (session, note), editing, actions);
    }
    let top = ui.cursor().top();
    let foot = whole.bottom() - STATUS_HEIGHT;
    let details = shows_details(state, whole.width());
    let split = if details {
        whole.right() - DETAILS_WIDTH
    } else {
        whole.right()
    };
    if details {
        let column = Rect::from_min_max(pos2(split, top), pos2(whole.right(), foot));
        let line = Stroke::new(1.0, palette(ui.ctx()).line_soft);
        ui.painter().vline(split, column.y_range(), line);
        let mut column_ui = child(ui, column, "editor-details");
        header::details(&mut column_ui, state, (session, note), editing, actions);
    }
    let main = Rect::from_min_max(pos2(whole.left(), top), pos2(split, foot));
    let mut main_ui = child(ui, main, "editor-main");
    let history_below = !details;
    main_column(
        &mut main_ui,
        state,
        (session, note),
        (editing, layout),
        history_below,
        actions,
    );
    let status = Rect::from_min_max(pos2(whole.left(), foot), whole.max);
    super::footer(
        &mut child(ui, status, "editor-status"),
        state,
        note,
        editing,
    );
    ui.advance_cursor_after_rect(whole);
    closed
}

/// Unless put away, and where the body keeps its room beside them.
fn shows_details(state: &AppState, width: f32) -> bool {
    state.settings.editor_details_open && fits_details(width)
}

fn fits_details(width: f32) -> bool {
    width - DETAILS_WIDTH >= BODY_MIN
}

fn child(ui: &mut Ui, rect: Rect, salt: &str) -> Ui {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(salt)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.spacing_mut().item_spacing = Vec2::ZERO;
    child
}

/// The fields over the code and the code's own strip, then the body — a snippet with fields
/// filled under it — and the history, when the details do not hold it.
fn main_column(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    (editing, layout): (&mut EditorView, &mut ExplorerLayout),
    history_below: bool,
    actions: &mut Actions,
) {
    let whole = ui.max_rect();
    fields::show(ui, state, (session, note), editing, actions);
    if editing.code.is_some() {
        json_view::follow(&mut editing.exploring, note);
    }
    if let Some(exploring) = &mut editing.exploring {
        json_view::band(ui, state, session, exploring, layout, whole.width() - 40.0);
    }
    if note.kind.has_language() {
        band(ui, Margin::symmetric(20, 8), true, |ui| {
            code_strip(ui, state, (session, note), editing, actions);
        });
    }
    let top = ui.cursor().top();
    let history = if history_below {
        revisions::height(state, whole.bottom() - top - super::BODY_FLOOR)
    } else {
        0.0
    };
    let bottom = whole.bottom() - history;
    let filled = !note.placeholders.is_empty() && editing.exploring.is_none();
    let code_bottom = if filled {
        (bottom - (bottom - top) * FILLED_SHARE).round()
    } else {
        bottom
    };
    let body_rect = Rect::from_min_max(pos2(whole.left(), top), pos2(whole.right(), code_bottom));
    let mut inner = body_rect.shrink2(Vec2::new(20.0, 14.0));
    if note.kind == NoteKind::Checklist && inner.width() > TASKS_WIDTH {
        inner = Rect::from_center_size(inner.center(), Vec2::new(TASKS_WIDTH, inner.height()));
    }
    let mut body_ui = ui.new_child(UiBuilder::new().max_rect(inner));
    body_ui.set_clip_rect(body_ui.clip_rect().intersect(body_rect));
    let typed = editing.body.clone();
    let left = body(
        &mut body_ui,
        state,
        (session, note),
        (editing, layout),
        false,
        actions,
    );
    if left {
        save(session, note, editing, actions);
    }
    format::notice(ui, state, body_rect, editing);
    if filled {
        let rect = Rect::from_min_max(pos2(whole.left(), code_bottom), pos2(whole.right(), bottom));
        filled_below(ui, state, (session, note), editing, rect, actions);
        if editing.body != typed {
            fields::preview(session, note, editing, actions);
        }
    }
    if history > 0.0 {
        let rect = Rect::from_min_max(pos2(whole.left(), bottom), whole.max);
        revisions::show(
            &mut ui.new_child(UiBuilder::new().max_rect(rect)),
            state,
            actions,
        );
    }
}

/// Over the code: its language and the formatter, where the toolbar beside the cards has them.
fn code_strip(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        language_menu(ui, state, note, actions);
        format::button(ui, state, (session, note), editing, true, actions);
    });
}

/// The snippet as its fields fill it, asked for once and again at every change.
fn filled_below(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    rect: Rect,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    if state.fill.preview(session).is_none() && !editing.fields.asked {
        editing.fields.asked = true;
        fields::preview(session, note, editing, actions);
    }
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line_soft));
    let mut area = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(20.0, 10.0)))
            .layout(Layout::top_down(Align::Min)),
    );
    area.set_clip_rect(rect.intersect(ui.clip_rect()));
    // The toolbar's copy already fills the fields: the heading stands alone.
    let heading = i18n::translate(state.language, keys::editor::FILLED_PREVIEW, &[]);
    let said = area.label(widgets::text(
        &area,
        &heading.to_uppercase(),
        10.5,
        600.0,
        egui::FontFamily::Proportional,
        p.text_2,
    ));
    widgets::address(&area, &said, "editor-filled");
    area.add_space(8.0);
    if let Some(filled) = state.fill.preview(session) {
        let lines: Vec<&str> = filled.lines().collect();
        widgets::code_viewer(&mut area, "editor-filled", &lines);
    }
}

fn toolbar(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) -> bool {
    let mut closed = false;
    // The band's margins are the panel's: the whole width, as `show` measures it.
    let width = ui.available_width() + 40.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            closed = close_button(ui, state);
            expand_button(ui, state, actions);
            // The header is there to fold only where the details have no room.
            let foldable = !fits_details(width);
            more_menu(ui, state, (session, note), (editing, foldable), actions);
            pin(ui, state, note, false, actions);
            details_button(ui, state, fits_details(width), actions);
            primary(ui, state, (session, note), editing, actions);
            ui.add_space(8.0);
            steps(ui, state, actions);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                crate::notes::ui::kind_icon(ui, note.kind, state.language);
                crate::notes::ui::format_label(ui, note.kind, note.language, state.language);
                title(ui, state, (session, note), editing, actions);
            });
        });
    });
    closed
}

/// What the kind is opened for: a drawing is exported, the others copied.
fn primary(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let Some(view) = &editing.drawing else {
        copy(ui, state, (session, note), editing, true, actions);
        return;
    };
    let words = i18n::translate(state.language, keys::editor::EXPORT, &[]);
    let trigger = Button::new(&words, "editor-export")
        .icon(Icon::ChevronDown)
        .enabled(view.shapes() > 0)
        .show(ui);
    let mut chosen = None;
    egui::Popup::menu(&trigger)
        .frame(widgets::menu_frame(ui.ctx()))
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            chosen = drawing::exports(ui, state.language);
        });
    if let Some(format) = chosen {
        super::export(ui, note, view, format, state, actions);
    }
}

/// « Details »: the column beside the body, shown or put away for every kind. Disabled where
/// there is no room for it.
fn details_button(ui: &mut Ui, state: &AppState, room: bool, actions: &mut Actions) {
    let open = state.settings.editor_details_open;
    let key = if open {
        keys::editor::details::HIDE
    } else {
        keys::editor::details::SHOW
    };
    let words = i18n::translate(state.language, keys::editor::details::SHOW, &[]);
    let hint = i18n::translate(state.language, key, &[]);
    let icon = if open {
        Icon::ChevronRight
    } else {
        Icon::ChevronLeft
    };
    let clicked = Button::new(&words, "editor-details")
        .kind(Kind::Outline)
        .icon(icon)
        .armed(open && room)
        .enabled(room)
        .show(ui);
    if widgets::tooltip(clicked, &hint).clicked() {
        actions.push(editor(Editing::Details(!open)));
    }
}

/// Over the expanded editor: the way back to the cards, where the note is, and the keys.
fn way_back(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(state.language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let back = tr(keys::editor::BACK_TO_CARDS);
        if Button::new(&back, "editor-back-to-cards")
            .kind(Kind::Quiet)
            .icon(Icon::ArrowLeft)
            .show(ui)
            .clicked()
        {
            actions.push(editor(Editing::Fullscreen(false)));
        }
        let place = place_of(state, note);
        let shown = ui.label(egui::RichText::new(&place).size(12.5).color(p.text_2));
        widgets::address(ui, &shown, "editor-breadcrumb");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let hint = tr(keys::editor::STEP_HINT);
            ui.label(
                egui::RichText::new(hint)
                    .size(11.5)
                    .family(egui::FontFamily::Monospace)
                    .color(p.text_2),
            );
        });
    });
}

/// The note's space, then its folders down to its own.
fn place_of(state: &AppState, note: &DisplayNote) -> String {
    let space = state
        .spaces
        .spaces()
        .iter()
        .find(|space| space.id == note.space_id)
        .map(|space| space.name.clone());
    let folders = note
        .folder
        .iter()
        .flat_map(|folder| folder.path.iter().cloned());
    space
        .into_iter()
        .chain(folders)
        .collect::<Vec<_>>()
        .join(" › ")
}

/// ‹ n / total ›: where the note open sits among the cards shown, and the way to its neighbours.
fn steps(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let Some((at, order)) = state.editor_position() else {
        return;
    };
    let tr = |key| i18n::translate(state.language, key, &[]);
    let next = ui.add_enabled_ui(at + 1 < order.len(), |ui| {
        IconButton::new(
            Icon::ChevronRight,
            &tr(keys::editor::NEXT_NOTE),
            "editor-next",
        )
        .show(ui)
    });
    if next.inner.clicked() {
        actions.push(editor(Editing::Step(1)));
    }
    let position = i18n::translate(
        state.language,
        keys::editor::POSITION,
        &[
            ("at", u32::try_from(at + 1).unwrap_or(u32::MAX).into()),
            (
                "total",
                u32::try_from(order.len()).unwrap_or(u32::MAX).into(),
            ),
        ],
    );
    let shown = ui.label(
        egui::RichText::new(position)
            .size(12.0)
            .family(egui::FontFamily::Monospace)
            .color(palette(ui.ctx()).text_2),
    );
    widgets::address(ui, &shown, "editor-position");
    let previous = ui.add_enabled_ui(at > 0, |ui| {
        let label = tr(keys::editor::PREVIOUS_NOTE);
        IconButton::new(Icon::ChevronLeft, &label, "editor-previous").show(ui)
    });
    if previous.inner.clicked() {
        actions.push(editor(Editing::Step(-1)));
    }
}
