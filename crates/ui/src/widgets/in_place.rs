//! A destructive action asked in place: its button, then, once pressed, the question beside
//! Cancel and the action in red, where the button was.

use egui::Ui;

use super::{Button, Kind, address};
use crate::theme::palette;
use crate::theme::tokens::font;

/// What was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InPlace {
    Ask,
    Cancel,
    Confirm,
}

/// Drawn in a right-to-left layout. `question` is `Some` while it is asked; `author_id` names the
/// button, and `-cancel`, `-confirm` and `-count` the rest.
pub fn ask_in_place(
    ui: &mut Ui,
    (word, cancel): (&str, &str),
    question: Option<&str>,
    enabled: bool,
    author_id: &str,
) -> Option<InPlace> {
    let Some(question) = question else {
        let asked = Button::new(word, author_id).enabled(enabled).show(ui);
        return asked.clicked().then_some(InPlace::Ask);
    };
    let mut pressed = None;
    if Button::new(cancel, &format!("{author_id}-cancel"))
        .show(ui)
        .clicked()
    {
        pressed = Some(InPlace::Cancel);
    }
    if Button::new(word, &format!("{author_id}-confirm"))
        .kind(Kind::Destructive)
        .show(ui)
        .clicked()
    {
        pressed = Some(InPlace::Confirm);
    }
    let question = egui::RichText::new(question)
        .size(font::BODY)
        .color(palette(ui.ctx()).text_1);
    let said = ui.add(egui::Label::new(question));
    address(ui, &said, &format!("{author_id}-count"));
    pressed
}
