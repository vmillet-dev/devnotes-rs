//! « + New note », split: the note of the preferred kind in one click, any kind in two.

use egui::{
    Color32, CornerRadius, FontFamily, Popup, PopupCloseBehavior, Rect, RectAlign, Response, Role,
    Sense, Stroke, Ui, Vec2, WidgetInfo, pos2,
};
use engine::notes::kind::NoteKind;
use i18n::{Key, keys};

use crate::action::{Action, Actions};
use crate::state::AppState;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, interact};

/// Each kind, its mark and its words, in the menu's order.
const KINDS: [(NoteKind, &str, Key, Key); 5] = [
    (
        NoteKind::Snippet,
        "</>",
        keys::notes::NEW_SNIPPET,
        keys::notes::kind_lines::SNIPPET,
    ),
    (
        NoteKind::Note,
        "Aa",
        keys::notes::NEW_RICH_NOTE,
        keys::notes::kind_lines::NOTE,
    ),
    (
        NoteKind::Checklist,
        "",
        keys::notes::NEW_CHECKLIST,
        keys::notes::kind_lines::CHECKLIST,
    ),
    (
        NoteKind::Calc,
        "",
        keys::notes::NEW_CALC,
        keys::notes::kind_lines::CALC,
    ),
    (
        NoteKind::Drawing,
        "",
        keys::notes::NEW_DRAWING,
        keys::notes::kind_lines::DRAWING,
    ),
];

/// How wide it is drawn, for the row that lays out what comes before it.
pub(super) fn width(ui: &Ui, language: i18n::Language) -> f32 {
    let label = i18n::translate(language, keys::notes::NEW_NOTE, &[]);
    widgets::text(
        ui,
        &label,
        13.0,
        700.0,
        FontFamily::Proportional,
        Color32::WHITE,
    )
    .size()
    .x + 26.0
        + 28.0
}

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.spacing_mut().item_spacing.x = 0.0;
    let label = tr(keys::notes::NEW_NOTE);
    let main = part(ui, &label, "new-note", Part::Main);
    if main.clicked() {
        new_note(actions, None);
    }
    let caret = part(
        ui,
        &tr(keys::notes::NEW_NOTE_KIND_LABEL),
        "new-note-kind",
        Part::Caret,
    );
    Popup::menu(&caret)
        .frame(widgets::menu_frame(ui.ctx()))
        .align(RectAlign::BOTTOM_END)
        .close_behavior(PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_width(360.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            let p = palette(ui.ctx());
            let title = tr(keys::notes::CREATE_MENU_TITLE).to_uppercase();
            let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(galley.size().x, 22.0), Sense::hover());
            ui.painter()
                .galley(pos2(rect.left() + 10.0, rect.top() + 6.0), galley, p.text_2);
            for (kind, mark, name, line) in KINDS {
                if kind_item(ui, kind, mark, &tr(name), &tr(line)).clicked() {
                    new_note(actions, Some(kind));
                }
            }
        });
}

fn new_note(actions: &mut Actions, kind: Option<NoteKind>) {
    actions.push(Action::Notes(crate::notes::Action::NewNote(kind)));
}

#[derive(Clone, Copy)]
enum Part {
    Main,
    Caret,
}

