//! The catalogue in its panels, the starred and the recent tools above it, and a search over the
//! tools — their names, lines, keywords and category — and inside the references.

mod hits;

use egui::{
    Color32, CornerRadius, FontFamily, Id, Key, Modifiers, Pos2, Rect, Response, Role, Sense,
    Stroke, Ui, UiBuilder, Vec2, WidgetInfo,
};
use i18n::{Language, Message, keys};

use super::catalogue::{Category, TOOLS, ToolEntry};
use super::{Action, Asked, Opening, bar, star, star_hint};
use crate::action::Actions;
use crate::api;
use crate::services::time;
use crate::state::AppState;
use crate::state::tools::{Filter, SHOWN_RECENTS};
use crate::theme::icons;
use crate::theme::{palette, spacing};
use crate::widgets::{self, SearchField, address, hovered, interact, towards};

const PANEL_MIN_WIDTH: f32 = 260.0;
const PANEL_GAP: f32 = 14.0;
const SECTION_GAP: f32 = 22.0;

/// What only the home's drawing sees: what is typed, and the results as last drawn.
#[derive(Default)]
pub(super) struct HomeView {
    search: String,
    focus_search: bool,
    /// The field held the focus when last drawn.
    typing: bool,
    /// In the order they are read.
    results: Vec<Id>,
    references: hits::References,
    /// Where the next frame opens the reference a hit asked for.
    opening: Option<(&'static ToolEntry, Opening)>,
}

impl HomeView {
    pub(super) fn want_search(&mut self) {
        self.focus_search = true;
    }

    fn searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    pub(super) fn take_opening(&mut self) -> Option<(&'static ToolEntry, Opening)> {
        self.opening.take()
    }
}

struct Panel {
    category: Category,
    tools: Vec<&'static ToolEntry>,
}

pub(super) fn show(
    ui: &mut Ui,
    rest: Rect,
    state: &AppState,
    view: &mut HomeView,
    actions: &mut Actions,
) {
    let margin = spacing(ui.ctx()).canvas;
    let (mut row, body) = bar(ui, rest, margin, state, actions);
    empty_on_escape(ui, view);
    let field = head(&mut row, state, view, actions);
    view.typing = widgets::takes_keys(ui, &field);
    let found = panels(state, view);
    let first = view
        .searching()
        .then(|| found.first()?.tools.first().copied())
        .flatten();
    let mut content = ui.new_child(UiBuilder::new().max_rect(body));
    view.results.clear();
    view.references.search(view.search.trim(), state.language);
    egui::ScrollArea::vertical()
        .id_salt("tools-home")
        .auto_shrink(false)
        .show(&mut content, |ui| {
            let margin = spacing(ui.ctx()).canvas;
            let room = ui.available_width() - 2.0 * margin;
            ui.add_space(SECTION_GAP);
            ui.horizontal_top(|ui| {
                ui.add_space(margin);
                ui.allocate_ui(Vec2::new(room, 0.0), |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(room);
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        sections(ui, state, view, (&found, first), actions);
                    });
                });
            });
            ui.add_space(SECTION_GAP);
        });
    keys_of_the_search(ui, view, &field, first, actions);
}

/// The panels a search or a filter leaves, in the home's order.
fn panels(state: &AppState, view: &HomeView) -> Vec<Panel> {
    let query = api::tools::fold(view.search.trim());
    let only = if view.searching() {
        None
    } else {
        state.tools.filter()
    };
    let language = state.language;
    let shown = |tool: &ToolEntry| {
        (only != Some(Filter::Favourites) || state.tools.is_favourite(tool.id))
            && matches(tool, &query, language)
    };
    Category::ALL
        .into_iter()
        .filter(|category| !matches!(only, Some(Filter::Category(wanted)) if wanted != *category))
        .map(|category| Panel {
            category,
            tools: TOOLS
                .iter()
                .filter(|tool| tool.category == category && shown(tool))
                .collect(),
        })
        .filter(|panel| !panel.tools.is_empty())
        .collect()
}

/// « generateur » finds the tools filed under « Générateurs ».
fn matches(tool: &ToolEntry, query: &str, language: Language) -> bool {
    if query.is_empty() {
        return true;
    }
    let found = |text: &str| api::tools::fold(text).contains(query);
    found(&tool.name(language))
        || found(&tool.line(language))
        || found(&i18n::translate(language, tool.category.name(), &[]))
        || tool.keywords.iter().any(|keyword| found(keyword))
}

