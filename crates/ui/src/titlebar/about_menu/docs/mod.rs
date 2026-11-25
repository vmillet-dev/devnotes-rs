//! The documentation, read offline: the pages devnotes.fr shows. Their list and a search on the
//! left, the page on the right; a link elsewhere opens the browser.

mod page;
mod text;

use egui::{
    Align, Color32, CornerRadius, FontFamily, Layout, Rect, Role, Sense, Stroke, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};
use engine::docs::model::{Block, Page};
use engine::docs::search::Hit;
use i18n::{Language, Message, keys};

use super::Panel;
use crate::action::{Action as Asked, Actions};
use crate::api::help;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::theme::tokens::{Palette, font, gap, radius};
use crate::titlebar::Action;
use crate::widgets::{self, Button, SearchField};

const SIZE: Vec2 = vec2(980.0, 720.0);
const RAIL: f32 = 250.0;
const HEADER: f32 = 48.0;
/// The site's button, under the list.
const FOOTER: f32 = 48.0;
/// A line of text stays readable however wide the window is.
const WIDEST: f32 = 680.0;

/// Where the About menu opens it.
pub(super) const FIRST: &str = "overview";
/// The page about a note, for a canvas that has none yet.
pub(crate) const WRITING: &str = "writing";
/// The page about folders and the board.
pub(crate) const ORGANISING: &str = "organising";

#[derive(Default)]
pub(super) struct View {
    query: String,
    /// What was searched when `hits` were found.
    searched: String,
    hits: Vec<Hit>,
    /// The section to scroll to, once its page is drawn.
    section: Option<String>,
    /// The page on screen: another one starts at its top.
    shown: Option<&'static str>,
}

/// What a click in the documentation asks.
enum Go {
    Page(&'static str, Option<String>),
    Address(String),
    Copy(String),
}

/// A page's name as the index holds it.
fn entry(id: &str) -> Option<&'static str> {
    help::docs_index()
        .pages()
        .find(|entry| entry.id == id)
        .map(|entry| entry.id.as_str())
}

/// The page's colour: a folder's, as the index names it.
fn swatch(p: &Palette, id: &str) -> Color32 {
    let colour = help::docs_index()
        .pages()
        .find(|entry| entry.id == id)
        .map_or("", |entry| entry.colour.as_str());
    match colour {
        "amber" => p.amber_fill,
        "green" => p.green,
        "blue" => p.blue,
        "purple" => p.purple,
        "teal" => p.teal,
        "olive" => p.olive,
        "pink" => p.pink,
        _ => p.text_2,
    }
}

pub(super) fn show(
    ctx: &egui::Context,
    state: &AppState,
    id: &'static str,
    (view, panel): (&mut View, &mut Option<Panel>),
    actions: &mut Actions,
) -> bool {
    let cross = i18n::translate(state.language, keys::docs::CLOSE, &[]);
    let mut asked = None;
    let shown = widgets::framed_dialog(
        ctx,
        &state.dialogs,
        (super::id(), (&cross, "docs-close")),
        SIZE,
        |ui| asked = body(ui, state, id, view),
    );
    match asked {
        Some(Go::Page(id, section)) => {
            view.section = section;
            view.shown = view.section.is_some().then_some(id);
            *panel = Some(Panel::Docs(id));
        }
        Some(Go::Address(address)) => actions.push(Asked::Menu(Action::OpenLink(address))),
        Some(Go::Copy(text)) => actions.push(Asked::Menu(Action::CopyExample(text))),
        None => {}
    }
    shown.is_some_and(|shown| shown.should_close())
}

fn body(ui: &mut Ui, state: &AppState, id: &'static str, view: &mut View) -> Option<Go> {
    let p = palette(ui.ctx());
    let whole = ui.max_rect();
    let line = Stroke::new(gap::HAIRLINE, p.line_soft);
    let top = whole.top() + HEADER;
    header(ui, state, whole);
    ui.painter().hline(whole.x_range(), top, line);

    let rail = Rect::from_min_max(
        pos2(whole.left(), top),
        pos2(whole.left() + RAIL, whole.bottom()),
    );
    let corner = CornerRadius {
        sw: radius::DIALOG,
        ..CornerRadius::ZERO
    };
    ui.painter().rect_filled(rail, corner, p.bg_0);
    ui.painter().vline(rail.right(), rail.y_range(), line);
    let mut list = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rail.shrink2(vec2(gap::INSET, gap::INSET)))
            .layout(Layout::top_down(Align::Min)),
    );
    let chosen = rail_content(&mut list, state, id, view);

    let room = Rect::from_min_max(pos2(rail.right(), top), whole.right_bottom());
    let mut reading = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(room)
            .layout(Layout::top_down(Align::Min)),
    );
    chosen.or(reader(&mut reading, state, id, view))
}

