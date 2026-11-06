//! Under the tools that match: what the search finds inside the references, a row an entry.

use egui::{CornerRadius, Pos2, Role, Sense, Ui, Vec2, WidgetInfo};
use i18n::{Language, Message, keys};

use super::{Action, Asked, HomeView, Opening, SECTION_GAP, enter_mark, section_title};
use crate::action::Actions;
use crate::theme::palette;
use crate::tools::catalogue::reference::Table;
use crate::tools::catalogue::{self, REFERENCES, ToolEntry};
use crate::widgets::{self, address, hovered, interact, towards};

/// How many rows of one reference are listed before « n more ».
const HITS_PER_REFERENCE: usize = 4;
/// A single character is nearly every row of every reference: they are read from the second one
/// on. Their own field, which searches one table, reads the first.
const WORTH_SEARCHING: usize = 2;

struct Hit {
    key: String,
    title: String,
    line: String,
}

struct Found {
    tool: &'static ToolEntry,
    hits: Vec<Hit>,
    /// Found past the ones listed.
    more: usize,
}

/// The tables, built by the first thing typed and once a language, and what the search last found.
#[derive(Default)]
pub(super) struct References {
    tables: Option<(Language, Vec<(&'static ToolEntry, Table)>)>,
    query: String,
    found: Vec<Found>,
}

impl References {
    pub(super) fn search(&mut self, query: &str, language: Language) {
        if query == self.query && self.tables.as_ref().is_none_or(|it| it.0 == language) {
            return;
        }
        query.clone_into(&mut self.query);
        self.found.clear();
        if query.chars().count() < WORTH_SEARCHING {
            return;
        }
        let (_, tables) = self
            .tables
            .take()
            .filter(|(built, _)| *built == language)
            .unwrap_or_else(|| (language, built(language)));
        self.found = tables.iter().filter_map(|it| found_in(it, query)).collect();
        self.tables = Some((language, tables));
    }

    pub(super) fn is_empty(&self) -> bool {
        self.found.is_empty()
    }
}

fn built(language: Language) -> Vec<(&'static ToolEntry, Table)> {
    REFERENCES
        .iter()
        .filter_map(|(id, table)| Some((catalogue::find(id)?, table(language))))
        .collect()
}

fn found_in((tool, table): &(&'static ToolEntry, Table), query: &str) -> Option<Found> {
    let rows = table.found(query);
    let hits: Vec<Hit> = rows
        .iter()
        .take(HITS_PER_REFERENCE)
        .map(|row| Hit {
            key: row.key.clone(),
            title: row.found_as.0.clone(),
            line: row.found_as.1.clone(),
        })
        .collect();
    (!hits.is_empty()).then(|| Found {
        tool,
        hits,
        more: rows.len().saturating_sub(HITS_PER_REFERENCE),
    })
}

/// On that row: its table whole, the row brought into view and lit.
fn open_hit(view: &mut HomeView, (tool, key): (&'static ToolEntry, String), actions: &mut Actions) {
    view.opening = Some((tool, Opening::OnRow(key)));
    actions.push(Asked::Tools(Action::Open(tool.id)));
}

/// What Enter opens when no tool matches.
pub(super) fn open_first(view: &mut HomeView, actions: &mut Actions) {
    let first = view.references.found.first();
    let hit = first.and_then(|found| Some((found.tool, found.hits.first()?.key.clone())));
    if let Some(hit) = hit {
        open_hit(view, hit, actions);
    }
}

pub(super) fn show(
    ui: &mut Ui,
    language: Language,
    view: &mut HomeView,
    first_is_ours: bool,
    actions: &mut Actions,
) {
    if view.references.found.is_empty() {
        return;
    }
    let found = std::mem::take(&mut view.references.found);
    let title = i18n::translate(language, keys::tools::references::TITLE, &[]);
    let shown = ui.vertical(|ui| {
        ui.add_space(SECTION_GAP);
        section_title(ui, &title);
        for (at, one) in found.iter().enumerate() {
            let place = one.tool.name(language);
            for (row, hit) in one.hits.iter().enumerate() {
                let first = first_is_ours && at == 0 && row == 0;
                if hit_row(ui, (hit, &place), first, language, view).clicked() {
                    open_hit(view, (one.tool, hit.key.clone()), actions);
                }
            }
            if one.more > 0 && more_row(ui, one.more, &place, language, view).clicked() {
                let query = view.search.trim().to_owned();
                view.opening = Some((one.tool, Opening::Searched(query)));
                actions.push(Asked::Tools(Action::Open(one.tool.id)));
            }
        }
    });
    address(ui, &shown.response, "tools-references");
    view.references.found = found;
}

fn hit_row(
    ui: &mut Ui,
    (hit, place): (&Hit, &str),
    first: bool,
    language: Language,
    view: &mut HomeView,
) -> egui::Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let where_ = widgets::wrapped(ui, place, (11.5, 400.0), p.text_2, width * 0.3);
    let room = width - 28.0 - where_.size().x - 16.0;
    let title = widgets::wrapped(ui, &hit.title, (13.0, 600.0), p.text_0, room);
    let line = widgets::wrapped(ui, &hit.line, (11.5, 400.0), p.text_2, room);
    let height = 8.0 + title.size().y + 2.0 + line.size().y + 8.0;
    let name = format!("{} {} {place}", hit.title, hit.line);
    let response = interact(
        ui,
        Vec2::new(width, height),
        Sense::click(),
        "tools-reference-hit",
        || WidgetInfo::labeled(Role::Button, true, &name),
        CornerRadius::same(8),
    );
    view.results.push(response.id);
    let rect = response.rect;
    let fill = towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response));
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    let left = rect.left() + 14.0;
    let title_size = title.size();
    title.paint(ui, Pos2::new(left, rect.top() + 8.0), p.text_0);
    let below = rect.top() + 8.0 + title_size.y + 2.0;
    line.paint(ui, Pos2::new(left, below), p.text_2);
    let at = Pos2::new(
        rect.right() - 14.0 - where_.size().x,
        rect.center().y - where_.size().y / 2.0,
    );
    where_.paint(ui, at, p.text_2);
    if first {
        let cap = Pos2::new(left + title_size.x + 6.0, rect.top() + 7.0);
        enter_mark(ui, cap, language);
    }
    response
}

fn more_row(
    ui: &mut Ui,
    more: usize,
    place: &str,
    language: Language,
    view: &mut HomeView,
) -> egui::Response {
    let p = palette(ui.ctx());
    let words = Message::new(keys::tools::references::MORE)
        .with("count", u32::try_from(more).unwrap_or(u32::MAX))
        .with("name", place)
        .text(language);
    let width = ui.available_width();
    let shown = widgets::wrapped(ui, &words, (12.0, 500.0), p.amber_text, width - 28.0);
    let response = interact(
        ui,
        Vec2::new(width, shown.size().y + 12.0),
        Sense::click(),
        "tools-reference-more",
        || WidgetInfo::labeled(Role::Button, true, &words),
        CornerRadius::same(8),
    );
    view.results.push(response.id);
    let rect = response.rect;
    let fill = towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response));
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    shown.paint(ui, rect.min + Vec2::new(14.0, 6.0), p.amber_text);
    response
}
