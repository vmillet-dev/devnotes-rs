//! The answer under the request: its status coloured by family, time and size, then its body,
//! headers, cookies and the exchange as it went. While the send is under way, the time goes by;
//! a failure or a cancelled send stays on screen as an answer does, and goes the same way.

mod body;

use std::sync::Arc;

use egui::{CornerRadius, FontFamily, Margin, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use engine::http::send::{SentBody, SentResponse};
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::send::{Phase, Sent};
use crate::state::http::tabs::Tab;
use crate::theme::palette;
use crate::widgets::{self, Button, interact};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Section {
    #[default]
    Body,
    Headers,
    Cookies,
    Timeline,
}

impl Section {
    const ALL: [Self; 4] = [Self::Body, Self::Headers, Self::Cookies, Self::Timeline];

    fn id(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Headers => "headers",
            Self::Cookies => "cookies",
            Self::Timeline => "timeline",
        }
    }

    fn name(self) -> i18n::Key {
        use keys::http::response::sections;
        match self {
            Self::Body => sections::BODY,
            Self::Headers => sections::HEADERS,
            Self::Cookies => sections::COOKIES,
            Self::Timeline => sections::TIMELINE,
        }
    }
}

/// What only the pane's drawing sees: the part chosen and the body's lines.
#[derive(Default)]
pub(crate) struct ResponseView {
    section: Section,
    body: body::BodyView,
}

pub(crate) fn show(
    ui: &mut Ui,
    state: &AppState,
    (tab, view): (&Tab, &mut ResponseView),
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let pane = ui.max_rect();
    ui.painter().rect_filled(pane, CornerRadius::ZERO, p.bg_0);
    let label = i18n::translate(language, keys::http::response::LABEL, &[]);
    let region = widgets::region(ui, pane, "http-response", Sense::hover());
    region.widget_info(|| WidgetInfo::labeled(Role::Region, true, &label));
    widgets::address(ui, &region, "http-response");
    egui::Frame::new()
        .inner_margin(Margin::symmetric(20, 12))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.spacing_mut().item_spacing.y = 10.0;
            match state.http.sends().of(&tab.key) {
                None => placeholder(
                    ui,
                    keys::http::response::IDLE,
                    "http-response-idle",
                    language,
                ),
                Some(Sent {
                    phase: Phase::Sending { started },
                    ..
                }) => sending(ui, *started, language),
                Some(Sent {
                    phase: Phase::Cancelled,
                    ..
                }) => {
                    placeholder(
                        ui,
                        keys::http::response::CANCELLED,
                        "http-response-cancelled",
                        language,
                    );
                    clear(ui, tab, actions, language);
                }
                Some(Sent {
                    phase: Phase::Failed(error),
                    ..
                }) => {
                    let message = crate::services::errors::message_of(error)
                        .unwrap_or_else(|| i18n::Message::new(keys::http::send::FAILED));
                    let words = egui::RichText::new(message.text(language))
                        .size(13.0)
                        .color(p.red);
                    let said = ui.add(egui::Label::new(words).wrap());
                    widgets::address(ui, &said, "http-response-failure");
                    clear(ui, tab, actions, language);
                }
                Some(Sent {
                    id,
                    phase: Phase::Answered(response),
                }) => answered(ui, state, (tab, id, response), view, actions),
            }
        });
}

fn placeholder(ui: &mut Ui, key: i18n::Key, author_id: &str, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(i18n::translate(language, key, &[]))
        .size(13.0)
        .color(p.text_2);
    let said = ui.add(egui::Label::new(words).wrap());
    widgets::address(ui, &said, author_id);
}

