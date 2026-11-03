//! The top of the notes area: how one looks at the notes — the list or the board, the search,
//! the quick filter, the order (`notes-header` in the reference). Each change is an action; the
//! query state asks the core again.

mod sort;

use egui::{
    Color32, CornerRadius, FontFamily, Margin, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
};
use engine::notes::view::NoteFilter;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, SearchField, Segmented, interact};

const GAP: f32 = 10.0;
/// The search's widths: its floor, and where it stops growing.
const SEARCH: (f32, f32) = (140.0, 520.0);

pub(super) fn show(ui: &mut Ui, state: &AppState, focus_search: &mut bool, actions: &mut Actions) {
    let s = spacing(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let (grid, canvas) = (s.grid.round() as i8, s.canvas.round() as i8);
    let shown = egui::Frame::new()
        .inner_margin(Margin::symmetric(canvas, grid))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                view_switch(ui, state);
                let language = state.language;
                let filters = [
                    (
                        NoteFilter::All,
                        i18n::translate(language, keys::filters::ALL, &[]),
                    ),
                    (
                        NoteFilter::Pinned,
                        i18n::translate(language, keys::filters::PINNED, &[]),
                    ),
                    (
                        NoteFilter::Untriaged,
                        i18n::translate(language, keys::filters::UNTRIAGED, &[]),
                    ),
                ];
                let filters: Vec<(NoteFilter, &str)> = filters
                    .iter()
                    .map(|(filter, label)| (*filter, label.as_str()))
                    .collect();
                let quick = Segmented::new(&filters, "quick-filter");
                let after = quick.width(ui) + GAP + 32.0 + GAP;
                let width = (ui.available_width() - after).clamp(SEARCH.0, SEARCH.1);
                search(ui, state, width, focus_search, actions);
                let mut chosen = state.notes_query.filter();
                if quick.show(ui, &mut chosen).changed() {
                    push(actions, crate::notes::Action::SetFilter(chosen));
                }
                sort::show(ui, state, actions);
            });
        });
    let rect = shown.response.rect;
    let p = palette(ui.ctx());
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
}

fn push(actions: &mut Actions, action: crate::notes::Action) {
    actions.push(Action::Notes(action));
}

fn search(
    ui: &mut Ui,
    state: &AppState,
    width: f32,
    focus_search: &mut bool,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let query = &state.notes_query;
    let mut typed = query.typed().to_owned();
    // Zero results is the answer that matters most: said, never left quiet.
    let matched = query.matched().map(|count| {
        i18n::translate(
            language,
            keys::notes::SEARCH_MATCHED,
            &[("count", count.into())],
        )
    });
    let (label, hint, clear) = (
        tr(keys::notes::SEARCH_LABEL),
        tr(keys::notes::SEARCH_PLACEHOLDER),
        tr(keys::notes::CLEAR_SEARCH),
    );
    let key = if cfg!(target_os = "macos") {
        "⌘K"
    } else {
        "Ctrl+K"
    };
    let searched = SearchField::new(&mut typed, &label, "search-input")
        .hint(&hint)
        .key(key)
        .matched(matched.as_deref())
        .clear(&clear, "search-clear")
        .width(width)
        .show(ui);
    let field = widgets::tooltip(searched.field, &tr(keys::notes::SEARCH_HINT));
    if std::mem::take(focus_search) {
        field.request_focus();
    }
    if searched.cleared {
        push(actions, crate::notes::Action::ClearSearch);
    } else if typed != query.typed() {
        push(actions, crate::notes::Action::Search(typed));
    }
}

/// List or board. The board is disabled rather than hidden: a control that vanishes on « all
/// spaces » reads as a bug, a disabled one with its reason as an explanation.
fn view_switch(ui: &mut Ui, state: &AppState) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(3));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            option(ui, &tr(keys::board::LIST_VIEW), "view-list", true, true);
            // The board arrives with its own phase; until then it is never on.
            let board = option(ui, &tr(keys::board::BOARD_VIEW), "view-board", false, false);
            if state.spaces.active_space().is_none() {
                widgets::tooltip(board, &tr(keys::board::NEEDS_ASPACE));
            }
        });
    });
}

fn option(ui: &mut Ui, label: &str, author_id: &str, on: bool, enabled: bool) -> Response {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, label, 12.0, 400.0, FontFamily::Proportional, p.text_2);
    let corner = CornerRadius::same(6);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = interact(
        ui,
        galley.size() + Vec2::new(24.0, 10.0),
        sense,
        author_id,
        || {
            let mut info = WidgetInfo::labeled(Role::Button, enabled, label);
            info.selected = Some(on);
            info
        },
        corner,
    );
    let (fill, ink) = if on {
        (p.amber_fill, p.amber_ink)
    } else if enabled && response.hovered() {
        (Color32::TRANSPARENT, p.text_0)
    } else {
        (Color32::TRANSPARENT, p.text_2)
    };
    let ink = if enabled {
        ink
    } else {
        ink.gamma_multiply(0.45)
    };
    ui.painter().rect(
        response.rect,
        corner,
        fill,
        Stroke::NONE,
        StrokeKind::Inside,
    );
    let galley = widgets::text(ui, label, 12.0, 400.0, FontFamily::Proportional, ink);
    ui.painter()
        .galley(response.rect.center() - galley.size() / 2.0, galley, ink);
    response
}

#[cfg(test)]
mod tests;
