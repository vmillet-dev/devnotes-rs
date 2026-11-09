//! The header under the toolbar: the context, the tags and the deadline on one band, what the note
//! is and where it lives on the next — or, folded, one line saying what they hold.

use chrono::{Local, NaiveTime, TimeZone, Utc};
use egui::{Align, FontFamily, Layout, Margin, Ui, Vec2};
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, NoteLifecycle};
use i18n::keys;

use super::{EditorView, band, editor, save};
use crate::action::Actions;
use crate::notes::Editing;
use crate::state::AppState;
use crate::state::editor::Edit;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, ChoiceMenu, IconButton, Naming, date};

/// The lifecycle's badge and the date, side by side.
const DEADLINE_WIDTH: f32 = 240.0;
/// Under it the deadline never shares the line of the tags.
const OWN_ROW_UNDER: f32 = 640.0;

pub(super) fn bands(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    band(ui, Margin::symmetric(20, 8), true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            context(ui, state, (session, note), editing, actions);
            tags(ui, state, (session, note), editing, actions);
            // In a panel the tags wrap under the field: the deadline takes a line of its own.
            if ui.available_width() < DEADLINE_WIDTH || ui.max_rect().width() < OWN_ROW_UNDER {
                ui.end_row();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                deadline(ui, state, (session, note), actions);
                lifecycle_badge(ui, state, note);
            });
        });
    });
    band(ui, Margin::symmetric(20, 12), true, |ui| {
        placement(ui, state, (session, note), editing, actions);
    });
    band(ui, Margin::symmetric(20, 8), true, |ui| {
        super::attachments::show(ui, state, &mut editing.removing, actions);
    });
}

/// The free-form breadcrumb a pinned card shows instead of its age.
fn context(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let name = i18n::translate(language, keys::editor::SOURCE_LABEL, &[]);
    let hint = i18n::translate(language, keys::editor::SOURCE_PLACEHOLDER, &[]);
    let response = ui.add(
        egui::TextEdit::singleline(&mut editing.source)
            .hint_text(hint)
            .text_color(p.text_1)
            .font(egui::FontId::new(11.5, FontFamily::Proportional))
            .desired_width(160.0),
    );
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.clone()));
    widgets::address(ui, &response, "editor-source");
    if response.lost_focus() {
        save(session, note, editing, actions);
    }
    ui.label(widgets::text(
        ui,
        "·",
        11.5,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    ));
}

/// Each tag with the cross that takes it off, then the field that adds one: on Enter, on leaving
/// it, on folding and on closing — a typed tag is never dropped.
fn tags(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &mut EditorView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    for tag in &note.tags {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let label = format!("#{tag}");
            widgets::Chip::new(&label, "editor-tag", true).show(ui);
            let remove = i18n::translate(
                language,
                keys::editor::REMOVE_TAG,
                &[("tag", tag.clone().into())],
            );
            if IconButton::new(Icon::X, &remove, "editor-tag-remove")
                .show(ui)
                .clicked()
            {
                let kept = note
                    .tags
                    .iter()
                    .filter(|kept| *kept != tag)
                    .cloned()
                    .collect();
                actions.push(editor(Editing::Save {
                    session,
                    edit: Edit {
                        tags: Some(kept),
                        ..Edit::default()
                    },
                }));
            }
        });
    }
    let name = i18n::translate(language, keys::editor::ADD_TAG_LABEL, &[]);
    let hint = i18n::translate(language, keys::editor::ADD_TAG_PLACEHOLDER, &[]);
    let response = ui.add(
        egui::TextEdit::singleline(&mut editing.tag)
            .hint_text(hint)
            .text_color(p.text_1)
            .font(egui::FontId::new(11.5, FontFamily::Proportional))
            .desired_width(90.0),
    );
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.clone()));
    widgets::address(ui, &response, "editor-tag-add");
    let entered = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if response.lost_focus() && !editing.tag.trim().is_empty() {
        save(session, note, editing, actions);
        editing.tag.clear();
        if entered {
            response.request_focus();
        }
    }
}