/// A spinner, and the time going by: counted from the send, not from when the tab was shown.
fn sending(ui: &mut Ui, started: f64, language: i18n::Language) {
    let p = palette(ui.ctx());
    let now = ui.input(|input| input.time);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = i18n::translate(language, keys::http::response::SENDING, &[]);
        widgets::spinner(ui, 14.0, &words);
        let said = ui.add(egui::Label::new(
            egui::RichText::new(&words).size(13.0).color(p.text_2),
        ));
        widgets::address(ui, &said, "http-response-sending");
        let elapsed = format!("{:.1} s", (now - started).max(0.0));
        let ticking = ui.add(egui::Label::new(
            egui::RichText::new(elapsed)
                .monospace()
                .size(12.0)
                .color(p.text_2),
        ));
        widgets::address(ui, &ticking, "http-response-elapsed");
    });
    // The spinner asks for the frames the count needs.
}

fn clear(ui: &mut Ui, tab: &Tab, actions: &mut Actions, language: i18n::Language) {
    let words = i18n::translate(language, keys::http::response::CLEAR, &[]);
    if Button::new(&words, "http-response-clear")
        .show(ui)
        .clicked()
    {
        actions.push(Asked::Http(Action::ClearResponse(tab.key.clone())));
    }
}

fn answered(
    ui: &mut Ui,
    state: &AppState,
    (tab, id, response): (&Tab, &str, &Arc<SentResponse>),
    view: &mut ResponseView,
    actions: &mut Actions,
) {
    bar(ui, state, (tab, response), view, actions);
    match view.section {
        Section::Body => body::show(ui, state, (id, response), &mut view.body, actions),
        Section::Headers => rows(
            ui,
            "http-response-headers",
            response
                .headers
                .iter()
                .map(|row| (row.key.as_str(), row.value.as_str())),
        ),
        Section::Cookies => cookies(ui, response, state.language),
        Section::Timeline => timeline(ui, response, state.language),
    }
}

/// The status, time and size, the parts, then what can be done with the answer.
fn bar(
    ui: &mut Ui,
    state: &AppState,
    (tab, response): (&Tab, &SentResponse),
    view: &mut ResponseView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        status(ui, response);
        let millis = i18n::translate(
            language,
            keys::http::response::MILLIS,
            &[(
                "value",
                i18n::format::at_most(f64::from(response.millis), 0, language).into(),
            )],
        );
        let time = ui.add(egui::Label::new(
            egui::RichText::new(millis).size(12.5).color(p.text_1),
        ));
        widgets::address(ui, &time, "http-response-time");
        let size = ui.add(egui::Label::new(
            egui::RichText::new(size_of(response.size, language))
                .size(12.5)
                .color(p.text_1),
        ));
        widgets::address(ui, &size, "http-response-size");
        for section in Section::ALL {
            let on = view.section == section;
            let count = match section {
                Section::Headers => response.headers.len(),
                Section::Cookies => response.cookies.len(),
                Section::Body | Section::Timeline => 0,
            };
            let words = if count > 0 {
                format!("{} {count}", tr(section.name()))
            } else if section == Section::Timeline && !response.redirects.is_empty() {
                let redirects = i18n::translate(
                    language,
                    keys::http::response::REDIRECTS,
                    &[(
                        "count",
                        u32::try_from(response.redirects.len())
                            .unwrap_or(u32::MAX)
                            .into(),
                    )],
                );
                format!("{} · {redirects}", tr(section.name()))
            } else {
                tr(section.name())
            };
            let pressed = Button::new(&words, &format!("http-response-section-{}", section.id()))
                .kind(widgets::Kind::Quiet)
                .armed(on)
                .show(ui);
            ui.ctx()
                .accesskit_node_builder(pressed.id, |node| node.set_selected(on));
            if pressed.clicked() {
                view.section = section;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if Button::new(&tr(keys::http::response::CLEAR), "http-response-clear")
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Http(Action::ClearResponse(tab.key.clone())));
            }
            let has_text = !response.binary && !response.body.is_empty();
            if Button::new(&tr(keys::http::response::COPY), "http-response-copy")
                .enabled(has_text)
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Http(Action::Copy(response.body.clone())));
            }
        });
    });
}

