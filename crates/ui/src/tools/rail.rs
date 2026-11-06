//! At home the categories and their counts; in a tool its category's tools and the recent ones.
//! In the frame the three areas share, so going from one to the other moves nothing.

use egui::{
    Color32, CornerRadius, FontFamily, Margin, Pos2, Rect, Role, Sense, Ui, Vec2, WidgetInfo,
};
use i18n::keys;

use super::catalogue::{self, Category, TOOLS, ToolEntry};
use super::{Action, Asked};
use crate::action::Actions;
use crate::state::AppState;
use crate::state::tools::{Filter, SHOWN_RECENTS};
use crate::theme::icons::{self, Icon};
use crate::theme::{palette, spacing};
use crate::widgets::{self, hovered, interact, towards};

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let s = spacing(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let margin = Margin {
        left: 10,
        right: 10,
        top: s.card as i8,
        bottom: s.canvas as i8,
    };
    egui::ScrollArea::vertical()
        .id_salt("tools-rail")
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                links(ui, state, actions);
            });
        });
}

fn links(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let tr = |key| i18n::translate(state.language, key, &[]);
    let open = state.tools.open_tool();
    let filter = state.tools.filter();
    let favourites = state.tools.favourites();
    if !favourites.is_empty() {
        let on = open.is_none() && filter == Some(Filter::Favourites);
        let words = tr(keys::tools::favourites::TITLE);
        let link = Link::new(&words, "tools-rail-favourites", on)
            .lead(Lead::Star)
            .count(favourites.len());
        if link.show(ui).clicked() {
            actions.push(Asked::Tools(Action::Home(Some(Filter::Favourites))));
        }
    }
    let on = open.is_none() && filter.is_none();
    let words = tr(keys::tools::ALL);
    let all = Link::new(&words, "tools-rail-all", on).count(TOOLS.len());
    if all.show(ui).clicked() {
        actions.push(Asked::Tools(Action::Home(None)));
    }
    ui.add_space(6.0);
    match open {
        Some(tool) => around_a_tool(ui, state, tool, actions),
        None => categories(ui, state, filter, actions),
    }
}

fn categories(ui: &mut Ui, state: &AppState, filter: Option<Filter>, actions: &mut Actions) {
    let p = palette(ui.ctx());
    for category in Category::ALL {
        let count = catalogue::in_category(category).count();
        let words = i18n::translate(state.language, category.name(), &[]);
        let on = filter == Some(Filter::Category(category));
        let id = format!("tools-rail-category-{}", category.id());
        let link = Link::new(&words, &id, on)
            .lead(Lead::Hue(category.hue(p)))
            .count(count);
        if link.show(ui).clicked() {
            actions.push(Asked::Tools(Action::Home(Some(Filter::Category(category)))));
        }
    }
}

/// The ones not already listed above them, no tool twice in one rail, and five all the same.
fn recents_beside(state: &AppState, tool: &ToolEntry) -> Vec<&'static ToolEntry> {
    state
        .tools
        .recents()
        .map(|(recent, _)| recent)
        .filter(|recent| recent.category != tool.category)
        .take(SHOWN_RECENTS)
        .collect()
}

fn around_a_tool(ui: &mut Ui, state: &AppState, tool: &ToolEntry, actions: &mut Actions) {
    let language = state.language;
    let category = i18n::translate(language, tool.category.name(), &[]);
    heading(ui, &category);
    for sibling in catalogue::in_category(tool.category) {
        let words = sibling.name(language);
        let id = format!("tools-rail-tool-{}", sibling.id);
        if Link::new(&words, &id, sibling.id == tool.id)
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Tools(Action::Open(sibling.id)));
        }
    }
    let recents = recents_beside(state, tool);
    if recents.is_empty() {
        return;
    }
    ui.add_space(12.0);
    let p = palette(ui.ctx());
    let (line, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        line.shrink2(Vec2::new(10.0, 0.0)).x_range(),
        line.top(),
        (1.0, p.line_soft),
    );
    heading(ui, &i18n::translate(language, keys::tools::RECENTS, &[]));
    for recent in recents {
        let words = recent.name(language);
        let id = format!("tools-rail-recent-{}", recent.id);
        if Link::new(&words, &id, false).show(ui).clicked() {
            actions.push(Asked::Tools(Action::Open(recent.id)));
        }
    }
}

/// A label over its rows, not one of them: small, muted, with air above.
fn heading(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    ui.add_space(14.0);
    let galley = widgets::text(ui, words, 11.5, 500.0, FontFamily::Proportional, p.text_2);
    let height = galley.size().y + 4.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    ui.painter()
        .galley(rect.min + Vec2::new(10.0, 0.0), galley, p.text_2);
}

enum Lead {
    None,
    Star,
    Hue(Color32),
}

/// A row of the rail: wrapped rather than cut, the rail opens at 160 points.
struct Link<'a> {
    words: &'a str,
    author_id: &'a str,
    on: bool,
    lead: Lead,
    count: Option<usize>,
}

impl<'a> Link<'a> {
    fn new(words: &'a str, author_id: &'a str, on: bool) -> Self {
        Self {
            words,
            author_id,
            on,
            lead: Lead::None,
            count: None,
        }
    }

    fn lead(mut self, lead: Lead) -> Self {
        self.lead = lead;
        self
    }

    fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    fn show(self, ui: &mut Ui) -> egui::Response {
        let p = palette(ui.ctx());
        let ink = if self.on { p.amber_text } else { p.text_0 };
        let weight = if self.on { 600.0 } else { 400.0 };
        let width = ui.available_width();
        let count = self.count.map(|count| {
            let ink = if self.on { p.amber_text } else { p.text_2 };
            widgets::text(
                ui,
                &count.to_string(),
                11.0,
                400.0,
                FontFamily::Monospace,
                ink,
            )
        });
        let lead = match self.lead {
            Lead::None => 0.0,
            Lead::Star => 13.0 + 9.0,
            Lead::Hue(_) => 8.0 + 9.0,
        };
        let room = width - 20.0 - lead - count.as_ref().map_or(0.0, |it| it.size().x + 9.0);
        let label = widgets::wrapped(ui, self.words, (13.0, weight), ink, room);
        let (words, on) = (self.words, self.on);
        let corner = CornerRadius::same(7);
        let response = interact(
            ui,
            Vec2::new(width, label.size().y + 14.0),
            Sense::click(),
            self.author_id,
            || WidgetInfo::selected(Role::Button, true, on, words),
            corner,
        );
        let rect = response.rect;
        let fill = if self.on {
            p.bg_3
        } else {
            towards(Color32::TRANSPARENT, p.bg_2, hovered(ui, &response))
        };
        ui.painter().rect_filled(rect, corner, fill);
        let middle = rect.center().y;
        let left = rect.left() + 10.0;
        match self.lead {
            Lead::None => {}
            Lead::Star => {
                let glyph =
                    Rect::from_center_size(Pos2::new(left + 6.5, middle), Vec2::splat(13.0));
                icons::paint(ui, glyph, Icon::StarFilled, p.amber_text);
            }
            Lead::Hue(hue) => {
                let dot = Rect::from_center_size(Pos2::new(left + 4.0, middle), Vec2::splat(8.0));
                ui.painter().rect_filled(dot, CornerRadius::same(2), hue);
            }
        }
        let top = middle - label.size().y / 2.0;
        label.paint(ui, Pos2::new(left + lead, top), ink);
        if let Some(count) = count {
            let at = Pos2::new(
                rect.right() - 10.0 - count.size().x,
                middle - count.size().y / 2.0,
            );
            ui.painter().galley(at, count, p.text_2);
        }
        response
    }
}
