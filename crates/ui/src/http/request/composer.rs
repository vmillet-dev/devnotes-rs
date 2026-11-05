//! The message to send on an open socket, and those kept to send again.

use egui::{Key, Modifiers, Ui};
use engine::http::websocket::{SavedMessage, WebsocketDocument};
use i18n::keys;

use super::body::{TextBox, text_box_of};
use super::{Asked, TextBoxes, edit};
use crate::action::Actions;
use crate::http::Action;
use crate::state::AppState;
use crate::state::http::tabs::{DraftEdit, Tab};
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, Button, IconButton, Kind, TextField};

/// Eight lines, as the reference's.
const MESSAGE_HEIGHT: f32 = 160.0;

/// A kept message is named after its first line, cut: the name is a label, not a field.
const NAME_LENGTH: usize = 40;

fn send(actions: &mut Actions, tab: &Tab, text: String) {
    actions.push(Asked::Http(Action::SendMessage {
        key: tab.key.clone(),
        text,
    }));
}

fn keep(actions: &mut Actions, tab: &Tab, messages: Vec<SavedMessage>) {
    let websocket = WebsocketDocument {
        messages,
        ..tab.draft.document.websocket.clone()
    };
    edit(actions, tab, DraftEdit::Websocket(websocket));
}

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    tab: &Tab,
    boxes: &mut TextBoxes,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let open = state.http.sockets().is_open(&tab.key);
    let (label, hint) = (
        tr(keys::http::socket::MESSAGE),
        tr(keys::http::socket::MESSAGE_PLACEHOLDER),
    );
    let field = TextBox {
        label: &label,
        author_id: "http-socket-message",
        hint: &hint,
        height: MESSAGE_HEIGHT,
    };
    let text = boxes.message.text();
    let (_, typing) = text_box_of(ui, &mut boxes.message, &text, &field, language);
    let text = boxes.message.text();
    let blank = text.trim().is_empty();
    // ⚠️ Taken here: the area answers the same key by connecting, which is not what was meant.
    if typing.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter))
    {
        boxes.refused = !open;
        if open && !blank {
            send(actions, tab, text.clone());
        }
    }
    if boxes.refused && !open {
        let words = egui::RichText::new(tr(keys::http::socket::NOT_OPEN))
            .size(12.0)
            .color(p.amber_text);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-socket-not-open");
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::socket::SEND), "http-socket-send")
            .kind(Kind::Filled)
            .enabled(open && !blank)
            .show(ui)
            .clicked()
        {
            send(actions, tab, text.clone());
        }
        if Button::new(&tr(keys::http::socket::KEEP), "http-socket-keep")
            .enabled(!blank)
            .show(ui)
            .clicked()
        {
            let line = text.trim().lines().next().unwrap_or_default();
            let name: String = line.chars().take(NAME_LENGTH).collect();
            let mut messages = tab.draft.document.websocket.messages.clone();
            messages.push(SavedMessage {
                name,
                text: text.clone(),
            });
            keep(actions, tab, messages);
        }
    });
    saved(ui, tab, boxes, (open, language), actions);
}

fn saved(
    ui: &mut Ui,
    tab: &Tab,
    boxes: &mut TextBoxes,
    (open, language): (bool, i18n::Language),
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let words = egui::RichText::new(tr(keys::http::socket::SAVED))
        .size(12.0)
        .strong()
        .color(p.text_2);
    ui.add(egui::Label::new(words));
    let messages = &tab.draft.document.websocket.messages;
    if messages.is_empty() {
        let words = egui::RichText::new(tr(keys::http::socket::NONE_SAVED))
            .size(12.0)
            .color(p.text_2);
        ui.add(egui::Label::new(words));
    }
    for (at, message) in messages.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if Button::new(&message.name, "http-socket-saved")
                .show(ui)
                .clicked()
            {
                boxes.message.replace_all(&message.text);
            }
            if Button::new(&tr(keys::http::socket::SEND), "http-socket-saved-send")
                .enabled(open)
                .show(ui)
                .clicked()
            {
                send(actions, tab, message.text.clone());
            }
            let label = i18n::translate(
                language,
                keys::http::socket::REMOVE,
                &[("name", message.name.as_str().into())],
            );
            if IconButton::new(Icon::X, &label, "http-socket-saved-remove")
                .show(ui)
                .clicked()
            {
                let mut rest = messages.clone();
                rest.remove(at);
                keep(actions, tab, rest);
            }
        });
    }
}

/// The subprotocols asked for, written on leaving the field.
pub(super) fn protocols(
    ui: &mut Ui,
    tab: &Tab,
    typed: &mut Option<String>,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let document = &tab.draft.document.websocket;
    let mut text = typed
        .clone()
        .unwrap_or_else(|| document.protocols.join(", "));
    let label = tr(keys::http::socket::PROTOCOLS);
    let words = egui::RichText::new(&label).size(12.5).color(p.text_1);
    ui.add(egui::Label::new(words));
    let field = TextField::new(&mut text, &label, "http-socket-protocols")
        .hint("chat, json")
        .show(ui);
    if field.changed() {
        *typed = Some(text.clone());
    }
    if field.lost_focus() && typed.take().is_some() {
        let protocols = text
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        let websocket = WebsocketDocument {
            protocols,
            ..document.clone()
        };
        edit(actions, tab, DraftEdit::Websocket(websocket));
    }
    let hint = egui::RichText::new(tr(keys::http::socket::PROTOCOLS_HINT))
        .size(12.0)
        .color(p.text_2);
    ui.add(egui::Label::new(hint));
}
