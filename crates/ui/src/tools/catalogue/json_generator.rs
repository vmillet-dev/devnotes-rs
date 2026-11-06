//! Documents drawn from a schema or from an example. Without a seed each draw takes a new one and
//! says which: pinned, it makes the draw again. A seed is any text, read by Rust.

use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::generate::{GenerateAnswer, GenerateRequest, JsonSource};
use i18n::{Language, Message, keys};

use crate::api;
use crate::theme::icons::Icon;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    CopyValue, code_output, columns, hint, label, line, note_import, number_field, problem,
    section_head, text_area,
};
use crate::tools::{Drawn, Tool, ToolAction, ToolResult};
use crate::widgets::{Button, IconButton, Segmented, TextField, address, tooltip};

const SAMPLE_EXAMPLE: &str = r#"{
  "id": "0b1c2d3e-4f50-4612-8a7b-9c0d1e2f3a4b",
  "email": "ada@example.com",
  "name": "Ada Lovelace",
  "createdAt": "2026-01-15T09:30:00Z",
  "tags": ["admin", "beta"],
  "score": 42
}"#;

const SAMPLE_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["id", "email"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "email": { "type": "string", "format": "email" },
    "name": { "type": "string", "minLength": 3, "maxLength": 24 },
    "createdAt": { "type": "string", "format": "date-time" },
    "tags": {
      "type": "array",
      "maxItems": 3,
      "items": { "enum": ["admin", "beta", "staff"] }
    },
    "score": { "type": "integer", "minimum": 0, "maximum": 100 }
  }
}"#;

/// The request, and which draw of it: asking again draws other documents.
type Asked = (GenerateRequest, u32);

pub(super) struct JsonGeneratorTool {
    source: JsonSource,
    text: String,
    count: String,
    seed: String,
    draw: u32,
    answer: Live<Asked, GenerateAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(JsonGeneratorTool {
        source: JsonSource::Example,
        text: String::new(),
        count: "3".to_owned(),
        seed: String::new(),
        draw: 0,
        answer: Live::default(),
    })
}

impl JsonGeneratorTool {
    fn request(&self) -> Option<Asked> {
        if self.text.trim().is_empty() {
            return None;
        }
        let seed = self.seed.trim();
        let request = GenerateRequest {
            source: self.source,
            text: self.text.clone(),
            count: self.count.trim().parse().ok()?,
            seed: (!seed.is_empty()).then(|| seed.to_owned()),
        };
        Some((request, self.draw))
    }

