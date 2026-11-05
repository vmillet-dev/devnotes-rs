//! The top of the notes area: where one is — the rail, the trash, the space and folder, the
//! list or the board — then how one looks at the notes — the search, the quick filter, the
//! order, the facets — and the new note (`notes-header` in the reference). One row while it
//! holds, two past that (`layout`). Each change is an action; the query state asks the core
//! again.

mod facets;
mod layout;
mod new_note;
pub(crate) use new_note::shortcut;
mod sort;
mod switchers;

use egui::{
    Color32, CornerRadius, FontFamily, Margin, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
};
use engine::notes::view::NoteFilter;
use i18n::keys;

use crate::action::{Action, Actions};
use crate::state::AppState;
use crate::state::board::Mode;
use crate::theme::icons::Icon;
use crate::theme::{palette, spacing};
use crate::widgets::{self, SearchField, Segmented, interact};
use layout::GAP;

/// What only the header's drawing sees: what its menus show, what it measured, the facets.
#[derive(Default)]
pub(super) struct HeaderView {
    space_menu: switchers::Menu,
    folder_menu: switchers::Menu,
    crumb: Option<crate::notes::panel::FolderEdit>,
    fixed: layout::Fixed,
    /// Asked open; a chip pressed holds it open whatever this says.
    facets: bool,
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (view, focus_search): (&mut HeaderView, &mut bool),
    actions: &mut Actions,
) {
    let s = spacing(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let (grid, canvas) = (s.grid.round() as i8, s.canvas.round() as i8);
    let language = state.language;
    let switchers = if switchers_shown(state) {
        switchers::natural_widths(ui, state).to_vec()
    } else {
        Vec::new()
    };
    let new_note = new_note::width(ui, language);
    let width = ui.available_width() - 2.0 * f32::from(canvas);
    let plan = layout::plan(width, view.fixed, &switchers, new_note);
    let pressed = state.notes_query.facets().count();
    let open = view.facets || pressed > 0;
    let mut fixed = layout::Fixed::default();
    let shown = egui::Frame::new()
        .inner_margin(Margin::symmetric(canvas, grid))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(GAP, 8.0);
            let facets = (open, pressed);
            if plan.two_rows {
                ui.horizontal(|ui| {
                    fixed.place = group(ui, |ui| {
                        where_one_is(ui, state, view, plan.switcher, actions)
                    });
                    new_note_at_end(ui, state, new_note, actions);
                });
                ui.horizontal(|ui| {
                    fixed.look = group(ui, |ui| {
                        how_one_looks(
                            ui,
                            state,
                            (plan.search, facets),
                            view,
                            focus_search,
                            actions,
                        )
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    fixed.place = group(ui, |ui| {
                        where_one_is(ui, state, view, plan.switcher, actions)
                    });
                    fixed.look = group(ui, |ui| {
                        how_one_looks(
                            ui,
                            state,
                            (plan.search, facets),
                            view,
                            focus_search,
                            actions,
                        )
                    });
                    new_note_at_end(ui, state, new_note, actions);
                });
            }
        });
    // What does not narrow is the same in one row and two: once measured, the plan holds.
    if fixed != view.fixed {
        view.fixed = fixed;
        ui.ctx().request_repaint();
    }
    let rect = shown.response.rect;
    let p = palette(ui.ctx());
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
    if open {
        ui.add_space(1.0);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            facets::show(ui, state, actions);
        });
    }
}

/// The switchers stand in for the rail while it is put away, and give way to a breadcrumb.
fn switchers_shown(state: &AppState) -> bool {
    !state.settings.rail_shown && state.active_folder().is_none()
}

/// A group of controls side by side; answers the width of what in it does not narrow.
fn group(ui: &mut Ui, draw: impl FnOnce(&mut Ui) -> f32) -> f32 {
    let shown = ui.horizontal(draw);
    shown.response.rect.width() - shown.inner
}

fn new_note_at_end(ui: &mut Ui, state: &AppState, width: f32, actions: &mut Actions) {
    ui.add_space((ui.available_width() - width).max(0.0));
    ui.scope(|ui| new_note::show(ui, state, actions));
}

/// The search, the quick filter, the order and the facets' toggle; answers the search's width.
fn how_one_looks(
    ui: &mut Ui,
    state: &AppState,
    (width, (open, pressed)): (f32, (bool, usize)),
    view: &mut HeaderView,
    focus_search: &mut bool,
    actions: &mut Actions,
) -> f32 {
    let language = state.language;
    let filters = [
        (NoteFilter::All, keys::filters::ALL),
        (NoteFilter::Pinned, keys::filters::PINNED),
        (NoteFilter::Untriaged, keys::filters::UNTRIAGED),
    ]
    .map(|(filter, key)| (filter, i18n::translate(language, key, &[])));
    let filters: Vec<(NoteFilter, &str)> = filters
        .iter()
        .map(|(filter, label)| (*filter, label.as_str()))
        .collect();
    search(ui, state, width, focus_search, actions);
    let mut chosen = state.notes_query.filter();
    if Segmented::new(&filters, "quick-filter")
        .show(ui, &mut chosen)
        .changed()
    {
        push(actions, crate::notes::Action::SetFilter(chosen));
    }
    sort::show(ui, state, actions);
    if facets_toggle(ui, open, pressed, language).clicked() {
        view.facets = !open;
    }
    width
}