fn head(ui: &mut Ui, state: &AppState, view: &mut HomeView, actions: &mut Actions) -> Response {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let title = tr(keys::tools::TITLE);
    let galley = widgets::text(ui, &title, 17.0, 700.0, FontFamily::Proportional, p.text_0);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title));
    ui.painter().galley(rect.min, galley, p.text_0);
    let width = 380.0_f32.min(ui.max_rect().width() * 0.4);
    let hidden = if state.settings.rail_shown || view.searching() {
        None
    } else {
        state.tools.filter()
    };
    let room = ui.available_width() - width - 16.0;
    match hidden {
        Some(filter) => filtered(ui, filter, language, actions),
        None => subtitle(ui, language, room),
    }
    ui.add_space((ui.available_width() - width).max(0.0));
    let field = ui
        .scope(|ui| {
            let (hint, keys_said) = (
                tr(keys::tools::SEARCH_PLACEHOLDER),
                tr(keys::tools::SEARCH_KEYS),
            );
            SearchField::new(
                &mut view.search,
                &tr(keys::tools::SEARCH_LABEL),
                "tools-search",
            )
            .hint(&hint)
            .key(&keys_said)
            .width(width)
            .show(ui)
            .field
        })
        .inner;
    if std::mem::take(&mut view.focus_search) {
        field.request_focus();
    }
    field
}

/// What gives way when the bar runs short: cut, never at the search's expense.
fn subtitle(ui: &mut Ui, language: Language, room: f32) {
    let p = palette(ui.ctx());
    let count = u32::try_from(TOOLS.len()).unwrap_or(u32::MAX);
    let words = Message::new(keys::tools::COUNT)
        .with("count", count)
        .text(language);
    let galley = widgets::one_line(ui, &words, (12.0, 400.0), p.text_2, room);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "tools-count");
    ui.painter().galley(rect.min, galley, p.text_2);
    widgets::tooltip(
        response,
        &i18n::translate(language, keys::tools::OFFLINE, &[]),
    );
}

/// The rail is what says which category is showing: without it, the filter would be invisible.
fn filtered(ui: &mut Ui, filter: Filter, language: Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let name = match filter {
        Filter::Category(category) => category.name(),
        Filter::Favourites => keys::tools::favourites::TITLE,
    };
    let words = format!("› {}", i18n::translate(language, name, &[]));
    let galley = widgets::text(ui, &words, 14.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "tools-category-shown");
    ui.painter().galley(rect.min, galley, p.text_0);
    let all = i18n::translate(language, keys::tools::ALL, &[]);
    let back = widgets::text(ui, &all, 12.5, 400.0, FontFamily::Proportional, p.text_2);
    let response = interact(
        ui,
        back.size() + Vec2::new(12.0, 0.0),
        Sense::click(),
        "tools-category-clear",
        || WidgetInfo::labeled(Role::Button, true, &all),
        CornerRadius::same(4),
    );
    let ink = towards(p.text_2, p.text_0, hovered(ui, &response));
    let at = response.rect.center() - back.size() / 2.0;
    let line = at.y + back.size().y;
    ui.painter()
        .hline(at.x..=at.x + back.size().x, line, (1.0, ink));
    ui.painter().galley(at, back, ink);
    if response.clicked() {
        actions.push(Asked::Tools(Action::Home(None)));
    }
}

fn sections(
    ui: &mut Ui,
    state: &AppState,
    view: &mut HomeView,
    (found, first): (&[Panel], Option<&'static ToolEntry>),
    actions: &mut Actions,
) {
    if !view.searching() {
        favourites(ui, state, actions);
        recents(ui, state, actions);
    }
    columns(ui, state, view, (found, first), actions);
    hits::show(ui, state.language, view, first.is_none(), actions);
    if view.searching() && found.is_empty() && view.references.is_empty() {
        no_match(ui, state.language, view.search.trim());
    }
}

fn section_title(ui: &mut Ui, words: &str) {
    super::ui::section_title(ui, words);
    ui.add_space(10.0);
}

fn favourites(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let tools = state.tools.favourites();
    if tools.is_empty() {
        return;
    }
    let language = state.language;
    section_title(
        ui,
        &i18n::translate(language, keys::tools::favourites::TITLE, &[]),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(8.0);
        for tool in tools {
            let id = format!("tools-favourite-{}", tool.id);
            if chip(ui, tool, language, None, &id).clicked() {
                actions.push(Asked::Tools(Action::Open(tool.id)));
            }
        }
    });
    ui.add_space(SECTION_GAP);
}