/// « Never expires », or how long is left — red once it is close, as the core decided.
fn lifecycle_badge(ui: &mut Ui, state: &AppState, note: &DisplayNote) {
    let p = palette(ui.ctx());
    let (icon, words) = match note.lifecycle {
        NoteLifecycle::Permanent => (
            Icon::Clock,
            i18n::translate(state.language, keys::lifecycle::NEVER_EXPIRES, &[]),
        ),
        NoteLifecycle::Expires { at } => (
            Icon::Hourglass,
            i18n::time::expiry(at, state.clock.now(), &Local).text(state.language),
        ),
    };
    let hue = if note.expiring_soon {
        p.red
    } else {
        p.amber_text
    };
    // The reference's `tint-badge`: the hue at a tenth under, a quarter on the edge.
    let galley = widgets::text(ui, &words, 11.0, 400.0, FontFamily::Proportional, hue);
    let size = galley.size() + Vec2::new(20.0 + 16.0, 6.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &words));
    let corner = egui::CornerRadius::same(12);
    ui.painter().rect(
        rect,
        corner,
        hue.gamma_multiply(0.1),
        egui::Stroke::new(1.0, hue.gamma_multiply(0.3)),
        egui::StrokeKind::Inside,
    );
    let glyph = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 10.0 + 5.0, rect.center().y),
        Vec2::splat(11.0),
    );
    crate::theme::icons::paint(ui, glyph, icon, hue);
    let at = egui::pos2(
        rect.left() + 10.0 + 16.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(at, galley, hue);
    widgets::address(ui, &response, "editor-lifecycle");
}

/// Emptied, the note is permanent again; a day, and it is due at the end of it, local time.
fn deadline(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    actions: &mut Actions,
) {
    let day = match note.lifecycle {
        NoteLifecycle::Expires { at } => Some(at.with_timezone(&Local).date_naive()),
        NoteLifecycle::Permanent => None,
    };
    let today = state.clock.now().with_timezone(&Local).date_naive();
    let name = i18n::translate(state.language, keys::editor::EXPIRY_LABEL, &[]);
    let picked = date::DateField::new(day, today, &name, "editor-deadline")
        .language(state.language)
        .show(ui);
    let lifecycle = match picked {
        Some(date::Picked::Day(day)) => {
            end_of_local_day(day).map(|at| NoteLifecycle::Expires { at })
        }
        Some(date::Picked::Cleared) => Some(NoteLifecycle::Permanent),
        None => None,
    };
    if let Some(lifecycle) = lifecycle {
        actions.push(editor(Editing::Save {
            session,
            edit: Edit {
                lifecycle: Some(lifecycle),
                ..Edit::default()
            },
        }));
    }
}

/// The reference's `endOfLocalDay`: midnight would leave a note due today expired at once.
fn end_of_local_day(day: chrono::NaiveDate) -> Option<chrono::DateTime<Utc>> {
    let end = NaiveTime::from_hms_milli_opt(23, 59, 59, 999)?;
    Local
        .from_local_datetime(&day.and_time(end))
        .latest()
        .map(|at| at.with_timezone(&Utc))
}

/// What the note is — a menu converting it into another kind, a drawing named only — then its
/// space and its folder, together: moving the space takes the note out of its folder.
fn placement(
    ui: &mut Ui,
    state: &AppState,
    (session, note): (u64, &DisplayNote),
    editing: &EditorView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        let label = |ui: &mut Ui, words: &str| {
            ui.label(widgets::text(
                ui,
                words,
                11.0,
                400.0,
                FontFamily::Proportional,
                p.text_2,
            ));
        };
        label(ui, &tr(keys::editor::kind::LABEL));
        let current = tr(kind_name(note.kind));
        if note.kind == NoteKind::Drawing {
            let response = ui.label(widgets::text(
                ui,
                &current,
                12.0,
                400.0,
                FontFamily::Proportional,
                p.text_1,
            ));
            widgets::address(ui, &response, "editor-kind");
        } else {
            let names: Vec<(NoteKind, String)> = NoteKind::ALL
                .into_iter()
                .filter(|kind| *kind != note.kind)
                .map(|kind| (kind, tr(convert_name(kind))))
                .collect();
            let options: Vec<(NoteKind, &str)> = names
                .iter()
                .map(|(kind, name)| (*kind, name.as_str()))
                .collect();
            let mut chosen = note.kind;
            if ChoiceMenu::new(&current, "editor-kind", &options)
                .naming(Naming::Label)
                .show(ui, &mut chosen)
                .changed()
                && chosen != note.kind
            {
                actions.push(editor(Editing::Convert {
                    session,
                    into: chosen,
                    edit: editing.edit(note),
                }));
            }
        }
        ui.add_space(14.0);
        label(ui, &tr(keys::editor::PLACEMENT_LABEL));
        space_menu(ui, state, note, actions);
        label(ui, "›");
        folder_menu(ui, state, note, actions);
    });
}

