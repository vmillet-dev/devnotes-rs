//! « Extract to a variable »: the value at a path of the answer becomes a rule of the request,
//! written after each of its sends into the active environment.

use egui::{CornerRadius, FontFamily, Id, Margin, Sense, Stroke, Ui};
use engine::http::model::Extraction;
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::api;
use crate::state::AppState;
use crate::theme::{palette, spacing};
use crate::widgets::{self, Button, Checkbox, Kind, TextField};

const WIDTH: f32 = 440.0;

/// The form while it is open: the name and the secret are proposed by the core from the path.
pub(crate) struct Extracting {
    path: String,
    name: String,
    secret: bool,
}

impl Extracting {
    pub(crate) fn of(explorer_path: &str) -> Self {
        let path = api::http::body_path(explorer_path);
        Self {
            name: api::http::variable_name_of(&path),
            secret: api::http::looks_secret(&path),
            path,
        }
    }
}

pub(super) fn show(
    ui: &Ui,
    state: &AppState,
    form: &mut Option<Extracting>,
    actions: &mut Actions,
) {
    let Some(tab) = state.http.tabs().active() else {
        *form = None;
        return;
    };
    let Some(open) = form else {
        return;
    };
    let p = palette(ui.ctx());
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let padding = spacing(ui.ctx()).canvas.round() as i8;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(14))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(padding));
    let shown = egui::Modal::new(Id::unique("http-extract-dialog"))
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ui.ctx(), |ui| body(ui, open, (&tab.draft.name, state)));
    match shown.inner {
        Some(true) => {
            let rule = Extraction {
                path: open.path.clone(),
                variable: open.name.trim().to_owned(),
                secret: open.secret,
            };
            actions.push(Asked::Http(Action::AddExtraction {
                key: tab.key.clone(),
                rule,
            }));
            *form = None;
        }
        Some(false) => *form = None,
        None if shown.should_close() => *form = None,
        None => {}
    }
}

/// `true` on « Extract », `false` on « Cancel ».
fn body(ui: &mut Ui, form: &mut Extracting, (request, state): (&str, &AppState)) -> Option<bool> {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = tr(keys::http::extract::TITLE);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "http-extract-dialog");
    caption(ui, &tr(keys::http::extract::PATH));
    let path = egui::RichText::new(&form.path)
        .monospace()
        .size(12.5)
        .color(p.text_0);
    let path = ui.add(egui::Label::new(path).truncate());
    widgets::address(ui, &path, "http-extract-path");
    let label = tr(keys::http::extract::NAME);
    caption(ui, &label);
    TextField::new(&mut form.name, &label, "http-extract-name").show(ui);
    let secret = tr(keys::http::extract::SECRET);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        Checkbox::new(&mut form.secret, &secret, "http-extract-secret").show(ui);
        let words = egui::RichText::new(&secret).size(12.5).color(p.text_1);
        ui.add(egui::Label::new(words));
    });
    let into = into_words(request, state);
    let into = egui::RichText::new(into).size(12.0).color(p.text_2);
    let into = ui.add(egui::Label::new(into).wrap());
    widgets::address(ui, &into, "http-extract-into");
    let mut chosen = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::extract::SUBMIT), "http-extract-submit")
            .kind(Kind::Filled)
            .enabled(!form.name.trim().is_empty())
            .show(ui)
            .clicked()
        {
            chosen = Some(true);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-extract-cancel")
            .show(ui)
            .clicked()
        {
            chosen = Some(false);
        }
    });
    chosen
}

fn caption(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(words).size(12.0).color(p.text_2);
    ui.add(egui::Label::new(words));
}

/// Where the value will go: the active environment, or the globals with none.
fn into_words(request: &str, state: &AppState) -> String {
    let language = state.language;
    match state.http.environments().active() {
        Some(active) => i18n::translate(
            language,
            keys::http::extract::INTO,
            &[
                ("request", request.into()),
                ("environment", active.name.as_str().into()),
            ],
        ),
        None => i18n::translate(
            language,
            keys::http::extract::INTO_GLOBALS,
            &[("request", request.into())],
        ),
    }
}