/// More are kept than shown, for the rail: here, the five opened last, a favourite aside.
fn recents(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let shown: Vec<_> = state
        .tools
        .recents()
        .filter(|(tool, _)| !state.tools.is_favourite(tool.id))
        .take(SHOWN_RECENTS)
        .collect();
    if shown.is_empty() {
        return;
    }
    let language = state.language;
    section_title(ui, &i18n::translate(language, keys::tools::RECENTS, &[]));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(8.0);
        for (tool, at) in shown {
            let (when, change) = time::relative(at, state.clock.as_ref());
            ui.ctx().request_repaint_after(change);
            let when = when.text(language);
            let id = format!("tools-recent-{}", tool.id);
            if chip(ui, tool, language, Some(&when), &id).clicked() {
                actions.push(Asked::Tools(Action::Open(tool.id)));
            }
        }
    });
    ui.add_space(SECTION_GAP);
}

fn chip(
    ui: &mut Ui,
    tool: &ToolEntry,
    language: Language,
    when: Option<&str>,
    author_id: &str,
) -> Response {
    let p = palette(ui.ctx());
    let name = tool.name(language);
    let galley = widgets::wrapped(ui, &name, (12.5, 400.0), p.text_0, f32::INFINITY);
    let when =
        when.map(|when| widgets::text(ui, when, 11.0, 400.0, FontFamily::Proportional, p.text_2));
    let room = when.as_ref().map_or(0.0, |when| when.size().x + 8.0);
    let corner = CornerRadius::same(8);
    let response = interact(
        ui,
        Vec2::new(
            10.0 + 8.0 + 8.0 + galley.size().x + room + 14.0,
            galley.size().y + 16.0,
        ),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::Button, true, &name),
        corner,
    );
    let rect = response.rect;
    let edge = towards(p.line_soft, p.amber_dim, hovered(ui, &response));
    ui.painter().rect(
        rect,
        corner,
        p.bg_1,
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let middle = rect.center().y;
    let dot = Rect::from_center_size(Pos2::new(rect.left() + 14.0, middle), Vec2::splat(8.0));
    ui.painter()
        .rect_filled(dot, CornerRadius::same(2), tool.category.hue(p));
    let left = rect.left() + 26.0;
    let width = galley.size().x;
    let at = Pos2::new(left, middle - galley.size().y / 2.0);
    galley.paint(ui, at, p.text_0);
    if let Some(when) = when {
        let at = Pos2::new(left + width + 8.0, middle - when.size().y / 2.0);
        ui.painter().galley(at, when, p.text_2);
    }
    response
}

/// As many columns of at least `min` as `width` holds, `gap` between each; never none.
fn column_count(width: f32, min: f32, gap: f32) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few columns"
    )]
    let count = ((width + gap) / (min + gap)).floor() as usize;
    count.max(1)
}

/// `heights` in reading order, cut into consecutive stacks, one per column, the tallest as short
/// as it can be: the first rank of each stack but the first.
fn stack_in_columns(heights: &[usize], columns: usize) -> Vec<std::ops::Range<usize>> {
    let count = columns.clamp(1, heights.len().max(1)).min(heights.len());
    let below = |from: usize, to: usize| heights[from..to].iter().sum::<usize>();
    // tallest[k][i]: the tallest column when the first `i` items fill `k` columns.
    let mut tallest = vec![vec![usize::MAX; heights.len() + 1]; count + 1];
    let mut cut = vec![vec![0; heights.len() + 1]; count + 1];
    tallest[0][0] = 0;
    for k in 1..=count {
        for i in 1..=heights.len() {
            for from in k - 1..i {
                let candidate = tallest[k - 1][from].max(below(from, i));
                if candidate < tallest[k][i] {
                    tallest[k][i] = candidate;
                    cut[k][i] = from;
                }
            }
        }
    }
    let mut stacks = Vec::with_capacity(count);
    let mut end = heights.len();
    for k in (1..=count).rev() {
        let from = cut[k][end];
        stacks.push(from..end);
        end = from;
    }
    stacks.reverse();
    stacks
}

fn columns(
    ui: &mut Ui,
    state: &AppState,
    view: &mut HomeView,
    (found, first): (&[Panel], Option<&'static ToolEntry>),
    actions: &mut Actions,
) {
    let width = ui.available_width();
    let count = column_count(width, PANEL_MIN_WIDTH, PANEL_GAP);
    // A panel's height counted in rows, its head one of them: near enough to share them out.
    let heights: Vec<usize> = found.iter().map(|panel| panel.tools.len() + 1).collect();
    let stacks = stack_in_columns(&heights, count);
    #[expect(clippy::cast_precision_loss, reason = "a few columns")]
    let column = (width - PANEL_GAP * (count - 1) as f32) / count as f32;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = PANEL_GAP;
        for stack in stacks {
            ui.allocate_ui(Vec2::new(column, 0.0), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(column);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, PANEL_GAP);
                    for panel in &found[stack] {
                        panel_of(ui, state, view, (panel, first), actions);
                    }
                });
            });
        }
    });
}

