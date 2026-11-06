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

/// Each kind and its words, in the menu's order.
const KINDS: [(NoteKind, Key, Key); 5] = [
    (
        NoteKind::Snippet,
        keys::notes::NEW_SNIPPET,
        keys::notes::kind_lines::SNIPPET,
    ),
    (
        NoteKind::Note,
        keys::notes::NEW_RICH_NOTE,
        keys::notes::kind_lines::NOTE,
    ),
    (
        NoteKind::Checklist,
        keys::notes::NEW_CHECKLIST,
        keys::notes::kind_lines::CHECKLIST,
    ),
    (
        NoteKind::Calc,
        keys::notes::NEW_CALC,
        keys::notes::kind_lines::CALC,
    ),
    (
        NoteKind::Drawing,
        keys::notes::NEW_DRAWING,
        keys::notes::kind_lines::DRAWING,
    ),
];

const MENU_WIDTH: f32 = 340.0;
const PADDING: f32 = 10.0;
const TILE: f32 = 30.0;
const GAP: f32 = 12.0;
const NAME_HEIGHT: f32 = 17.0;

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
            ui.set_width(MENU_WIDTH);
            ui.spacing_mut().item_spacing.y = 2.0;
            let p = palette(ui.ctx());
            let title = tr(keys::notes::CREATE_MENU_TITLE).to_uppercase();
            let galley = widgets::text(ui, &title, 10.5, 600.0, FontFamily::Proportional, p.text_2);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(galley.size().x, 22.0), Sense::hover());
            ui.painter()
                .galley(pos2(rect.left() + 10.0, rect.top() + 6.0), galley, p.text_2);
            let fresh = (!state.settings.note_kind_tried).then(|| tr(keys::notes::KIND_NEW));
            for (kind, name, line) in KINDS {
                let badge = fresh.as_deref().filter(|_| kind == NoteKind::Note);
                if kind_item(ui, kind, (&tr(name), &tr(line)), badge).clicked() {
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
                Vec2::new(galley.size().x + 26.0, 32.0),
                CornerRadius {
                    nw: 8,
                    sw: 8,
                    ne: 0,
                    se: 0,
                },
            )
        }
        Part::Caret => (
            Vec2::new(28.0, 32.0),
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

/// A kind of note: its tile, its name, what it is for, the keys that make one.
fn kind_item(
    ui: &mut Ui,
    kind: NoteKind,
    (name, line): (&str, &str),
    badge: Option<&str>,
) -> Response {
    let p = palette(ui.ctx());
    let author_id = format!("new-note-{}", kind_id(kind));
    let corner = CornerRadius::same(8);
    let width = ui.available_width();
    let keys = shortcut(kind);
    let room = width - 2.0 * PADDING - TILE - 2.0 * GAP - key_caps_width(ui, keys);
    let text_size = egui::FontId::new(11.5, FontFamily::Proportional);
    let line = ui.fonts_mut(|fonts| fonts.layout(line.to_owned(), text_size, p.text_2, room));
    let height = 2.0 * PADDING + NAME_HEIGHT + 3.0 + line.size().y;
    let response = interact(
        ui,
        Vec2::new(width, height),
        Sense::click(),
        &author_id,
        || WidgetInfo::labeled(Role::MenuItem, true, name),
        corner,
    );
    let rect = response.rect;
    if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_3);
    }
    tile(ui, rect.min + Vec2::splat(PADDING), kind);
    let left = rect.left() + PADDING + TILE + GAP;
    let top = rect.top() + PADDING;
    let name = widgets::text(ui, name, 13.0, 600.0, FontFamily::Proportional, p.text_1);
    let after_name = left + name.size().x + 8.0;
    ui.painter().galley(pos2(left, top), name, p.text_1);
    if let Some(words) = badge {
        new_badge(ui, pos2(after_name, top + NAME_HEIGHT / 2.0), words);
    }
    ui.painter()
        .galley(pos2(left, top + NAME_HEIGHT + 3.0), line, p.text_2);
    key_caps(ui, keys, rect.right() - PADDING, rect.center().y);
    response
}

/// The kind's mark on a neutral tile, ringed in its hue: the hue is how the kinds are told apart.
fn tile(ui: &Ui, at: egui::Pos2, kind: NoteKind) {
    let p = palette(ui.ctx());
    let hue = match kind {
        NoteKind::Snippet => p.blue,
        NoteKind::Note => p.purple,
        NoteKind::Checklist => p.green,
        NoteKind::Calc => p.pink,
        NoteKind::Drawing => p.teal,
    };
    let rect = Rect::from_min_size(at, Vec2::splat(TILE));
    ui.painter().rect(
        rect,
        CornerRadius::same(7),
        p.bg_3,
        Stroke::new(1.0, hue.gamma_multiply(0.4)),
        egui::StrokeKind::Inside,
    );
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    let mark = match kind {
        NoteKind::Snippet => "</>",
        NoteKind::Note => "Aa",
        NoteKind::Checklist => return icons::paint(ui, glyph, Icon::Check, hue),
        NoteKind::Calc => return icons::paint(ui, glyph, Icon::Calculator, hue),
        NoteKind::Drawing => return icons::paint(ui, glyph, Icon::Image, hue),
    };
    let words = widgets::text(ui, mark, 12.0, 700.0, FontFamily::Monospace, hue);
    ui.painter()
        .galley(rect.center() - words.size() / 2.0, words, hue);
}

/// « New », until a note of that kind was made.
fn new_badge(ui: &Ui, left_middle: egui::Pos2, words: &str) {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, words, 10.0, 700.0, FontFamily::Proportional, p.text_1);
    let size = galley.size() + Vec2::new(12.0, 2.0);
    let rect = Rect::from_min_size(left_middle - Vec2::new(0.0, size.y / 2.0), size);
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(9),
        p.amber_fill.gamma_multiply(0.15),
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, p.text_1);
}

fn key_cap_text(ui: &Ui, word: &str) -> std::sync::Arc<egui::Galley> {
    let colour = palette(ui.ctx()).text_1;
    widgets::text(ui, word, 10.5, 400.0, FontFamily::Monospace, colour)
}

fn key_caps_width(ui: &Ui, keys: &str) -> f32 {
    keys.split('+')
        .map(|word| key_cap_text(ui, word).size().x + 12.0 + 3.0)
        .sum()
}

/// One cap a key, the last ending at `right`.
fn key_caps(ui: &Ui, keys: &str, right: f32, middle: f32) {
    let p = palette(ui.ctx());
    let mut right = right;
    for word in keys.rsplit('+') {
        let galley = key_cap_text(ui, word);
        let size = galley.size() + Vec2::new(12.0, 4.0);
        let rect = Rect::from_min_size(pos2(right - size.x, middle - size.y / 2.0), size);
        ui.painter().rect(
            rect,
            CornerRadius::same(4),
            p.bg_3,
            Stroke::new(1.0, p.line),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, p.text_1);
        right = rect.left() - 3.0;
    }
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
