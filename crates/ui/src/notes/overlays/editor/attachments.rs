//! The note's files (the reference's `attachment-strip`): each with its name and size, what can
//! be done with it, and under them the one image shown. The first click on a cross arms it.

use egui::{CornerRadius, Margin, Rect, RichText, Sense, Stroke, Ui, Vec2};
use engine::attachments::model::Attachment;
use i18n::{Language, keys};

use crate::action::Actions;
use crate::notes::Editing;
use crate::state::AppState;
use crate::state::attachments::{Asked, is_shown};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind};

const BYTES_PER_KB: u32 = 1024;
/// The file size the core accepts, in the hint's words.
const MAX_MEGABYTES: u32 = 10;
const NAME_WIDTH: f32 = 220.0;
/// Height-bounded: a full-size screenshot would push the editor off screen.
const PREVIEW_HEIGHT: f32 = 220.0;

fn ask(actions: &mut Actions, asked: Asked) {
    actions.push(super::editor(Editing::Attachment(asked)));
}

/// `titled`: the band says what it is; the details column names it above instead.
pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (confirming, titled): (&mut Option<String>, bool),
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let attachments = &state.attachments;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
        if titled {
            let title = i18n::translate(language, keys::attachments::TITLE, &[]);
            ui.label(RichText::new(title).size(11.0).color(p.text_2));
        }
        for attachment in attachments.list() {
            item(ui, state, attachment, confirming, actions);
        }
        if attachments.list().is_empty() {
            let hint = i18n::translate(
                language,
                keys::attachments::DROP_HINT,
                &[("limit", MAX_MEGABYTES.into())],
            );
            let response = ui.label(RichText::new(hint).size(11.0).color(p.text_2));
            widgets::address(ui, &response, "attachment-empty");
        }
        let key = if attachments.is_busy() {
            keys::attachments::ADDING
        } else {
            keys::attachments::ADD
        };
        let words = i18n::translate(language, key, &[]);
        let add = Button::new(&words, "attachment-add").enabled(!attachments.is_busy());
        if add.kind(Kind::Outline).show(ui).clicked() {
            ask(actions, Asked::Pick);
        }
    });
    preview(ui, state, actions);
}

fn item(
    ui: &mut Ui,
    state: &AppState,
    attachment: &Attachment,
    confirming: &mut Option<String>,
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let name = attachment.file_name.as_str();
    let tr = |key| i18n::translate(language, key, &[("name", name.into())]);
    let id = &attachment.id;
    let shown = state.attachments.preview_id() == Some(id.as_str());
    let edge = if shown { p.amber_edge } else { p.line };
    egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if name_button(ui, state, attachment).clicked() {
                    ask(actions, Asked::Open(id.clone()));
                }
                if is_shown(attachment) {
                    let words = tr(keys::attachments::OPEN_PREVIEW);
                    if IconButton::new(Icon::Eye, &words, "attachment-preview")
                        .pressed(shown)
                        .show(ui)
                        .clicked()
                    {
                        ask(actions, Asked::TogglePreview(id.clone()));
                    }
                }
                let words = tr(keys::attachments::SAVE);
                if IconButton::new(Icon::ArrowDown, &words, "attachment-save")
                    .show(ui)
                    .clicked()
                {
                    ask(actions, Asked::Save(id.clone()));
                }
                let armed = confirming.as_deref() == Some(id.as_str());
                let words = tr(keys::attachments::REMOVE);
                if IconButton::new(Icon::X, &words, "attachment-remove")
                    .pressed(armed)
                    .show(ui)
                    .clicked()
                {
                    if armed {
                        *confirming = None;
                        ask(actions, Asked::Remove(id.clone()));
                    } else {
                        *confirming = Some(id.clone());
                    }
                }
            });
        });
}

/// Open with the default application. Rounded up: « 0 KB » for a file is wrong.
fn name_button(ui: &mut Ui, state: &AppState, attachment: &Attachment) -> egui::Response {
    let language = state.language;
    let p = palette(ui.ctx());
    let size = attachment.byte_size.div_ceil(BYTES_PER_KB).max(1);
    let size = i18n::translate(language, keys::attachments::SIZE, &[("size", size.into())]);
    let words = format!("{} · {size}", attachment.file_name);
    ui.scope(|ui| {
        ui.set_max_width(NAME_WIDTH);
        let response = ui.add(
            egui::Label::new(RichText::new(words).size(11.0).color(p.text_1))
                .truncate()
                .sense(Sense::click()),
        );
        let label = i18n::translate(
            language,
            keys::attachments::OPEN,
            &[("name", attachment.file_name.as_str().into())],
        );
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_label(label));
        widgets::address(ui, &response, "attachment-open");
        response
    })
    .inner
}

fn preview(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let attachments = &state.attachments;
    let Some(id) = attachments.preview_id() else {
        return;
    };
    let Some(attachment) = attachments.list().iter().find(|file| file.id == id) else {
        return;
    };
    let language = state.language;
    ui.add_space(6.0);
    if let Some(texture) = attachments.preview_image() {
        let size = fitted(texture.size_vec2(), ui.available_width(), PREVIEW_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        paint(ui, rect, texture.id());
        let label = zoom_label(language, &attachment.file_name);
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_label(label));
        widgets::address(ui, &response, "attachment-zoom");
        if response.on_hover_cursor(egui::CursorIcon::ZoomIn).clicked() {
            ask(actions, Asked::Zoom(true));
        }
    }
    let close = i18n::translate(language, keys::attachments::CLOSE_PREVIEW, &[]);
    if Button::new(&close, "attachment-preview-close")
        .kind(Kind::Quiet)
        .show(ui)
        .clicked()
    {
        ask(actions, Asked::TogglePreview(id.to_owned()));
    }
}

fn zoom_label(language: Language, name: &str) -> String {
    i18n::translate(language, keys::attachments::ZOOM, &[("name", name.into())])
}

pub(in crate::notes::overlays) fn paint(ui: &Ui, rect: Rect, texture: egui::TextureId) {
    egui::Image::new((texture, rect.size()))
        .corner_radius(6)
        .paint_at(ui, rect);
}

/// As big as it is, or as fits: never enlarged.
pub(in crate::notes::overlays) fn fitted(size: Vec2, width: f32, height: f32) -> Vec2 {
    size * (width / size.x).min(height / size.y).min(1.0)
}

#[cfg(test)]
mod tests;