/// Amber, both halves: the one primary action of the area.
fn part(ui: &mut Ui, label: &str, author_id: &str, part: Part) -> Response {
    let p = palette(ui.ctx());
    let (size, corner) = match part {
        Part::Main => {
            let galley = widgets::text(
                ui,
                label,
                13.0,
                700.0,
                FontFamily::Proportional,
                p.amber_ink,
            );
            (
                Vec2::new(galley.size().x + 26.0, 34.0),
                CornerRadius {
                    nw: 8,
                    sw: 8,
                    ne: 0,
                    se: 0,
                },
            )
        }
        Part::Caret => (
            Vec2::new(28.0, 34.0),
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 8,
                se: 8,
            },
        ),
    };
    let response = interact(
        ui,
        size,
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, label),
        corner,
    );
    let fill = if response.hovered() {
        p.amber_fill.gamma_multiply(1.08)
    } else {
        p.amber_fill
    };
    let rect = response.rect;
    ui.painter().rect_filled(rect, corner, fill);
    match part {
        Part::Main => {
            let galley = widgets::text(
                ui,
                label,
                13.0,
                700.0,
                FontFamily::Proportional,
                p.amber_ink,
            );
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, p.amber_ink);
        }
        Part::Caret => {
            ui.painter().vline(
                rect.left(),
                rect.y_range(),
                Stroke::new(1.0, Color32::BLACK.gamma_multiply(0.22)),
            );
            let glyph = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
            icons::paint(ui, glyph, Icon::ArrowDown, p.amber_ink);
        }
    }
    response
}

/// A kind of note: its mark, its name, what it is for.
fn kind_item(ui: &mut Ui, kind: NoteKind, mark: &str, name: &str, line: &str) -> Response {
    let p = palette(ui.ctx());
    let author_id = format!("new-note-{}", kind_id(kind));
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), 44.0),
        Sense::click(),
        &author_id,
        || WidgetInfo::labeled(Role::MenuItem, true, name),
        corner,
    );
    let rect = response.rect;
    if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    let glyph =
        Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), Vec2::splat(16.0));
    match kind {
        NoteKind::Checklist => icons::paint(ui, glyph, Icon::Check, p.text_1),
        NoteKind::Calc => icons::paint(ui, glyph, Icon::Calculator, p.text_1),
        NoteKind::Drawing => icons::paint(ui, glyph, Icon::Image, p.text_1),
        NoteKind::Snippet | NoteKind::Note => {
            let galley = widgets::text(ui, mark, 11.5, 600.0, FontFamily::Monospace, p.text_1);
            ui.painter()
                .galley(glyph.center() - galley.size() / 2.0, galley, p.text_1);
        }
    }
    let left = rect.left() + 44.0;
    let name = widgets::text(ui, name, 12.5, 600.0, FontFamily::Proportional, p.text_0);
    ui.painter()
        .galley(pos2(left, rect.top() + 6.0), name, p.text_0);
    let mut job = egui::text::LayoutJob::single_section(
        line.to_owned(),
        egui::TextFormat::simple(egui::FontId::new(11.0, FontFamily::Proportional), p.text_2),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: rect.right() - 10.0 - left,
        max_rows: 1,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    let line = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let keys = widgets::text(
        ui,
        shortcut(kind),
        10.5,
        400.0,
        FontFamily::Monospace,
        p.text_2,
    );
    ui.painter().galley(
        pos2(rect.right() - 10.0 - keys.size().x, rect.top() + 8.0),
        keys,
        p.text_2,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 24.0), line, p.text_2);
    response
}

/// The canvas key each kind answers to (`canvas::shortcuts::CANVAS`).
pub(crate) fn shortcut(kind: NoteKind) -> &'static str {
    let mac = cfg!(target_os = "macos");
    match (kind, mac) {
        (NoteKind::Snippet, false) => "Ctrl+N",
        (NoteKind::Note, false) => "Ctrl+Shift+N",
        (NoteKind::Checklist, false) => "Ctrl+Shift+L",
        (NoteKind::Calc, false) => "Ctrl+Shift+M",
        (NoteKind::Drawing, false) => "Ctrl+Shift+D",
        (NoteKind::Snippet, true) => "⌘N",
        (NoteKind::Note, true) => "⌘⇧N",
        (NoteKind::Checklist, true) => "⌘⇧L",
        (NoteKind::Calc, true) => "⌘⇧M",
        (NoteKind::Drawing, true) => "⌘⇧D",
    }
}

fn kind_id(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Snippet => "snippet",
        NoteKind::Note => "note",
        NoteKind::Checklist => "checklist",
        NoteKind::Calc => "calc",
        NoteKind::Drawing => "drawing",
    }
}