/// `2xx` green, `3xx` blue, `4xx` amber, anything else red: what the badge's hue says.
fn status(ui: &mut Ui, response: &SentResponse) {
    let p = palette(ui.ctx());
    let hue = match response.status / 100 {
        2 => p.green,
        3 => p.blue,
        4 => p.amber_text,
        _ => p.red,
    };
    let words = format!("{} {}", response.status, response.reason)
        .trim()
        .to_owned();
    let galley = widgets::text(ui, &words, 12.5, 700.0, FontFamily::Monospace, hue);
    let size = galley.size() + Vec2::new(16.0, 8.0);
    let response = interact(
        ui,
        size,
        Sense::hover(),
        "http-response-status",
        || WidgetInfo::labeled(Role::Label, true, &words),
        CornerRadius::same(6),
    );
    ui.painter().rect_filled(
        response.rect,
        CornerRadius::same(6),
        hue.gamma_multiply(0.16),
    );
    ui.painter()
        .galley(response.rect.center() - galley.size() / 2.0, galley, hue);
}

/// Bytes, kilobytes or megabytes, as the reference wrote them.
fn size_of(bytes: u32, language: i18n::Language) -> String {
    let bytes = f64::from(bytes);
    let (key, value, digits) = if bytes < 1024.0 {
        (keys::http::response::BYTES, bytes, 0)
    } else if bytes < 1024.0 * 1024.0 {
        (keys::http::response::KILOBYTES, bytes / 1024.0, 1)
    } else {
        (keys::http::response::MEGABYTES, bytes / 1024.0 / 1024.0, 1)
    };
    let value = i18n::format::at_most(value, digits, language);
    i18n::translate(language, key, &[("value", value.into())])
}

/// A name and its value a line: the headers.
fn rows<'a>(ui: &mut Ui, author_id: &str, rows: impl Iterator<Item = (&'a str, &'a str)>) {
    let p = palette(ui.ctx());
    let shown = egui::ScrollArea::vertical()
        .id_salt(author_id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new(author_id)
                .num_columns(2)
                .spacing(Vec2::new(16.0, 6.0))
                .show(ui, |ui| {
                    for (key, value) in rows {
                        ui.label(
                            egui::RichText::new(key)
                                .monospace()
                                .size(12.5)
                                .color(p.text_1),
                        );
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(value)
                                    .monospace()
                                    .size(12.5)
                                    .color(p.text_0),
                            )
                            .wrap(),
                        );
                        ui.end_row();
                    }
                });
        });
    let region = widgets::region(ui, shown.inner_rect, author_id, Sense::hover());
    region.widget_info(|| WidgetInfo::labeled(Role::Table, true, author_id));
    widgets::address(ui, &region, author_id);
}

fn cookies(ui: &mut Ui, response: &SentResponse, language: i18n::Language) {
    let p = palette(ui.ctx());
    if response.cookies.is_empty() {
        placeholder(
            ui,
            keys::http::response::NO_COOKIES,
            "http-response-no-cookies",
            language,
        );
        return;
    }
    let tr = |key| i18n::translate(language, key, &[]);
    egui::Grid::new("http-response-cookies")
        .num_columns(4)
        .spacing(Vec2::new(16.0, 6.0))
        .show(ui, |ui| {
            for key in [
                keys::http::response::cookie::NAME,
                keys::http::response::cookie::VALUE,
                keys::http::response::cookie::DOMAIN,
                keys::http::response::cookie::PATH,
            ] {
                ui.label(
                    egui::RichText::new(tr(key))
                        .size(11.5)
                        .strong()
                        .color(p.text_2),
                );
            }
            ui.end_row();
            for each in &response.cookies {
                let name = ui.label(
                    egui::RichText::new(&each.name)
                        .monospace()
                        .size(12.5)
                        .color(p.text_0),
                );
                widgets::address(ui, &name, "http-response-cookie");
                ui.label(
                    egui::RichText::new(&each.value)
                        .monospace()
                        .size(12.5)
                        .color(p.text_1),
                );
                ui.label(
                    egui::RichText::new(each.domain.as_deref().unwrap_or_default())
                        .size(12.0)
                        .color(p.text_1),
                );
                ui.label(
                    egui::RichText::new(each.path.as_deref().unwrap_or_default())
                        .size(12.0)
                        .color(p.text_1),
                );
                ui.end_row();
            }
        });
}

