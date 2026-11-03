//! The one field that holds a secret (plan § 3.8, § 3.13). egui's text editor keeps an undo
//! history of what is typed, in its memory; this field keeps nothing of its own: the text is a
//! `Zeroizing<String>` the screen owns, typed at its end, wiped when it is taken.

use egui::{
    CornerRadius, Event, FontFamily, Key, Margin, Response, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo,
};
use zeroize::Zeroizing;

use crate::theme::palette;
use crate::widgets;

/// Stars, or the letters when « Show » is pressed.
pub struct PassphraseField<'a> {
    value: &'a mut Zeroizing<String>,
    name: &'a str,
    author_id: &'a str,
    revealed: bool,
    invalid: bool,
    enabled: bool,
}

/// What the field reports beside its response.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Typed {
    pub changed: bool,
    /// Enter, the form's submit.
    pub submitted: bool,
}

impl<'a> PassphraseField<'a> {
    pub fn new(value: &'a mut Zeroizing<String>, name: &'a str, author_id: &'a str) -> Self {
        Self {
            value,
            name,
            author_id,
            revealed: false,
            invalid: false,
            enabled: true,
        }
    }

    #[must_use]
    pub fn revealed(mut self, revealed: bool) -> Self {
        self.revealed = revealed;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> (Response, Typed) {
        let p = palette(ui.ctx());
        let height = 14.0 * 1.4 + 18.0;
        let name = self.name;
        let response = widgets::interact(
            ui,
            Vec2::new(ui.available_width(), height),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
            self.author_id,
            || WidgetInfo::labeled(Role::PasswordInput, true, name),
            CornerRadius::same(8),
        );
        if response.clicked() {
            response.request_focus();
        }
        let typed = if response.has_focus() && self.enabled {
            edit(ui, self.value)
        } else {
            Typed::default()
        };

        let edge = if self.invalid { p.red } else { p.line };
        ui.painter().rect(
            response.rect,
            CornerRadius::same(8),
            p.bg_1,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        let count = self.value.chars().count();
        let shown: Zeroizing<String> = if self.revealed {
            self.value.clone()
        } else {
            Zeroizing::new("•".repeat(count))
        };
        let galley = widgets::text(ui, &shown, 14.0, 400.0, FontFamily::Proportional, p.text_0);
        let inner = response.rect - Margin::symmetric(11, 9);
        let at = egui::pos2(inner.left(), inner.center().y - galley.size().y / 2.0);
        let caret_x = at.x + galley.size().x + 1.0;
        ui.painter()
            .with_clip_rect(inner)
            .galley(at, galley, p.text_0);
        if response.has_focus() {
            ui.painter().vline(
                caret_x,
                egui::Rangef::new(inner.top() + 1.0, inner.bottom() - 1.0),
                Stroke::new(1.5, p.text_0),
            );
        }
        (response, typed)
    }
}

/// The keys of the frame, taken by the field: letters at its end, Backspace, a paste, an
/// input method's committed text; Enter submits. Everything else is left to the others.
fn edit(ui: &Ui, value: &mut Zeroizing<String>) -> Typed {
    let mut typed = Typed::default();
    ui.input_mut(|input| {
        input.events.retain(|event| {
            match event {
                Event::Text(text)
                | Event::Paste(text)
                | Event::Ime(egui::ImeEvent::Commit(text)) => {
                    value.push_str(text);
                    typed.changed = true;
                }
                Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                } => {
                    typed.changed |= value.pop().is_some();
                }
                Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                } => typed.submitted = true,
                _ => return true,
            }
            false
        });
    });
    typed
}

/// Hands the phrase over and leaves the field empty: what was typed lives on only in what the
/// core was given, which wipes it.
pub fn take(value: &mut Zeroizing<String>) -> Zeroizing<String> {
    std::mem::replace(value, Zeroizing::new(String::new()))
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::*;

    #[derive(Default)]
    struct Form {
        phrase: Zeroizing<String>,
        submitted: bool,
    }

    fn form() -> egui_kittest::Harness<'static, Form> {
        crate::testing::harness().build_ui_state(
            |ui, form: &mut Form| {
                let (_, typed) =
                    PassphraseField::new(&mut form.phrase, "Phrase", "phrase").show(ui);
                form.submitted |= typed.submitted;
            },
            Form::default(),
        )
    }

    #[test]
    fn typed_letters_go_to_the_secret_and_nowhere_else() {
        let mut harness = form();
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("phrase"))
            .click();
        harness.run();

        harness
            .get_by(|node| node.author_id() == Some("phrase"))
            .type_text("hunter2 again");
        harness.key_press(Key::Backspace);
        harness.key_press(Key::Enter);
        harness.run();

        assert_eq!(harness.state().phrase.as_str(), "hunter2 agai");
        assert!(harness.state().submitted);
        let node = harness.get_by(|node| node.author_id() == Some("phrase"));
        assert!(!format!("{:?}", node.accesskit_node()).contains("hunter2"));
        let remembered = harness.ctx.memory(|memory| format!("{:?}", memory.data));
        assert!(!remembered.contains("hunter2"), "egui keeps a copy");
    }

    #[test]
    fn taking_the_phrase_leaves_the_field_empty() {
        let mut phrase = Zeroizing::new("correct horse".to_string());

        let given = take(&mut phrase);

        assert_eq!(given.as_str(), "correct horse");
        assert!(phrase.is_empty());
    }
}