/// Amber while the rails show; it refuses to fold while a chip is pressed, and counts them.
fn facets_toggle(ui: &mut Ui, open: bool, pressed: usize, language: i18n::Language) -> Response {
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::filters::FACETS_LABEL, &[]);
    let enabled = !(open && pressed > 0);
    let count = (pressed > 0).then(|| {
        widgets::text(
            ui,
            &pressed.to_string(),
            10.5,
            600.0,
            FontFamily::Proportional,
            p.amber_text,
        )
    });
    let counted = count.as_ref().map_or(0.0, |galley| galley.size().x + 4.0);
    let corner = CornerRadius::same(7);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = interact(
        ui,
        Vec2::new(14.0 + 16.0 + counted, 32.0),
        sense,
        "facets-toggle",
        || {
            let mut info = WidgetInfo::labeled(Role::Button, enabled, &label);
            info.selected = Some(open);
            info
        },
        corner,
    );
    let hover = if enabled {
        widgets::hovered(ui, &response)
    } else {
        0.0
    };
    let lit = if open { 1.0 } else { hover };
    let rect = response.rect;
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, widgets::towards(p.line_soft, p.amber_dim, lit)),
        StrokeKind::Inside,
    );
    let ink = if open {
        p.amber_text
    } else {
        widgets::towards(p.text_1, p.text_0, hover)
    };
    let left = rect.center().x - f32::midpoint(16.0, counted);
    let glyph =
        egui::Rect::from_min_size(egui::pos2(left, rect.center().y - 8.0), Vec2::splat(16.0));
    crate::theme::icons::paint(ui, glyph, Icon::Filter, ink);
    if let Some(count) = count {
        let rect_count = egui::Rect::from_min_size(
            egui::pos2(glyph.right() + 4.0, rect.center().y - count.size().y / 2.0),
            count.size(),
        );
        // In a child: allocated in the row, it would bring the row's cursor back.
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(rect_count));
        let named = inside.allocate_rect(rect_count, Sense::hover());
        let words = pressed.to_string();
        named.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
        widgets::address(&inside, &named, "facets-count");
        ui.painter().galley(rect_count.min, count, p.amber_text);
    }
    response
}

/// The rail toggle, the trash, then the folder open or the switchers, and the view; answers
/// the switchers' widths.
fn where_one_is(
    ui: &mut Ui,
    state: &AppState,
    view: &mut HeaderView,
    widest: f32,
    actions: &mut Actions,
) -> f32 {
    let label = i18n::translate(state.language, keys::sidebar::TOGGLE, &[]);
    let shown = state.settings.rail_shown;
    if widgets::SquareButton::new(Icon::Sidebar, &label, "library-rail-toggle")
        .pressed(shown)
        .show(ui)
        .clicked()
    {
        actions.push(Action::App(crate::action::AppAction::ToggleRail));
    }
    // Away from the filters and the primary action: a view on the notes, not a filter.
    let trash = i18n::translate(state.language, keys::notes::TRASH, &[]);
    if widgets::SquareButton::new(Icon::Trash, &trash, "trash-open")
        .show(ui)
        .clicked()
    {
        actions.push(Action::Notes(crate::notes::Action::Trash(
            crate::notes::Trash::Open,
        )));
    }
    let open = state.active_folder().and_then(|id| state.folders.find(id));
    if let Some(folder) = open {
        switchers::breadcrumb(ui, state, folder, &mut view.crumb, actions);
        return 0.0;
    }
    let mut narrowed = 0.0;
    if !shown {
        narrowed += switchers::space_switcher(ui, state, &mut view.space_menu, widest, actions);
    }
    view_switch(ui, state, actions);
    if !shown {
        narrowed += switchers::folder_switcher(ui, state, &mut view.folder_menu, widest, actions);
    }
    narrowed
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
    // Zero results is the answer that matters most: said, never left quiet. The board's own
    // count while it shows: it dims what the list would drop.
    let matched = if state.board_place().is_some() {
        state.board.matched()
    } else {
        query.matched()
    };
    let matched = matched.map(|count| {
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
    // Down from the field to what it found: the first card.
    if widgets::takes_keys(ui, &field) && ui.input(|input| input.key_pressed(egui::Key::ArrowDown))
    {
        push(
            actions,
            crate::notes::Action::Canvas(crate::notes::canvas::shortcuts::CanvasKey::Move(
                crate::notes::canvas::grid::Direction::Down,
            )),
        );
    }
    if searched.cleared {
        push(actions, crate::notes::Action::ClearSearch);
    } else if typed != query.typed() {
        push(actions, crate::notes::Action::Search(typed));
    }
}

/// List or board. The board is disabled rather than hidden: a control that vanishes on « all
/// spaces » reads as a bug, a disabled one with its reason as an explanation.
fn view_switch(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
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
            let space = state.spaces.active_space();
            let on_board = state.board_place().is_some();
            let list = option(
                ui,
                &tr(keys::board::LIST_VIEW),
                "view-list",
                !on_board,
                true,
            );
            let board = option(
                ui,
                &tr(keys::board::BOARD_VIEW),
                "view-board",
                on_board,
                space.is_some(),
            );
            if space.is_none() {
                widgets::tooltip(board.clone(), &tr(keys::board::NEEDS_ASPACE));
            }
            let mode = if list.clicked() {
                Mode::List
            } else if board.clicked() {
                Mode::Board
            } else {
                return;
            };
            actions.push(Action::Notes(crate::notes::Action::SetView(mode)));
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