fn header(ui: &mut Ui, state: &AppState, whole: Rect) {
    let p = palette(ui.ctx());
    let title = i18n::translate(state.language, keys::docs::TITLE, &[]);
    let words = widgets::text(
        ui,
        &title,
        font::TITLE,
        600.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let at = pos2(
        whole.left() + gap::PANEL,
        whole.top() + (HEADER - words.size().y) / 2.0,
    );
    let heading = ui.allocate_rect(Rect::from_min_size(at, words.size()), Sense::hover());
    ui.painter().galley(at, words, p.text_0);
    heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title));
    widgets::address(ui, &heading, "docs-dialog");
}

/// The search, then what it found, or every page; the site under them.
fn rail_content(ui: &mut Ui, state: &AppState, id: &str, view: &mut View) -> Option<Go> {
    let whole = ui.max_rect();
    let mut asked = search(ui, state, view);
    ui.add_space(gap::CONTROLS);
    let list = Rect::from_min_max(
        ui.cursor().min,
        pos2(whole.right(), whole.bottom() - FOOTER),
    );
    let mut area = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(list)
            .layout(Layout::top_down(Align::Min)),
    );
    let scrolled = egui::ScrollArea::vertical()
        .id_salt("docs-rail")
        .auto_shrink(false)
        .show(&mut area, |ui| {
            ui.spacing_mut().item_spacing.y = gap::LINE;
            if view.query.trim().is_empty() {
                contents(ui, state, id)
            } else {
                results(ui, state, view)
            }
        });
    asked = asked.or(scrolled.inner);

    let site = Rect::from_min_max(pos2(whole.left(), whole.bottom() - FOOTER), whole.max);
    let mut foot = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(site)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let address = site_of(state.language);
    if Button::new(address.trim_start_matches("https://"), "docs-site")
        .icon(Icon::Link)
        .show(&mut foot)
        .clicked()
    {
        asked = Some(Go::Address(address.to_owned()));
    }
    asked
}

fn site_of(language: Language) -> &'static str {
    match language {
        Language::Fr => "https://devnotes.fr/docs",
        Language::En => "https://devnotes.fr/en/docs",
    }
}

/// Enter opens the first answer.
fn search(ui: &mut Ui, state: &AppState, view: &mut View) -> Option<Go> {
    let tr = |key| i18n::translate(state.language, key, &[]);
    let (name, hint, clear) = (
        tr(keys::docs::SEARCH),
        tr(keys::docs::SEARCH_HINT),
        tr(keys::docs::CLEAR),
    );
    let searched = SearchField::new(&mut view.query, &name, "docs-search")
        .hint(&hint)
        .clear(&clear, "docs-search-clear")
        .show(ui);
    if searched.cleared {
        view.query.clear();
    }
    if view.query != view.searched {
        view.hits = help::search_docs(state.language, &view.query);
        view.searched.clone_from(&view.query);
    }
    let entered =
        searched.field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let first = view.hits.first().filter(|_| entered)?;
    let went = went_to(first);
    view.query.clear();
    Some(went)
}

fn went_to(hit: &Hit) -> Go {
    Go::Page(&hit.page.id, hit.section.map(str::to_owned))
}

/// Every page under its group; the sections of the one that is read, under it.
fn contents(ui: &mut Ui, state: &AppState, reading: &str) -> Option<Go> {
    let p = palette(ui.ctx());
    let language = state.language.code();
    let mut asked = None;
    for group in &help::docs_index().groups {
        let name = group.name.of(language);
        let words = widgets::text(
            ui,
            name,
            font::META,
            600.0,
            FontFamily::Proportional,
            p.text_2,
        );
        let size = vec2(ui.available_width(), words.size().y + gap::GROUPS);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
        let at = pos2(
            rect.left() + gap::SNUG,
            rect.bottom() - words.size().y - gap::LINE,
        );
        ui.painter().galley(at, words, p.text_2);
        for entry in &group.pages {
            let on = entry.id == reading;
            let name = entry.name.of(language);
            if row(
                ui,
                name,
                &format!("docs-page-{}", entry.id),
                Some(&entry.id),
                on,
            ) {
                asked = Some(Go::Page(&entry.id, None));
            }
            if on && let Some(page) = help::docs_page(state.language, &entry.id) {
                asked = asked.or(sections(ui, page));
            }
        }
    }
    asked
}

fn sections(ui: &mut Ui, page: &'static Page) -> Option<Go> {
    let mut asked = None;
    for block in &page.body {
        if let Block::Heading { level: 2, id, text } = block
            && row(ui, text, &format!("docs-section-{id}"), None, false)
        {
            asked = Some(Go::Page(&page.id, Some(id.clone())));
        }
    }
    asked
}

