//! A response's body: laid out and coloured, as it came, as a graph or a tree, or an image. A
//! body up to a megabyte is drawn by lines — the lines in view alone, coloured window by window
//! on the parser's thread — so a long answer costs what a screenful does.

use std::ops::Range;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{FontFamily, FontId, Ui, Vec2};
use engine::http::send::SentResponse;
use engine::json::read::StructureFormat;
use engine::notes::language::Language;
use i18n::keys;

use super::super::Action;
use crate::action::{Action as Asked, Actions};
use crate::services::highlight::{self, Colouring};
use crate::state::AppState;
use crate::theme::palette;
use crate::widgets::json_explorer::{Explorer, ExplorerView, Mode};
use crate::widgets::{self, Segmented};

const LINE: f32 = 18.0;
const SIZE: f32 = 12.5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum View {
    #[default]
    Pretty,
    Raw,
    Graph,
    Tree,
}

/// One answer's text as lines, its colours, and how it is shown.
pub(super) struct BodyView {
    /// The send and the view the lines were cut for.
    cut_for: Option<(String, bool)>,
    text: Arc<str>,
    lines: Vec<Range<usize>>,
    rope: text::Rope,
    colouring: Option<Colouring>,
    view: View,
    explorer: Explorer,
    pub(super) extracting: Option<crate::http::extract::Extracting>,
}

impl Default for BodyView {
    fn default() -> Self {
        Self {
            cut_for: None,
            text: Arc::from(""),
            lines: Vec::new(),
            rope: text::Rope::new(),
            colouring: None,
            view: View::Pretty,
            explorer: Explorer::default(),
            extracting: None,
        }
    }
}

impl BodyView {
    /// The text a view shows, cut into lines once per send and view.
    fn follow(&mut self, ui: &Ui, (id, response): (&str, &SentResponse)) {
        let pretty = self.view == View::Pretty;
        if self
            .cut_for
            .as_ref()
            .is_some_and(|(cut, was)| cut == id && *was == pretty)
        {
            return;
        }
        self.cut_for = Some((id.to_owned(), pretty));
        let shown = if pretty {
            response.pretty.as_deref().unwrap_or(&response.body)
        } else {
            &response.body
        };
        self.text = Arc::from(shown);
        self.lines = lines_of(&self.text);
        self.rope = text::Rope::from_str(&self.text);
        self.colouring = pretty
            .then(|| crate::notes::language::syntax(response.language))
            .flatten()
            .and_then(|syntax| Colouring::new(ui.ctx(), syntax));
    }
}

/// Each line's byte range, its ending left out.
fn lines_of(text: &str) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (at, _) in text.match_indices('\n') {
        let end = if text[..at].ends_with('\r') {
            at - 1
        } else {
            at
        };
        lines.push(start..end);
        start = at + 1;
    }
    lines.push(start..text.len());
    lines
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (id, response): (&str, &SentResponse),
    view: &mut BodyView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    if response.binary {
        image(ui, state, (id, response), actions);
        return;
    }
    if response.body.is_empty() {
        let words =
            egui::RichText::new(i18n::translate(language, keys::http::response::EMPTY, &[]))
                .size(12.5)
                .color(p.text_2);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-response-empty");
        return;
    }
    let json = response.language == Language::Json;
    if !json && matches!(view.view, View::Graph | View::Tree) {
        view.view = View::Pretty;
    }
    views(ui, view, (json, response), language);
    view.follow(ui, (id, response));
    match view.view {
        View::Pretty | View::Raw => lines(ui, view, view.view == View::Pretty),
        View::Graph | View::Tree => explore(ui, state, (id, response), view, actions),
    }
}

fn views(
    ui: &mut Ui,
    view: &mut BodyView,
    (json, response): (bool, &SentResponse),
    language: i18n::Language,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let mut offered = vec![
        (View::Pretty, tr(keys::http::response::views::PRETTY)),
        (View::Raw, tr(keys::http::response::views::RAW)),
    ];
    if json {
        offered.push((View::Graph, tr(keys::http::response::views::GRAPH)));
        offered.push((View::Tree, tr(keys::http::response::views::TREE)));
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let options: Vec<(View, &str)> = offered
            .iter()
            .map(|(view, name)| (*view, name.as_str()))
            .collect();
        Segmented::new(&options, "http-response-view").show(ui, &mut view.view);
        if response.cut {
            let key = if response.incomplete {
                keys::http::response::INCOMPLETE
            } else {
                keys::http::response::CUT
            };
            let words = egui::RichText::new(tr(key)).size(11.5).color(p.text_2);
            let said = ui.add(egui::Label::new(words));
            widgets::address(ui, &said, "http-response-cut");
        }
    });
}

