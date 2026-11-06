//! The passphrase a file asks for: when an export is sealed with one of its own (« protect »,
//! the floor applies), or when a sealed file is imported (« unlock », any length). A modal on the
//! highest rung, up while the key is derived.

use egui::{Id, RichText, Ui};
use i18n::{Key, Language, Message, keys};
use zeroize::Zeroizing;

use super::passphrase::{self, PassphraseField};
use crate::theme::palette;
use crate::widgets::{self, Button, DialogStack, Kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Protect,
    Unlock,
}

/// What the prompt was opened for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub purpose: Purpose,
    pub file_name: String,
    /// The phrase given last did not open the file.
    pub refused: bool,
    /// Answered, and the file is being read or written: nothing more can be asked of it.
    pub working: bool,
}

#[derive(Default)]
pub struct PromptView {
    pub passphrase: Zeroizing<String>,
    pub confirmation: Zeroizing<String>,
}

/// What the person answered.
pub enum Answer {
    Phrase(Zeroizing<String>),
    /// « Export in clear », offered only to protect.
    Plain,
    Cancel,
}

pub fn id() -> Id {
    Id::unique("passphrase-prompt")
}

pub fn can_submit(request: &Request, view: &PromptView) -> bool {
    match request.purpose {
        Purpose::Unlock => !view.passphrase.is_empty(),
        Purpose::Protect => {
            view.passphrase.chars().count() >= engine::vault::MINIMUM_LENGTH as usize
                && *view.confirmation == *view.passphrase
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    stack: &DialogStack,
    request: &Request,
    view: &mut PromptView,
    language: Language,
) -> Option<Answer> {
    let shown = widgets::dialog(ctx, stack, id(), |ui| body(ui, request, view, language))?;
    let dismissed = shown.should_close() && !request.working;
    shown.inner.or(dismissed.then_some(Answer::Cancel))
}

fn body(
    ui: &mut Ui,
    request: &Request,
    view: &mut PromptView,
    language: Language,
) -> Option<Answer> {
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let protecting = request.purpose == Purpose::Protect;
    let (title, lead) = if protecting {
        (keys::file::PROTECT_TITLE, keys::file::PROTECT_LEAD)
    } else {
        (keys::file::UNLOCK_TITLE, keys::file::UNLOCK_LEAD)
    };
    ui.spacing_mut().item_spacing.y = 12.0;
    let heading = ui.label(
        RichText::new(tr(title))
            .size(14.0)
            .variation(*b"wght", 600.0)
            .color(p.text_0),
    );
    widgets::address(ui, &heading, "passphrase-prompt");
    let said = Message::new(lead)
        .with("path", request.file_name.as_str())
        .text(language);
    ui.label(RichText::new(said).size(12.0).color(p.text_2));
    if protecting {
        ui.label(
            RichText::new(tr(keys::file::PROTECT_WARNING))
                .size(12.0)
                .color(p.amber_text),
        );
    }
    let name = tr(keys::vault::PASSPHRASE);
    ui.label(RichText::new(&name).size(11.2).color(p.text_2));
    let (field, typed) =
        PassphraseField::new(&mut view.passphrase, &name, "passphrase-prompt-field")
            .invalid(request.refused)
            .enabled(!request.working)
            .show(ui);
    if !request.working && ui.memory(|memory| memory.focused().is_none()) {
        field.request_focus();
    }
    let mut submitted = typed.submitted;
    if protecting {
        let confirmation = tr(keys::vault::CONFIRMATION);
        ui.label(RichText::new(&confirmation).size(11.2).color(p.text_2));
        submitted |= PassphraseField::new(
            &mut view.confirmation,
            &confirmation,
            "passphrase-prompt-confirmation",
        )
        .enabled(!request.working)
        .show(ui)
        .1
        .submitted;
    }
    let problem = if request.refused {
        tr(keys::errors::WRONG_PASSPHRASE)
    } else if protecting && !view.confirmation.is_empty() && *view.confirmation != *view.passphrase
    {
        tr(keys::vault::MISMATCHED)
    } else {
        String::new()
    };
    let shown = ui.label(RichText::new(problem).size(11.9).color(p.red));
    widgets::address(ui, &shown, "passphrase-prompt-problem");

    let mut answer = None;
    ui.horizontal(|ui| {
        if Button::new(&tr(keys::file::CANCEL), "passphrase-prompt-cancel")
            .enabled(!request.working)
            .show(ui)
            .clicked()
        {
            answer = Some(Answer::Cancel);
        }
        if protecting
            && Button::new(&tr(keys::file::PROTECT_PLAIN), "passphrase-prompt-plain")
                .enabled(!request.working)
                .show(ui)
                .clicked()
        {
            answer = Some(Answer::Plain);
        }
        let action = if request.working {
            keys::file::WORKING
        } else if protecting {
            keys::file::PROTECT_ACTION
        } else {
            keys::file::UNLOCK_ACTION
        };
        let ready = !request.working && can_submit(request, view);
        let submit = Button::new(&tr(action), "passphrase-prompt-submit")
            .kind(Kind::Filled)
            .enabled(ready)
            .show(ui);
        if ready && (submit.clicked() || submitted) {
            view.confirmation = Zeroizing::new(String::new());
            answer = Some(Answer::Phrase(passphrase::take(&mut view.passphrase)));
        }
    });
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(purpose: Purpose) -> Request {
        Request {
            purpose,
            file_name: "notes.devnotes".to_string(),
            refused: false,
            working: false,
        }
    }

    fn view(passphrase: &str, confirmation: &str) -> PromptView {
        PromptView {
            passphrase: Zeroizing::new(passphrase.to_string()),
            confirmation: Zeroizing::new(confirmation.to_string()),
        }
    }

    #[test]
    fn protecting_a_file_asks_twelve_characters_twice() {
        let protect = request(Purpose::Protect);

        assert!(!can_submit(&protect, &view("short", "short")));
        assert!(!can_submit(
            &protect,
            &view("long enough phrase", "long enough frase")
        ));
        assert!(can_submit(
            &protect,
            &view("long enough phrase", "long enough phrase")
        ));
    }

    #[test]
    fn unlocking_a_file_takes_any_phrase() {
        assert!(can_submit(&request(Purpose::Unlock), &view("short", "")));
        assert!(!can_submit(&request(Purpose::Unlock), &view("", "")));
    }

    #[test]
    fn the_prompt_draws_over_everything_and_answers_its_phrase() {
        use egui_kittest::kittest::Queryable;

        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui_state(
                |ui, (stack, view, answered): &mut (DialogStack, PromptView, Option<String>)| {
                    stack.open(widgets::Layer::Passphrase, id(), None);
                    if let Some(Answer::Phrase(phrase)) = show(
                        ui.ctx(),
                        stack,
                        &request(Purpose::Unlock),
                        view,
                        Language::En,
                    ) {
                        *answered = Some(phrase.to_string());
                    }
                },
                (DialogStack::default(), PromptView::default(), None),
            );
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("passphrase-prompt-field"))
            .click();
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("passphrase-prompt-field"))
            .type_text("opens it");
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("passphrase-prompt-submit"))
            .click();
        harness.run();

        assert_eq!(harness.state().2.as_deref(), Some("opens it"));
        assert!(harness.state().1.passphrase.is_empty());
    }
}
