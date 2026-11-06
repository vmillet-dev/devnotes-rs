//! What the rail opens in place: a name typed under a row or over its name, and the question
//! before a deletion. A field left for anything else keeps what was typed, as a file explorer
//! does; left empty, it closes; Escape cancels.

use egui::{CornerRadius, Margin, Stroke, Ui};
use engine::http::model::{HttpContents, HttpItem};
use i18n::keys;

use super::{Action, push};
use crate::action::Actions;
use crate::state::http::tree::RailRow;
use crate::theme::palette;
use crate::widgets::{Button, Kind, TextField};

pub(super) struct Draft {
    name: String,
    focus: bool,
}

impl Default for Draft {
    fn default() -> Self {
        Self::named("")
    }
}

impl Draft {
    pub(super) fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            focus: true,
        }
    }
}

/// What a name's form says: the field's placeholder, which is its name too, and its button.
pub(super) struct Words {
    pub(super) hint: String,
    pub(super) submit: String,
}

pub(super) enum Typed {
    Typing,
    Name(String),
    /// Escape, or the field left empty: nothing is written.
    Dropped,
}

/// The field, then its button under it: the rail is too narrow for both on one line.
pub(super) fn name_form(ui: &mut Ui, draft: &mut Draft, words: &Words, ids: (&str, &str)) -> Typed {
    let field = field(ui, draft, words, ids.0);
    let clicked = Button::new(&words.submit, ids.1).show(ui).clicked();
    settle(ui, draft, &field, clicked)
}

/// The field and its button on the row itself, in place of its name.
pub(super) fn inline_form(
    ui: &mut Ui,
    draft: &mut Draft,
    words: &Words,
    ids: (&str, &str),
) -> Typed {
    let button = Button::width(ui, &words.submit);
    ui.spacing_mut().item_spacing.x = 4.0;
    let room = ui.available_width() - button - 4.0;
    let field = ui
        .allocate_ui(egui::vec2(room.max(48.0), ui.available_height()), |ui| {
            field(ui, draft, words, ids.0)
        })
        .inner;
    let clicked = Button::new(&words.submit, ids.1).show(ui).clicked();
    settle(ui, draft, &field, clicked)
}

fn field(ui: &mut Ui, draft: &mut Draft, words: &Words, id: &str) -> egui::Response {
    let field = TextField::new(&mut draft.name, &words.hint, id)
        .hint(&words.hint)
        .compact()
        .show(ui);
    if std::mem::take(&mut draft.focus) {
        field.request_focus();
    }
    field
}

/// ⚠️ Enter, the button and the field left can each arrive for one name: the first one settles
/// the form, which the caller then closes, and the others find nothing left to write.
fn settle(ui: &Ui, draft: &Draft, field: &egui::Response, clicked: bool) -> Typed {
    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        return Typed::Dropped;
    }
    if !(clicked || field.lost_focus()) {
        return Typed::Typing;
    }
    let name = draft.name.trim();
    if name.is_empty() {
        Typed::Dropped
    } else {
        Typed::Name(name.to_owned())
    }
}

/// « Delete “Auth”? », what goes with it counted by Rust, and its two buttons — red at rest.
pub(super) fn confirm_delete(
    ui: &mut Ui,
    (row, contents, unsaved): (&RailRow, HttpContents, &[String]),
    actions: &mut Actions,
    language: i18n::Language,
) {
    let p = palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .outer_margin(Margin {
            left: 4,
            right: 4,
            top: 4,
            bottom: 6,
        });
    let response = frame
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let question = i18n::translate(
                language,
                keys::http::rail::CONFIRM,
                &[("name", row.name.as_str().into())],
            );
            let label = egui::RichText::new(&question).size(12.0).color(p.text_0);
            ui.add(egui::Label::new(label).wrap());
            if contents.folders + contents.requests > 0 {
                let count = egui::RichText::new(noun(contents, language))
                    .size(12.0)
                    .color(p.text_2);
                let response = ui.add(egui::Label::new(count).wrap());
                crate::widgets::address(ui, &response, "http-delete-count");
            }
            if !unsaved.is_empty() {
                lost(ui, row, unsaved, language);
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let delete = i18n::translate(language, keys::http::rail::actions::DELETE, &[]);
                if Button::new(&delete, "http-delete-submit")
                    .kind(Kind::Destructive)
                    .show(ui)
                    .clicked()
                {
                    let item = HttpItem {
                        kind: row.kind,
                        id: row.id.clone(),
                    };
                    push(
                        actions,
                        Action::Write(crate::state::http::Write::Delete(item)),
                    );
                }
                let cancel = i18n::translate(language, keys::http::rail::CANCEL, &[]);
                if Button::new(&cancel, "http-delete-cancel")
                    .show(ui)
                    .clicked()
                {
                    push(actions, Action::CancelDelete);
                }
            });
        })
        .response;
    let question = i18n::translate(
        language,
        keys::http::rail::CONFIRM,
        &[("name", row.name.as_str().into())],
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::AlertDialog, true, &question));
    crate::widgets::address(ui, &response, "http-delete-confirm");
}

/// What Rust cannot count: a tab the deletion closes takes what was typed in it. A note in the
/// same question, not a second one; red stays the button's.
fn lost(ui: &mut Ui, row: &RailRow, unsaved: &[String], language: i18n::Language) {
    let p = palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.amber_text.gamma_multiply(0.08))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 6));
    let response = frame
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let caption = if row.kind == engine::http::model::HttpItemKind::Request {
                keys::http::rail::unsaved::SELF
            } else {
                keys::http::rail::unsaved::INSIDE
            };
            let words = egui::RichText::new(i18n::translate(language, caption, &[]))
                .size(11.5)
                .color(p.text_1);
            ui.add(egui::Label::new(words).wrap());
            if row.kind == engine::http::model::HttpItemKind::Request {
                return;
            }
            for name in unsaved {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    // The dot a modified tab wears, drawn: no embedded face has it.
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(6.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.0, p.amber_text);
                    let line = egui::RichText::new(name).size(11.5).color(p.text_0);
                    let said = ui.add(egui::Label::new(line).truncate());
                    crate::widgets::address(ui, &said, "http-delete-unsaved-name");
                });
            }
        })
        .response;
    crate::widgets::address(ui, &response, "http-delete-unsaved");
}

/// Says only what is there: « no folder » is noise in a sentence about loss.
fn noun(contents: HttpContents, language: i18n::Language) -> String {
    use keys::http::rail::takes;
    let key = if contents.folders == 0 {
        takes::REQUESTS
    } else if contents.requests == 0 {
        takes::FOLDERS
    } else {
        takes::BOTH
    };
    i18n::translate(
        language,
        key,
        &[
            ("folders", contents.folders.into()),
            ("requests", contents.requests.into()),
        ],
    )
}
