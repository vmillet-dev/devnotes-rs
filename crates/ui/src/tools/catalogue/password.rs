//! Passwords drawn in Rust. The options are remembered for the session, the passwords are not: a
//! trip elsewhere draws new ones.

use egui::{CornerRadius, Rect, Sense, Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::random::{CharacterSet, PasswordAnswer, PasswordRequest, Strength};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    OutputRow, check, columns, label, named, number_field, problem, section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Kind, address};

const SETS: [(CharacterSet, &str, &str); 4] = [
    (CharacterSet::Lowercase, "lowercase", "abc"),
    (CharacterSet::Uppercase, "uppercase", "ABC"),
    (CharacterSet::Digits, "digits", "123"),
    (CharacterSet::Symbols, "symbols", "!#$"),
];

const STRENGTHS: [Strength; 5] = [
    Strength::VeryWeak,
    Strength::Weak,
    Strength::Fair,
    Strength::Strong,
    Strength::VeryStrong,
];

const LENGTH: (u32, u32) = (8, 64);

/// A new draw with the same options is part of what is asked.
type Asked = (PasswordRequest, u32);

pub(super) struct PasswordTool {
    length: String,
    sets: [bool; 4],
    avoid_look_alikes: bool,
    count: String,
    draw: u32,
    answer: Live<Asked, PasswordAnswer>,
}

impl Default for PasswordTool {
    fn default() -> Self {
        Self {
            length: "20".to_owned(),
            sets: [true; 4],
            avoid_look_alikes: true,
            count: "5".to_owned(),
            draw: 0,
            answer: Live::default(),
        }
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<PasswordTool>::default()
}

impl PasswordTool {
    fn request(&self) -> Option<Asked> {
        let sets = SETS
            .iter()
            .zip(self.sets)
            .filter(|(_, on)| *on)
            .map(|((set, _, _), _)| *set)
            .collect();
        let request = PasswordRequest {
            length: self.length.trim().parse().ok()?,
            sets,
            avoid_look_alikes: self.avoid_look_alikes,
            count: self.count.trim().parse().ok()?,
        };
        Some((request, self.draw))
    }

    fn options(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::password::OPTIONS));
        let name = tr(keys::tools::password::LENGTH);
        label(ui, &name);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            let mut length: u32 = self.length.trim().parse().unwrap_or(LENGTH.0);
            let range = LENGTH.0..=LENGTH.1;
            let slider = egui::Slider::new(&mut length, range).show_value(false);
            let moved = ui.add(slider);
            ui.ctx()
                .accesskit_node_builder(moved.id, |node| node.set_label(name.clone()));
            if moved.changed() {
                self.length = length.to_string();
            }
            number_field(ui, &mut self.length, &name, "password-length");
        });
        label(ui, &tr(keys::tools::password::SETS));
        for (at, (_, id, sample)) in SETS.iter().enumerate() {
            let words = named(language, "tools.password.setNames", &json!(id));
            let words = format!("{words}  {sample}");
            check(
                ui,
                &mut self.sets[at],
                &words,
                &format!("password-set-{id}"),
            );
        }
        let words = format!("{}  l 1 I 0 O |", tr(keys::tools::password::LOOK_ALIKES));
        check(
            ui,
            &mut self.avoid_look_alikes,
            &words,
            "password-look-alikes",
        );
        let name = tr(keys::tools::password::COUNT);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            label(ui, &name);
            number_field(ui, &mut self.count, &name, "password-count");
            let words = tr(keys::tools::password::GENERATE);
            if Button::new(&words, "password-generate")
                .kind(Kind::Filled)
                .show(ui)
                .clicked()
            {
                self.draw += 1;
            }
        });
    }

    fn passwords(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::password::PASSWORDS));
        match self.answer.value() {
            Some(PasswordAnswer::Generated {
                passwords,
                entropy_bits,
                strength,
            }) => {
                strength_bar(ui, *entropy_bits, *strength, language);
                for (at, password) in passwords.iter().enumerate() {
                    OutputRow::new(&format!("#{}", at + 1), password).show(ui, language);
                }
            }
            Some(PasswordAnswer::NoCharacters) => {
                let words = tr(keys::tools::password::NO_CHARACTERS);
                problem(ui, &words, "password-problem");
            }
            None => {}
        }
    }
}

fn strength_bar(ui: &mut Ui, bits: f64, strength: Strength, language: Language) {
    let p = palette(ui.ctx());
    let level = STRENGTHS.iter().position(|it| *it == strength).unwrap_or(0) + 1;
    let hue = match level {
        1 | 2 => p.red,
        3 => p.amber_text,
        _ => p.green,
    };
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), Sense::hover());
    let step = (rect.width() - 4.0 * 4.0) / 5.0;
    for at in 0..5 {
        #[expect(clippy::cast_precision_loss, reason = "five steps")]
        let left = rect.left() + at as f32 * (step + 4.0);
        let piece =
            Rect::from_min_size(egui::pos2(left, rect.top()), Vec2::new(step, rect.height()));
        let fill = if at < level { hue } else { p.bg_3 };
        ui.painter().rect_filled(piece, CornerRadius::same(2), fill);
    }
    #[expect(clippy::cast_possible_truncation, reason = "a few hundred bits")]
    let bits = bits.round() as i32;
    let entropy = Message::new(keys::tools::password::ENTROPY)
        .with("bits", f64::from(bits))
        .text(language);
    let name = named(language, "tools.password.strengths", &json!(strength));
    let said = crate::tools::ui::hint(ui, &format!("{entropy} · {name}"));
    address(ui, &said, "password-strength");
}

impl Tool for PasswordTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.answer.update(turn, request, |backend, (asked, _)| {
            api::tools::generate_passwords(backend, asked)
        });
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::options, Self::passwords));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let Some(PasswordAnswer::Generated { passwords, .. }) = self.answer.value() else {
            return None;
        };
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::password::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: passwords.join("\n"),
            warning: Some(keys::tools::password::SAVE_WARNING),
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        "24".clone_into(&mut self.length);
        self.sets = [true; 4];
        self.avoid_look_alikes = true;
        "3".clone_into(&mut self.count);
    }

    /// Nothing typed to empty: Clear puts the options back as they were.
    fn clear(&mut self) {
        *self = Self {
            draw: self.draw,
            ..Self::default()
        };
    }

    fn left(&mut self) {
        self.answer = Live::default();
    }
}