fn panel_of(
    ui: &mut Ui,
    state: &AppState,
    view: &mut HomeView,
    (panel, first): (&Panel, Option<&'static ToolEntry>),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            panel_head(ui, panel, state.language);
            let last = panel.tools.len() - 1;
            for (rank, tool) in panel.tools.iter().copied().enumerate() {
                let is_first = first.is_some_and(|it| std::ptr::eq(it, tool));
                let place = Place {
                    first: is_first,
                    last: rank == last,
                };
                tool_row(ui, state, view, (tool, place), actions);
            }
        });
}

fn panel_head(ui: &mut Ui, panel: &Panel, language: Language) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 53.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.bottom() - 0.5, (1.0, p.line_soft));
    let hue = panel.category.hue(p);
    let tile = Rect::from_min_size(rect.min + Vec2::new(14.0, 12.0), Vec2::splat(28.0));
    ui.painter()
        .rect_filled(tile, CornerRadius::same(7), hue.gamma_multiply(0.15));
    let glyph = Rect::from_center_size(tile.center(), Vec2::splat(15.0));
    icons::paint(ui, glyph, panel.category.icon(), hue);
    let name = i18n::translate(language, panel.category.name(), &[]);
    let galley = widgets::text(ui, &name, 13.0, 700.0, FontFamily::Proportional, p.text_0);
    let at = Pos2::new(tile.right() + 10.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, p.text_0);
    let count = panel.tools.len().to_string();
    let galley = widgets::text(ui, &count, 11.0, 400.0, FontFamily::Monospace, p.text_2);
    let at = Pos2::new(
        rect.right() - 14.0 - galley.size().x,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(at, galley, p.text_2);
}

#[derive(Clone, Copy)]
struct Place {
    /// What Enter opens from the field, said on the result itself.
    first: bool,
    last: bool,
}

/// Two layers, as a note's card: the row's opening one and, over its end, the star.
fn tool_row(
    ui: &mut Ui,
    state: &AppState,
    view: &mut HomeView,
    (tool, place): (&'static ToolEntry, Place),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let width = ui.available_width();
    let name = tool.name(language);
    let room = width - 14.0 - 44.0;
    let title = widgets::wrapped(ui, &name, (13.0, 600.0), p.text_0, room);
    let line = widgets::wrapped(ui, &tool.line(language), (11.5, 400.0), p.text_2, room);
    let height = 10.0 + title.size().y + 3.0 + line.size().y + 10.0;
    let corner = if place.last {
        CornerRadius {
            sw: 12,
            se: 12,
            ..CornerRadius::ZERO
        }
    } else {
        CornerRadius::ZERO
    };
    let response = interact(
        ui,
        Vec2::new(width, height),
        Sense::click(),
        &format!("tools-entry-{}", tool.id),
        || WidgetInfo::labeled(Role::Button, true, &name),
        corner,
    );
    view.results.push(response.id);
    let rect = response.rect;
    let fill = towards(Color32::TRANSPARENT, p.bg_2, hovered(ui, &response));
    ui.painter().rect_filled(rect, corner, fill);
    if !place.last {
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 0.5, (1.0, p.bg_2));
    }
    let left = rect.left() + 14.0;
    let title_size = title.size();
    title.paint(ui, Pos2::new(left, rect.top() + 10.0), p.text_0);
    let below = rect.top() + 10.0 + title_size.y + 3.0;
    line.paint(ui, Pos2::new(left, below), p.text_2);
    if place.first {
        enter_mark(
            ui,
            Pos2::new(left + title_size.x + 6.0, rect.top() + 9.0),
            language,
        );
    }
    if response.clicked() {
        actions.push(Asked::Tools(Action::Open(tool.id)));
    }
    tool_star(ui, state, tool, (&response, &name), actions);
}