/// A page with its colour, or a section of the page that is read, further in and dimmer.
fn row(ui: &mut Ui, name: &str, author_id: &str, page: Option<&str>, on: bool) -> bool {
    const SQUARE: f32 = 8.0;
    let p = palette(ui.ctx());
    let height = if page.is_some() { 26.0 } else { 24.0 };
    let corner = CornerRadius::same(radius::CONTROL);
    let response = widgets::interact(
        ui,
        vec2(ui.available_width(), height),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::Link, true, on, name),
        corner,
    );
    let rect = response.rect;
    if on {
        ui.painter()
            .rect_filled(rect, corner, p.amber_fill.gamma_multiply(0.12));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_2);
    }
    let left = rect.left() + gap::CONTROLS;
    if let Some(id) = page {
        let square = Rect::from_center_size(
            pos2(left + SQUARE / 2.0, rect.center().y),
            Vec2::splat(SQUARE),
        );
        ui.painter()
            .rect_filled(square, CornerRadius::same(radius::MARK), swatch(p, id));
    }
    let ink = match (on, response.hovered(), page.is_some()) {
        (true, ..) => p.amber_text,
        (_, true, _) => p.text_0,
        (_, _, true) => p.text_1,
        _ => p.text_2,
    };
    let size = if page.is_some() {
        font::BODY
    } else {
        font::LABEL
    };
    let left = left + SQUARE + gap::INSET;
    let words = widgets::one_line(
        ui,
        name,
        (size, 400.0),
        ink,
        rect.right() - left - gap::TIGHT,
    );
    let at = pos2(left, rect.center().y - words.size().y / 2.0);
    ui.painter().galley(at, words, ink);
    response.clicked()
}

fn results(ui: &mut Ui, state: &AppState, view: &mut View) -> Option<Go> {
    let p = palette(ui.ctx());
    if view.hits.is_empty() {
        let none = Message::new(keys::docs::NONE)
            .with("query", view.query.trim())
            .text(state.language);
        let said = ui.label(egui::RichText::new(&none).size(font::LABEL).color(p.text_2));
        widgets::address(ui, &said, "docs-none");
        return None;
    }
    let chosen = view.hits.iter().find(|hit| result(ui, hit)).map(went_to)?;
    view.query.clear();
    Some(chosen)
}

/// Where the words are, then what is around them.
fn result(ui: &mut Ui, hit: &Hit) -> bool {
    let p = palette(ui.ctx());
    let place = match hit.section {
        Some(_) => format!("{} › {}", hit.page.title, hit.heading),
        None => hit.page.title.clone(),
    };
    let width = ui.available_width();
    let inside = width - 2.0 * gap::CONTROLS;
    let name = widgets::one_line(ui, &place, (font::LABEL, 600.0), p.text_0, inside);
    let mut job = egui::text::LayoutJob::simple(
        hit.excerpt.clone(),
        egui::FontId::proportional(font::CONTROL),
        p.text_1,
        inside,
    );
    job.wrap.max_rows = 3;
    let about = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let height = name.size().y + about.size().y + gap::TIGHT + 2.0 * gap::SNUG;
    let corner = CornerRadius::same(radius::CONTROL);
    let response = widgets::interact(
        ui,
        vec2(width, height),
        Sense::click(),
        "docs-result",
        || WidgetInfo::labeled(Role::Link, true, &place),
        corner,
    );
    if response.hovered() {
        ui.painter().rect_filled(response.rect, corner, p.bg_2);
    }
    let at = response.rect.min + vec2(gap::CONTROLS, gap::SNUG);
    let under = at + vec2(0.0, name.size().y + gap::TIGHT);
    ui.painter().galley(at, name, p.text_0);
    ui.painter().galley(under, about, p.text_1);
    response.clicked()
}

fn reader(ui: &mut Ui, state: &AppState, id: &'static str, view: &mut View) -> Option<Go> {
    let page = help::docs_page(state.language, id)?;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("docs-page")
        .auto_shrink(false);
    if view.shown != Some(id) {
        area = area.vertical_scroll_offset(0.0);
        view.shown = Some(id);
    }
    let scrolled = area.show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(gap::margin(gap::PAGE, gap::PANEL))
            .show(ui, |ui| {
                let room = ui.available_width();
                ui.set_width(room);
                ui.set_max_width(room.min(WIDEST));
                page::show(ui, state, page, &mut view.section)
            })
            .inner
    });
    let region = widgets::region(ui, scrolled.inner_rect, "docs-reader", Sense::hover());
    // A place on screen for the scenarios to scroll over: a node with no role has none.
    region.widget_info(|| WidgetInfo::labeled(Role::ScrollView, true, &page.title));
    widgets::address(ui, &region, "docs-reader");
    scrolled.inner
}
