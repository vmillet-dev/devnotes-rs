//! An image at its size, over everything (the reference's `image-lightbox`). It reads nothing:
//! the picture is the one the strip already holds.

use egui::{Id, Margin, RichText, Sense, Ui, Vec2};
use i18n::keys;

use crate::action::{Action, Actions};
use crate::notes::overlays::editor::{fitted, paint};
use crate::notes::{Action as Asked, Editing};
use crate::state::AppState;
use crate::state::attachments::Asked as Attachment;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, IconButton};

/// How much of the window it may take.
const ROOM: f32 = 0.9;

pub(in crate::notes) fn show(ui: &Ui, state: &AppState, actions: &mut Actions) {
    let Some((name, texture)) = state.attachments.zoomed() else {
        return;
    };
    let p = palette(ui.ctx());
    let room = ui.ctx().content_rect().size() * ROOM - Vec2::new(0.0, 40.0);
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .corner_radius(10)
        .inner_margin(Margin::same(10));
    let language = state.language;
    let shown = egui::Modal::new(Id::unique("image-lightbox"))
        .backdrop_color(p.backdrop)
        .frame(frame)
        .show(ui.ctx(), |ui| {
            let size = fitted(texture.size_vec2(), room.x, room.y);
            ui.set_width(size.x.max(240.0));
            let closed = ui
                .horizontal(|ui| {
                    ui.label(RichText::new(name).size(12.0).color(p.text_1));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close = i18n::translate(language, keys::attachments::CLOSE_ZOOM, &[]);
                        IconButton::new(Icon::X, &close, "lightbox-close")
                            .show(ui)
                            .clicked()
                    })
                    .inner
                })
                .inner;
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            paint(ui, rect, texture.id());
            let label = i18n::translate(
                language,
                keys::attachments::PREVIEW,
                &[("name", name.into())],
            );
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(label));
            widgets::address(ui, &response, "lightbox-image");
            closed
        });
    if shown.inner || shown.should_close() {
        let closed = Attachment::Zoom(false);
        actions.push(Action::Notes(Asked::Editor(Editing::Attachment(closed))));
    }
}