/// The redirects followed, then the exchange as raw text: what left and what came back.
fn timeline(ui: &mut Ui, response: &SentResponse, language: i18n::Language) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    egui::ScrollArea::vertical()
        .id_salt("http-response-timeline")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let heading = |ui: &mut Ui, key| {
                ui.label(
                    egui::RichText::new(tr(key))
                        .size(12.0)
                        .strong()
                        .color(p.text_2),
                );
            };
            if !response.redirects.is_empty() {
                heading(ui, keys::http::response::timeline::REDIRECTS);
                for redirect in &response.redirects {
                    let line = format!("{} -> {}", redirect.status, redirect.url);
                    let shown = ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .size(12.5)
                            .color(p.text_0),
                    );
                    widgets::address(ui, &shown, "http-response-redirect");
                }
            }
            heading(ui, keys::http::response::timeline::REQUEST);
            let exchange = &response.exchange;
            let mut head = vec![format!("{} {}", exchange.method, exchange.url)];
            head.extend(
                exchange
                    .headers
                    .iter()
                    .map(|row| format!("{}: {}", row.key, row.value)),
            );
            exchange_text(ui, &head.join("\n"), "http-response-sent");
            match &exchange.body {
                SentBody::None => {}
                SentBody::Text { text } => exchange_text(ui, text, "http-response-sent-body"),
                SentBody::Bytes { size } => {
                    let words = i18n::translate(
                        language,
                        keys::http::response::timeline::BYTES,
                        &[("size", (*size).into())],
                    );
                    ui.label(egui::RichText::new(words).size(12.0).color(p.text_2));
                }
                SentBody::Multipart { fields } => {
                    let words = i18n::translate(
                        language,
                        keys::http::response::timeline::MULTIPART,
                        &[("fields", fields.join(", ").into())],
                    );
                    ui.label(egui::RichText::new(words).size(12.0).color(p.text_2));
                }
            }
            heading(ui, keys::http::response::timeline::RESPONSE);
            let mut received = vec![
                format!("{} {}", response.status, response.reason)
                    .trim()
                    .to_owned(),
            ];
            received.extend(
                response
                    .headers
                    .iter()
                    .map(|row| format!("{}: {}", row.key, row.value)),
            );
            exchange_text(ui, &received.join("\n"), "http-response-received");
        });
}

fn exchange_text(ui: &mut Ui, text: &str, author_id: &str) {
    let p = palette(ui.ctx());
    let shown = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .monospace()
                        .size(12.0)
                        .color(p.text_0),
                )
                .wrap(),
            )
        })
        .inner;
    widgets::address(ui, &shown, author_id);
}

/// The answer's type, as its `Content-Type` says it.
fn content_type(response: &SentResponse) -> Option<&str> {
    response
        .headers
        .iter()
        .find(|row| row.key.eq_ignore_ascii_case("content-type"))
        .and_then(|row| row.value.split(';').next())
        .map(str::trim)
}

/// The line between a request and its answer: dragged, it shares the height between them; a
/// double click or Enter leaves it to what the request holds again.
pub(crate) fn split_handle(ui: &mut Ui, rect: Rect, language: i18n::Language) -> egui::Response {
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::http::split::LABEL, &[]);
    let mut over = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let response = interact(
        &mut over,
        rect.size(),
        Sense::click_and_drag(),
        "http-split-handle",
        || WidgetInfo::labeled(Role::Splitter, true, &label),
        CornerRadius::ZERO,
    );
    let lit = response.hovered() || response.dragged() || response.has_focus();
    let ink = if lit { p.amber_dim } else { p.line_soft };
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, ink));
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    response
}