/// The key cap that says Enter opens this one: its arrow is in none of the embedded faces.
fn enter_mark(ui: &mut Ui, at: Pos2, language: Language) {
    let p = palette(ui.ctx());
    let cap = Rect::from_min_size(at, Vec2::new(22.0, 16.0));
    let mut layer = ui.new_child(UiBuilder::new().max_rect(cap));
    let (_, response) = layer.allocate_exact_size(cap.size(), Sense::hover());
    let hint = i18n::translate(language, keys::tools::ENTER_OPENS, &[]);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &hint));
    address(&layer, &response, "tools-first-result");
    widgets::tooltip(response, &hint);
    layer.painter().rect(
        cap,
        CornerRadius::same(4),
        p.bg_3,
        Stroke::new(1.0, p.line),
        egui::StrokeKind::Inside,
    );
    let glyph = Rect::from_center_size(cap.center(), Vec2::splat(11.0));
    icons::paint(&layer, glyph, icons::Icon::CornerDownLeft, p.text_1);
}

/// Shown on the row's hover and on its own focus, and always once on.
fn tool_star(
    ui: &mut Ui,
    state: &AppState,
    tool: &'static ToolEntry,
    (row, name): (&Response, &str),
    actions: &mut Actions,
) {
    let on = state.tools.is_favourite(tool.id);
    let place = Rect::from_min_size(
        row.rect.right_top() + Vec2::new(-36.0, 8.0),
        Vec2::splat(28.0),
    );
    let mut layer = ui.new_child(UiBuilder::new().max_rect(place));
    let id = layer.next_auto_id();
    let lit = on || ui.rect_contains_pointer(row.rect) || ui.memory(|memory| memory.has_focus(id));
    layer.set_opacity(if lit { 1.0 } else { 0.0 });
    let label = i18n::Message::new(keys::tools::favourites::OF)
        .with("name", name)
        .text(state.language);
    let author = format!("tools-entry-star-{}", tool.id);
    if star(
        &mut layer,
        on,
        &label,
        &author,
        &star_hint(state.language, on),
    )
    .clicked()
    {
        actions.push(Asked::Tools(Action::ToggleFavourite(tool.id)));
    }
}

fn no_match(ui: &mut Ui, language: Language, query: &str) {
    let p = palette(ui.ctx());
    let words = Message::new(keys::tools::NO_MATCH)
        .with("query", query)
        .text(language);
    ui.add_space(16.0);
    let galley = widgets::wrapped(
        ui,
        &words,
        (13.0, 400.0),
        p.text_1,
        ui.available_width() - 32.0,
    );
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    address(ui, &response, "tools-no-match");
    galley.paint(ui, rect.min + Vec2::new(16.0, 0.0), p.text_1);
}

/// egui has dropped the focus by the time the frame starts: the field empties and takes it back.
/// With nothing typed, it stays dropped.
fn empty_on_escape(ui: &Ui, view: &mut HomeView) {
    if view.typing
        && view.searching()
        && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
    {
        view.search.clear();
        view.focus_search = true;
    }
}

/// Enter opens the first result, the arrow goes down into them, above the first is the field
/// again; Escape from a result empties and goes back to it.
fn keys_of_the_search(
    ui: &Ui,
    view: &mut HomeView,
    field: &Response,
    first: Option<&'static ToolEntry>,
    actions: &mut Actions,
) {
    let pressed = |ui: &Ui, key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    // The field gives the focus up on Enter by itself.
    if field.lost_focus() && pressed(ui, Key::Enter) {
        match first {
            Some(tool) => actions.push(Asked::Tools(Action::Open(tool.id))),
            None => hits::open_first(view, actions),
        }
    }
    super::ui::walk(ui, &view.results, field);
    let focused = ui.memory(egui::Memory::focused);
    if !focused.is_some_and(|id| view.results.contains(&id)) {
        return;
    }
    if view.searching() && pressed(ui, Key::Escape) {
        view.search.clear();
        field.request_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_are_as_many_as_the_width_holds_and_never_none() {
        assert_eq!(column_count(100.0, 260.0, 14.0), 1);
        assert_eq!(column_count(534.0, 260.0, 14.0), 2);
        assert_eq!(column_count(1_000.0, 260.0, 14.0), 3);
    }

    #[test]
    fn the_stacks_keep_the_reading_order_and_the_tallest_as_short_as_it_can_be() {
        let stacks = stack_in_columns(&[5, 5, 5, 5, 4, 5, 10], 3);

        assert_eq!(stacks, [0..2, 2..5, 5..7]);
    }

    #[test]
    fn fewer_items_than_columns_leave_no_empty_stack() {
        assert_eq!(stack_in_columns(&[3], 4).len(), 1);
        assert!(stack_in_columns(&[], 4).is_empty());
    }
}