/// The lines in view alone, a gutter of numbers, the colours as they arrive.
fn lines(ui: &mut Ui, view: &mut BodyView, coloured: bool) {
    let p = palette(ui.ctx());
    let count = view.lines.len();
    let digits = count.max(1).ilog10() + 1;
    let gutter = widgets::text(
        ui,
        &"8".repeat(digits as usize),
        SIZE,
        400.0,
        FontFamily::Monospace,
        p.line_no,
    )
    .size()
    .x + 16.0;
    let author_id = if coloured {
        "http-response-pretty"
    } else {
        "http-response-body"
    };
    let shown = egui::ScrollArea::both()
        .id_salt(("http-response-lines", author_id))
        .auto_shrink([false, false])
        .show_rows(ui, LINE, count, |ui, range| {
            if coloured && let Some(colouring) = view.colouring.as_mut() {
                colouring.window(&view.rope, range.clone());
            }
            ui.spacing_mut().item_spacing.y = 0.0;
            for at in range {
                let line = &view.text[view.lines[at].clone()];
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let number = widgets::text(
                        ui,
                        &(at + 1).to_string(),
                        SIZE,
                        400.0,
                        FontFamily::Monospace,
                        p.line_no,
                    );
                    let (cell, _) =
                        ui.allocate_exact_size(Vec2::new(gutter, LINE), egui::Sense::hover());
                    ui.painter().galley(
                        egui::pos2(cell.right() - 10.0 - number.size().x, cell.top() + 1.0),
                        number,
                        p.line_no,
                    );
                    let tokens = view
                        .colouring
                        .as_ref()
                        .and_then(|colouring| colouring.line(at));
                    let job = coloured_line(line, tokens, ui);
                    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(galley.size().x, LINE),
                        egui::Sense::hover(),
                    );
                    ui.painter().galley(
                        egui::pos2(rect.left(), rect.top() + 1.0),
                        galley,
                        p.text_0,
                    );
                });
            }
        });
    let region = widgets::region(ui, shown.inner_rect, author_id, egui::Sense::hover());
    let label = view.text.chars().take(200).collect::<String>();
    region.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Document, true, &label));
    widgets::address(ui, &region, author_id);
}

fn coloured_line(line: &str, tokens: Option<&text::highlight::Line>, ui: &Ui) -> LayoutJob {
    let p = palette(ui.ctx());
    let font = FontId::new(SIZE, FontFamily::Monospace);
    let mut job = LayoutJob::default();
    let mut last = 0;
    for (range, token) in tokens.into_iter().flatten() {
        let (start, end) = (range.start.min(line.len()), range.end.min(line.len()));
        if start < last || !line.is_char_boundary(start) || !line.is_char_boundary(end) {
            continue;
        }
        job.append(
            &line[last..start],
            0.0,
            TextFormat::simple(font.clone(), p.text_0),
        );
        let (colour, italics) = highlight::colour(p, *token);
        let format = TextFormat {
            italics,
            ..TextFormat::simple(font.clone(), colour)
        };
        job.append(&line[start..end], 0.0, format);
        last = end;
    }
    job.append(&line[last..], 0.0, TextFormat::simple(font, p.text_0));
    job
}

/// The graph or the tree of a JSON answer, explored by the core once asked.
fn explore(
    ui: &mut Ui,
    state: &AppState,
    (id, response): (&str, &SentResponse),
    view: &mut BodyView,
    actions: &mut Actions,
) {
    let query = view
        .explorer
        .query(response.body.clone(), StructureFormat::Json);
    actions.push(Asked::Http(Action::Explore {
        send_id: id.to_owned(),
        query,
    }));
    let Some((explored, text)) = state.http.explored(id) else {
        return;
    };
    let mode = if view.view == View::Tree {
        Mode::Tree
    } else {
        Mode::Graph
    };
    // Its own room: the explorer fills the rectangle it is given.
    let room = ui.available_rect_before_wrap();
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(room));
    let output = ExplorerView::new(&mut view.explorer, explored, text)
        .mode(mode)
        .format(StructureFormat::Json)
        .language(state.language)
        .with_code(false)
        .extractable(true)
        .show(&mut inside);
    if output.tree_proposed {
        view.view = View::Tree;
    }
    if let Some(path) = output.extract {
        view.extracting = Some(crate::http::extract::Extracting::of(&path));
    }
}

/// An image answer, shown when Rust kept its bytes; any other binary said by its type.
fn image(
    ui: &mut Ui,
    state: &AppState,
    (id, response): (&str, &SentResponse),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let kind = super::content_type(response).unwrap_or("?");
    if kind.to_ascii_lowercase().starts_with("image/") {
        actions.push(Asked::Http(Action::ResponseImage(id.to_owned())));
        if let Some(texture) = state.http.response_image(id) {
            let size = texture.size_vec2();
            let room = ui.available_size();
            let scale = (room.x / size.x).min(room.y / size.y).min(1.0);
            let shown = ui.add(egui::Image::new(texture).fit_to_exact_size(size * scale));
            let alt = i18n::translate(
                state.language,
                keys::http::response::IMAGE_ALT,
                &[("type", kind.into())],
            );
            shown.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, &alt));
            widgets::address(ui, &shown, "http-response-image");
            return;
        }
    }
    let words = i18n::translate(
        state.language,
        keys::http::response::BINARY,
        &[("type", kind.into())],
    );
    let said = ui.add(egui::Label::new(
        egui::RichText::new(words).size(12.5).color(p.text_2),
    ));
    widgets::address(ui, &said, "http-response-binary");
}
