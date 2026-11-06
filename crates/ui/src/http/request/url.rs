//! The URL, its `{{variables}}` lit in the colour of where each resolves from — red when nothing
//! sets it. What a variable is, and where it stands, is the core's (`variables::spans`).

use std::collections::HashMap;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Response, Stroke, TextEdit, Ui};
use engine::http::variables::{UsedVariable, VariableScope};

use crate::api;
use crate::theme::palette;
use crate::theme::tokens::Palette;
use crate::widgets;

pub(crate) const HEIGHT: f32 = 38.0;
const SIZE: f32 = 13.5;

/// By name, where each variable resolves from; `None` when nothing sets it.
pub(crate) fn scopes(used: &[UsedVariable]) -> HashMap<String, Option<VariableScope>> {
    used.iter()
        .map(|known| {
            let scope = known.resolved.as_ref().map(|found| found.scope);
            (known.name.clone(), scope)
        })
        .collect()
}

/// The hue a variable wears: what sets it, red when nothing does.
pub(crate) fn hue(p: &Palette, scope: Option<VariableScope>) -> Color32 {
    match scope {
        None => p.red,
        Some(VariableScope::Request) => p.green,
        Some(VariableScope::Folder) => p.teal,
        Some(VariableScope::Collection) => p.blue,
        Some(VariableScope::Environment) => p.amber_text,
        Some(VariableScope::Globals) => p.pink,
        Some(VariableScope::Dynamic) => p.purple,
    }
}

pub(crate) struct UrlField<'a> {
    pub(crate) label: &'a str,
    pub(crate) hint: &'a str,
    pub(crate) scopes: &'a HashMap<String, Option<VariableScope>>,
}

pub(crate) fn show(ui: &mut Ui, url: &mut String, field: &UrlField<'_>, width: f32) -> Response {
    let p = palette(ui.ctx());
    let scopes = field.scopes;
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
        let job = lit(text.as_str(), scopes, p);
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(11, 0));
    frame
        .show(ui, |ui| {
            let response = ui.add_sized(
                egui::vec2(width - 24.0, HEIGHT - 2.0),
                TextEdit::singleline(url)
                    .frame(egui::Frame::new())
                    .margin(Margin::ZERO)
                    .hint_text(field.hint)
                    .font(FontId::new(SIZE, FontFamily::Monospace))
                    .align(egui::Align2::LEFT_CENTER)
                    .layouter(&mut layouter),
            );
            let label = field.label.to_owned();
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(label));
            widgets::address(ui, &response, "http-url");
            response
        })
        .inner
}

/// The URL as its parts: plain text, and each variable in its hue on a tint of it.
fn lit(text: &str, scopes: &HashMap<String, Option<VariableScope>>, p: &Palette) -> LayoutJob {
    let font = FontId::new(SIZE, FontFamily::Monospace);
    let plain = TextFormat::simple(font.clone(), p.text_0);
    let mut job = LayoutJob::default();
    let mut last = 0;
    for (range, name) in api::http::spans(text) {
        job.append(&text[last..range.start], 0.0, plain.clone());
        // Amber while Rust has not said where it resolves from.
        let hue = scopes
            .get(&name)
            .map_or(p.amber_text, |scope| hue(p, *scope));
        let format = TextFormat {
            background: hue.gamma_multiply(0.16),
            ..TextFormat::simple(font.clone(), hue)
        };
        job.append(&text[range.clone()], 0.0, format);
        last = range.end;
    }
    job.append(&text[last..], 0.0, plain);
    job
}