fn space_menu(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    let spaces = state.spaces.spaces();
    let options: Vec<(usize, &str)> = spaces
        .iter()
        .enumerate()
        .map(|(at, space)| (at, space.name.as_str()))
        .collect();
    let Some(mut chosen) = spaces.iter().position(|space| space.id == note.space_id) else {
        return;
    };
    let label = i18n::translate(state.language, keys::notes::MOVE_TO_SPACE, &[]);
    if ChoiceMenu::new(&label, "editor-space", &options)
        .show(ui, &mut chosen)
        .changed()
        && let Some(space) = spaces.get(chosen)
        && space.id != note.space_id
    {
        actions.push(editor(Editing::MoveTo(space.id.clone())));
    }
}

/// The note's own space's folders, never the one shown: a note opened from « all spaces »
/// belongs to a space of its own. The last entry takes it out of every folder.
fn folder_menu(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    let folders: Vec<_> = state
        .folders
        .list()
        .iter()
        .filter(|folder| folder.space_id == note.space_id)
        .collect();
    let unfiled = i18n::translate(state.language, keys::notes::NO_FOLDER, &[]);
    let mut options: Vec<(usize, &str)> = folders
        .iter()
        .enumerate()
        .map(|(at, folder)| (at, folder.name.as_str()))
        .collect();
    options.push((folders.len(), unfiled.as_str()));
    let mut chosen = note
        .folder_id
        .as_ref()
        .and_then(|id| folders.iter().position(|folder| folder.id == *id))
        .unwrap_or(folders.len());
    let before = chosen;
    let label = i18n::translate(state.language, keys::notes::FILE_INTO, &[]);
    if ChoiceMenu::new(&label, "editor-folder", &options)
        .show(ui, &mut chosen)
        .changed()
        && chosen != before
    {
        let folder = folders.get(chosen).map(|folder| folder.id.clone());
        actions.push(editor(Editing::File(folder)));
    }
}

/// Folded: the bands' content minus what is empty, the deadline kept in sight, and the way back.
pub(super) fn summary(ui: &mut Ui, state: &AppState, note: &DisplayNote, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let mut parts = Vec::new();
    if !note.source.trim().is_empty() {
        parts.push(note.source.clone());
    }
    let space = state
        .spaces
        .spaces()
        .iter()
        .find(|space| space.id == note.space_id)
        .map_or_else(String::new, |space| space.name.clone());
    let folder = note
        .folder
        .as_ref()
        .map_or_else(|| tr(keys::notes::NO_FOLDER), |folder| folder.name.clone());
    parts.push(format!("{space} › {folder}"));
    if !note.tags.is_empty() {
        parts.push(count(language, keys::editor::header::TAGS, note.tags.len()));
    }
    if note.attachment_count > 0 {
        let attached = usize::try_from(note.attachment_count).unwrap_or(usize::MAX);
        parts.push(count(language, keys::editor::header::ATTACHMENTS, attached));
    }
    let words = format!(
        "{}   {}",
        tr(keys::editor::header::TITLE),
        parts.join(" · ")
    );
    ui.horizontal(|ui| {
        let (caret, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
        crate::theme::icons::paint(ui, caret, Icon::ChevronRight, p.text_2);
        let unfold = tr(keys::editor::header::UNFOLD);
        let response = ui
            .add(
                egui::Button::new(widgets::text(
                    ui,
                    &words,
                    11.5,
                    400.0,
                    FontFamily::Proportional,
                    p.text_2,
                ))
                .frame(false),
            )
            .on_hover_text(&unfold);
        widgets::address(ui, &response, "editor-header-summary");
        if response.clicked() {
            actions.push(editor(Editing::FoldHeader(false)));
        }
        if matches!(note.lifecycle, NoteLifecycle::Expires { .. }) {
            lifecycle_badge(ui, state, note);
        }
    });
}

fn count(language: i18n::Language, key: i18n::Key, count: usize) -> String {
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    i18n::translate(language, key, &[("count", count.into())])
}

fn kind_name(kind: NoteKind) -> i18n::Key {
    match kind {
        NoteKind::Snippet => keys::editor::kind::SNIPPET,
        NoteKind::Note => keys::editor::kind::NOTE,
        NoteKind::Checklist => keys::editor::kind::CHECKLIST,
        NoteKind::Calc => keys::editor::kind::CALC,
        NoteKind::Drawing => keys::editor::kind::DRAWING,
    }
}

fn convert_name(kind: NoteKind) -> i18n::Key {
    match kind {
        NoteKind::Snippet => keys::editor::kind::convert_to::SNIPPET,
        NoteKind::Note => keys::editor::kind::convert_to::NOTE,
        NoteKind::Checklist => keys::editor::kind::convert_to::CHECKLIST,
        NoteKind::Calc => keys::editor::kind::convert_to::CALC,
        NoteKind::Drawing => keys::editor::kind::convert_to::DRAWING,
    }
}
