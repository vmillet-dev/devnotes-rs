//! Go to a line of the code: Ctrl+Alt+G opens a field over it, Enter or the button goes there and
//! gives the keys back to the code, Escape leaves. A line past the end is the last one.

use egui::{Key, Margin, Modifiers, Stroke, Ui, Vec2};
use i18n::{Language, keys};

use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::editor::Editor;
use crate::widgets::{Button, IconButton, TextField};

const FIELD_WIDTH: f32 = 90.0;

#[derive(Debug, Default)]
pub(super) struct GoToLine {
    pub(super) open: bool,
    typed: String,
    /// The field takes the focus at the next frame.
    focus: bool,
}

impl GoToLine {
    pub(super) fn open(&mut self) {
        self.open = true;
        self.focus = true;
    }

    fn close(&mut self, editor: &mut Editor) {
        self.open = false;
        self.typed.clear();
        editor.focus();
    }

    /// The caret at the start of the line typed; what reads as no line leaves the bar open.
    fn go(&mut self, editor: &mut Editor) {
        let Some(line) = self.typed.trim().parse::<usize>().ok().filter(|it| *it > 0) else {
            return;
        };
        let last = editor.buffer.line_count().saturating_sub(1);
        let at = editor.buffer.line_start((line - 1).min(last));
        editor.select(at..at);
        self.close(editor);
    }
}

/// Ctrl+Alt+G, read while the editor or the bar has the keys.
pub(super) fn shortcut(ui: &Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(Modifiers::COMMAND | Modifiers::ALT, Key::G))
}

pub(super) fn show(ui: &mut Ui, bar: &mut GoToLine, editor: &mut Editor, language: Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    let p = palette(ui.ctx());
    let label = tr(keys::editor::code::GO_TO_LINE);
    egui::Frame::new()
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&label).size(12.0).color(p.text_1));
                let field = ui
                    .scope(|ui| {
                        ui.set_width(FIELD_WIDTH);
                        TextField::new(&mut bar.typed, &label, "go-to-line-input")
                            .compact()
                            .show(ui)
                    })
                    .inner;
                if std::mem::take(&mut bar.focus) {
                    field.request_focus();
                }
                // Taken, or the code that gets the keys back would type it.
                let entered = field.lost_focus()
                    && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
                let pressed = Button::new(&tr(keys::editor::code::GO), "go-to-line-submit")
                    .show(ui)
                    .clicked();
                if entered || pressed {
                    bar.go(editor);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if IconButton::new(
                        Icon::X,
                        &tr(keys::editor::search::CLOSE),
                        "go-to-line-close",
                    )
                    .show(ui)
                    .clicked()
                    {
                        bar.close(editor);
                    }
                });
            });
        });
    let rect = ui.min_rect();
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        bar.close(editor);
    }
}
