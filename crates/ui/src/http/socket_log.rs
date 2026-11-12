//! The lower half of a WebSocket's tab: where the socket stands, and what went each way, in rows
//! drawn only in view. A message reads on one line, whole in its tooltip.

use chrono::Local;
use egui::{CornerRadius, FontFamily, Margin, Rect, Role, Sense, Ui, Vec2, WidgetInfo, pos2};
use engine::error::AppError;
use engine::http::websocket::SocketEvent;
use i18n::keys;

use super::Action;
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::sockets::{Phase, SocketEntry};
use crate::state::http::tabs::{DraftEdit, Tab};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, TextField, virtual_list};

const ROW: f32 = 26.0;

/// What only the log's drawing sees.
#[derive(Default)]
pub(crate) struct LogView {
    filter: String,
}

/// The words a row reads, and the filter looks in.
fn text_of(entry: &SocketEntry) -> &str {
    match &entry.event {
        SocketEvent::Sent { text, .. } | SocketEvent::Received { text, .. } => text,
        SocketEvent::Opened { url, .. } => url,
        SocketEvent::Closed { reason, .. } => reason,
        SocketEvent::Failed { detail, .. } => detail,
    }
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    (tab, view): (&Tab, &mut LogView),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let pane = ui.max_rect();
    ui.painter().rect_filled(pane, CornerRadius::ZERO, p.bg_0);
    let label = i18n::translate(state.language, keys::http::socket::LOG, &[]);
    let region = widgets::region(ui, pane, "http-socket-log", Sense::hover());
    region.widget_info(|| WidgetInfo::labeled(Role::Region, true, &label));
    widgets::address(ui, &region, "http-socket-log");
    egui::Frame::new()
        .inner_margin(Margin::symmetric(20, 12))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.spacing_mut().item_spacing.y = 10.0;
            bar(ui, state, (tab, view), actions);
            if let Some(notice) = state
                .http
                .sockets()
                .of(&tab.key)
                .and_then(|it| it.notice.as_ref())
            {
                failure(ui, tab, notice, state.language, actions);
            }
            entries(ui, state, tab, view);
        });
}

fn bar(ui: &mut Ui, state: &AppState, (tab, view): (&Tab, &mut LogView), actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let socket = state.http.sockets().of(&tab.key);
    let phase = socket.map_or(Phase::Closed, |socket| socket.phase);
    let (key, hue) = match phase {
        Phase::Connecting => (keys::http::socket::phases::CONNECTING, p.amber_text),
        Phase::Open => (keys::http::socket::phases::OPEN, p.green),
        Phase::Closing => (keys::http::socket::phases::CLOSING, p.amber_text),
        Phase::Closed => (keys::http::socket::phases::CLOSED, p.red),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        if phase == Phase::Connecting {
            widgets::spinner(ui, 14.0, &tr(key));
        }
        let words = egui::RichText::new(tr(key))
            .size(12.0)
            .strong()
            .color(p.text_1);
        let said = egui::Frame::new()
            .fill(hue.gamma_multiply(0.15))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 3))
            .show(ui, |ui| ui.add(egui::Label::new(words)))
            .inner;
        widgets::address(ui, &said, "http-socket-phase");
        if let Some(protocol) = socket.and_then(|socket| socket.protocol.as_deref()) {
            let words = i18n::translate(
                language,
                keys::http::socket::PROTOCOL,
                &[("protocol", protocol.into())],
            );
            let words = egui::RichText::new(words).size(12.0).color(p.text_2);
            ui.add(egui::Label::new(words));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let logged = socket.is_some_and(|socket| !socket.log.is_empty());
            if Button::new(&tr(keys::http::socket::CLEAR), "http-socket-clear")
                .enabled(logged)
                .show(ui)
                .clicked()
            {
                actions.push(Asked::Http(Action::ClearSocketLog(tab.key.clone())));
            }
            let size = Vec2::new(220.0, ui.available_height());
            let layout = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(size, layout, |ui| {
                ui.set_width(220.0);
                let label = tr(keys::http::socket::FILTER);
                TextField::new(&mut view.filter, &label, "http-socket-filter")
                    .hint(&label)
                    .compact()
                    .show(ui);
            });
        });
    });
}

/// « Network error » alone tells nothing: the cause's own words go with it.
fn failure(
    ui: &mut Ui,
    tab: &Tab,
    notice: &AppError,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let message = crate::services::errors::message_of(notice)
        .unwrap_or_else(|| i18n::Message::new(keys::http::socket::FAILED));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let words = egui::RichText::new(message.text(language))
            .size(12.5)
            .color(p.red);
        let said = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &said, "http-socket-failure");
        if let Some(cause) = notice.params.get("cause") {
            let words = egui::RichText::new(cause).size(12.0).color(p.text_2);
            let said = ui.add(egui::Label::new(words));
            widgets::address(ui, &said, "http-socket-cause");
        }
        if let Some(twin) = notice.params.get("twin") {
            let words = i18n::translate(
                language,
                keys::http::socket::USE_TWIN,
                &[("url", twin.as_str().into())],
            );
            if Button::new(&words, "http-socket-twin").show(ui).clicked() {
                actions.push(Asked::Http(Action::Edit {
                    key: tab.key.clone(),
                    edit: DraftEdit::Url(twin.clone()),
                }));
            }
        }
    });
}