    fn generated(&self) -> Option<(&str, u32)> {
        match self.answer.value()? {
            GenerateAnswer::Generated { text, seed, .. } => Some((text, *seed)),
            _ => None,
        }
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::json_generator as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (JsonSource::Schema, tr(words::sources::SCHEMA)),
            (JsonSource::Example, tr(words::sources::EXAMPLE)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        let source = &mut self.source;
        section_head(ui, &tr(words::SOURCE), |ui| {
            Segmented::new(&options, "json-generator-source").show(ui, source);
            note_import(ui, drawn, "text");
        });
        let (expects, placeholder) = match self.source {
            JsonSource::Schema => (words::expects::SCHEMA, words::placeholders::SCHEMA),
            JsonSource::Example => (words::expects::EXAMPLE, words::placeholders::EXAMPLE),
        };
        let said = hint(ui, &tr(expects));
        address(ui, &said, "json-generator-expects");
        let name = tr(words::INPUT_LABEL);
        let placeholder = tr(placeholder);
        text_area(
            ui,
            &mut self.text,
            (&name, "json-generator-input"),
            &placeholder,
            14,
        );
        self.refusal(ui, language);
        self.seed_row(ui, language);
    }

    fn refusal(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::json_generator as words;
        let tr = |key| i18n::translate(language, key, &[]);
        match self.answer.value() {
            Some(GenerateAnswer::Unreadable { line, column }) => {
                let said = Message::new(words::UNREADABLE)
                    .with("line", *line)
                    .with("column", *column)
                    .text(language);
                problem(ui, &said, "json-generator-unreadable");
            }
            Some(GenerateAnswer::NotASchema) => {
                problem(ui, &tr(words::NOT_ASCHEMA), "json-generator-not-schema");
            }
            Some(GenerateAnswer::AnExample) => {
                let said = hint(ui, &tr(words::AN_EXAMPLE));
                address(ui, &said, "json-generator-an-example");
                if Button::new(&tr(words::AS_EXAMPLE), "json-generator-as-example")
                    .show(ui)
                    .clicked()
                {
                    self.source = JsonSource::Example;
                }
            }
            _ => {}
        }
    }

    /// One place for the seed: the one typed, or the one just drawn and the pin that fixes it.
    fn seed_row(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::json_generator as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let drawn = self.generated().map(|(_, seed)| seed);
        let pinned = !self.seed.trim().is_empty();
        line(ui, |ui| {
            let name = tr(words::COUNT);
            label(ui, &name);
            number_field(ui, &mut self.count, &name, "json-generator-count");
            let name = tr(words::SEED);
            label(ui, &name);
            ui.scope(|ui| {
                ui.set_max_width(140.0);
                TextField::new(&mut self.seed, &name, "json-generator-seed")
                    .hint(&tr(words::SEED_PLACEHOLDER))
                    .compact()
                    .show(ui);
            });
            if pinned || drawn.is_some() {
                let key = if pinned {
                    words::FREE_SEED
                } else {
                    words::KEEP_SEED
                };
                let why = tr(key);
                let pin = IconButton::new(Icon::Pin, &why, "json-generator-keep-seed")
                    .pressed(pinned)
                    .show(ui);
                if tooltip(pin, &why).clicked() {
                    self.seed = match (pinned, drawn) {
                        (false, Some(seed)) => seed.to_string(),
                        _ => String::new(),
                    };
                }
            }
        });
        if pinned {
            let said = hint(ui, &tr(words::SEED_PINNED));
            address(ui, &said, "json-generator-seed-pinned");
        } else if let Some(seed) = drawn {
            let words = Message::new(words::SEED_USED)
                .with("seed", seed)
                .text(language);
            let said = hint(ui, &words);
            address(ui, &said, "json-generator-seed-used");
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::json_generator as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::DOCUMENTS);
        let Some(GenerateAnswer::Generated {
            text, unsupported, ..
        }) = self.answer.value()
        else {
            section_head(ui, &name, |_| {});
            if !matches!(self.answer.value(), Some(GenerateAnswer::AnExample)) {
                let said = hint(ui, &tr(words::EMPTY));
                address(ui, &said, "json-generator-empty");
            }
            return;
        };
        section_head(ui, &name, |ui| {
            CopyValue::new(text, &name, language).show(ui);
        });
        code_output(ui, text, "json-generator-output");
        if !unsupported.is_empty() {
            let said = hint(ui, &tr(words::UNSUPPORTED));
            address(ui, &said, "json-generator-unsupported");
            let at = tr(words::UNSUPPORTED_AT);
            for item in unsupported {
                hint(ui, &format!("{} ({at} {})", item.keyword, item.path));
            }
        }
    }
}

impl Tool for JsonGeneratorTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.answer.update(turn, request, |backend, (asked, _)| {
            api::tools::generate_json(backend, asked)
        });
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let (text, _) = self.generated()?;
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::json_generator::NOTE_TITLE, &[]),
            language: NoteLanguage::Json,
            content: text.to_owned(),
            warning: None,
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
    }

    /// Never disabled: with a seed pinned it draws the same documents, and the page says so.
    fn actions(&self) -> Vec<ToolAction> {
        vec![ToolAction {
            id: "draw",
            label: keys::tools::json_generator::AGAIN,
            enabled: true,
        }]
    }

    fn run_action(&mut self, id: &str, _: &egui::Context) {
        if id == "draw" {
            self.draw += 1;
        }
    }

    fn has_sample(&self) -> bool {
        true
    }

    /// One sample a mode: read as a schema, an example gave nonsense.
    fn sample(&mut self, _: Language) {
        let sample = match self.source {
            JsonSource::Schema => SAMPLE_SCHEMA,
            JsonSource::Example => SAMPLE_EXAMPLE,
        };
        sample.clone_into(&mut self.text);
        "3".clone_into(&mut self.count);
    }

    fn clear(&mut self) {
        self.text.clear();
        self.seed.clear();
    }
}