fn entries(ui: &mut Ui, state: &AppState, tab: &Tab, view: &LogView) {
    let p = palette(ui.ctx());
    let language = state.language;
    let wanted = view.filter.trim().to_lowercase();
    let shown: Vec<&SocketEntry> = state
        .http
        .sockets()
        .of(&tab.key)
        .into_iter()
        .flat_map(|socket| socket.log.iter())
        .filter(|entry| wanted.is_empty() || text_of(entry).to_lowercase().contains(&wanted))
        .collect();
    if shown.is_empty() {
        let words = i18n::translate(language, keys::http::socket::EMPTY, &[]);
        let words = egui::RichText::new(words).size(12.5).color(p.text_2);
        let said = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &said, "http-socket-empty");
        return;
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    virtual_list(ui, "http-socket-entries", ROW, shown.len(), |ui, at| {
        row(ui, shown[at], language);
    });
}

/// What a row says of its event.
fn words_of(entry: &SocketEntry, language: i18n::Language) -> String {
    let tr = |key| i18n::translate(language, key, &[]);
    match &entry.event {
        SocketEvent::Opened { url, .. } => i18n::translate(
            language,
            keys::http::socket::OPENED,
            &[("url", url.as_str().into())],
        ),
        SocketEvent::Closed { code, reason } => {
            let code = code.map_or_else(String::new, |code| code.to_string());
            format!("{} {code} {reason}", tr(keys::http::socket::CLOSED))
                .trim_end()
                .to_owned()
        }
        SocketEvent::Failed { detail, .. } => detail.clone(),
        SocketEvent::Received { binary: true, .. } => tr(keys::http::socket::BINARY),
        SocketEvent::Sent { text, .. } | SocketEvent::Received { text, .. } => text.clone(),
    }
}

fn size_of(entry: &SocketEntry, language: i18n::Language) -> Option<String> {
    match entry.event {
        SocketEvent::Sent { size, .. } | SocketEvent::Received { size, .. } => {
            Some(i18n::translate(
                language,
                keys::http::response::BYTES,
                &[("value", size.into())],
            ))
        }
        _ => None,
    }
}

fn row(ui: &mut Ui, entry: &SocketEntry, language: i18n::Language) {
    let p = palette(ui.ctx());
    let size = Vec2::new(ui.available_width(), ROW);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let words = words_of(entry, language);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "http-socket-entry");
    let middle = rect.center().y;
    let time = entry
        .at
        .with_timezone(&Local)
        .format("%H:%M:%S%.3f")
        .to_string();
    let galley = widgets::text(ui, &time, 11.0, 400.0, FontFamily::Monospace, p.text_2);
    let width = galley.size().x;
    ui.painter().galley(
        pos2(rect.left(), middle - galley.size().y / 2.0),
        galley,
        p.text_2,
    );
    let glyph = Rect::from_center_size(pos2(rect.left() + width + 16.0, middle), Vec2::splat(12.0));
    match entry.event {
        SocketEvent::Sent { .. } => icons::paint(ui, glyph, Icon::ArrowUp, p.amber_text),
        SocketEvent::Received { .. } => icons::paint(ui, glyph, Icon::ArrowDown, p.green),
        _ => {
            ui.painter().circle_filled(glyph.center(), 2.5, p.text_2);
        }
    }
    let size_words = size_of(entry, language);
    let right = rect.right()
        - size_words.as_ref().map_or(0.0, |words| {
            let galley = widgets::text(ui, words, 11.0, 400.0, FontFamily::Proportional, p.text_2);
            let width = galley.size().x;
            let at = pos2(rect.right() - width, middle - galley.size().y / 2.0);
            ui.painter().galley(at, galley, p.text_2);
            width + 12.0
        });
    let left = glyph.right() + 8.0;
    let line = words.replace(['\n', '\r'], " ");
    let ink = if matches!(entry.event, SocketEvent::Failed { .. }) {
        p.red
    } else {
        p.text_0
    };
    let galley = widgets::text(ui, &line, 12.0, 400.0, FontFamily::Monospace, ink);
    let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
    let at = pos2(left, middle - galley.size().y / 2.0);
    ui.painter_at(clip).galley(at, galley, ink);
    widgets::tooltip(response, &words);
}
