//! Any format to any other, through one value in Rust; the conventions are said beside the result.

use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::convert::{ConvertAnswer, ConvertRequest, DataFormat};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::icons::Icon;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    CopyValue, code_output, columns, hint, named, note_import, notice, problem, section_head,
    text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{IconButton, Segmented, tooltip};

/// The mockup's document: its `null` is what TOML leaves out and names.
const SAMPLE: &str = r#"{
  "nom": "Dupont",
  "prenom": "Jean",
  "age": 30,
  "estActif": true,
  "competences": ["JavaScript", "Python", "JSON"],
  "adresse": {
    "ville": "Paris",
    "codePostal": "75001"
  },
  "telephone": null
}"#;

const FORMATS: [(DataFormat, &str); 4] = [
    (DataFormat::Json, "JSON"),
    (DataFormat::Toml, "TOML"),
    (DataFormat::Xml, "XML"),
    (DataFormat::Yaml, "YAML"),
];

/// How many dropped keys are named before « … ».
const NAMED: usize = 3;

pub(super) struct ConvertTool {
    text: String,
    from: DataFormat,
    to: DataFormat,
    answer: Live<ConvertRequest, ConvertAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(ConvertTool {
        text: String::new(),
        from: DataFormat::Json,
        to: DataFormat::Yaml,
        answer: Live::default(),
    })
}

fn name_of(format: DataFormat) -> &'static str {
    FORMATS
        .iter()
        .find(|(it, _)| *it == format)
        .map_or("", |(_, name)| name)
}

/// What a note keeps the result as, so it is highlighted in its own language.
fn language_of(format: DataFormat) -> NoteLanguage {
    match format {
        DataFormat::Json => NoteLanguage::Json,
        DataFormat::Toml => NoteLanguage::Toml,
        DataFormat::Xml => NoteLanguage::Xml,
        DataFormat::Yaml => NoteLanguage::Yml,
    }
}

fn format_of(language: NoteLanguage) -> Option<DataFormat> {
    match language {
        NoteLanguage::Json => Some(DataFormat::Json),
        NoteLanguage::Toml => Some(DataFormat::Toml),
        NoteLanguage::Xml => Some(DataFormat::Xml),
        NoteLanguage::Yml => Some(DataFormat::Yaml),
        _ => None,
    }
}

/// A format is never converted to itself: the one the other side holds is not taken.
fn side(ui: &mut Ui, chosen: &mut DataFormat, other: DataFormat, author_id: &str) {
    Segmented::new(&FORMATS, author_id)
        .taken(other)
        .show(ui, chosen);
}

impl ConvertTool {
    fn converted(&self) -> Option<&str> {
        match self.answer.value()? {
            ConvertAnswer::Converted { text, .. } => Some(text),
            _ => None,
        }
    }

    /// The formats trade sides, and the result becomes what is converted back: a round trip.
    fn swap(&mut self) {
        let converted = self.converted().map(str::to_owned);
        let asked = self.answer.answered().filter(|_| converted.is_some());
        let (from, to) = asked.map_or((self.from, self.to), |it| (it.from, it.to));
        self.from = to;
        self.to = from;
        if let Some(text) = converted {
            self.text = text;
        }
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let (from, to) = (&mut self.from, self.to);
        section_head(ui, &tr(keys::tools::convert::FROM), |ui| {
            side(ui, from, to, "convert-from");
        });
        let name = tr(keys::tools::convert::INPUT_LABEL);
        let placeholder = named(language, "tools.convert.placeholders", &json!(self.from));
        text_area(
            ui,
            &mut self.text,
            (&name, "convert-input"),
            &placeholder,
            16,
        );
        note_import(ui, drawn, "text");
        if let (Some(ConvertAnswer::Unreadable { line, column }), Some(asked)) =
            (self.answer.value(), self.answer.answered())
        {
            let words = Message::new(keys::tools::convert::UNREADABLE)
                .with("format", name_of(asked.from))
                .with("line", *line)
                .with("column", *column)
                .text(language);
            problem(ui, &words, "convert-unreadable");
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let converted = self.converted().map(str::to_owned);
        let (to, from) = (&mut self.to, self.from);
        let mut swapped = false;
        section_head(ui, &tr(keys::tools::convert::TO), |ui| {
            side(ui, to, from, "convert-to");
            if let Some(text) = &converted {
                CopyValue::new(text, &tr(keys::tools::RESULT), language).show(ui);
            }
            let name = tr(keys::tools::convert::SWAP);
            let swap = IconButton::new(Icon::ArrowLeftRight, &name, "convert-swap").show(ui);
            swapped = tooltip(swap, &name).clicked();
        });
        if swapped {
            self.swap();
        }
        self.dropped(ui, language);
        match (&converted, self.answer.value()) {
            (Some(text), _) => code_output(ui, text, "convert-output"),
            (None, Some(ConvertAnswer::Impossible { crossing, path })) => {
                let reason = named(language, "tools.convert.crossings", &json!(crossing));
                problem(ui, &format!("{reason} {path}"), "convert-impossible");
            }
            (None, None) => {
                hint(ui, &tr(keys::tools::convert::EMPTY));
            }
            _ => {}
        }
        for (format, key) in [
            (DataFormat::Xml, keys::tools::convert::conventions::XML),
            (DataFormat::Toml, keys::tools::convert::conventions::TOML),
        ] {
            if self.from == format || self.to == format {
                hint(ui, &tr(key));
            }
        }
    }

    /// « telephone, adresse.fax, … »: the first keys TOML left out.
    fn dropped(&self, ui: &mut Ui, language: Language) {
        let Some(ConvertAnswer::Converted { dropped, .. }) = self.answer.value() else {
            return;
        };
        if dropped.is_empty() {
            return;
        }
        let count = u32::try_from(dropped.len()).unwrap_or(u32::MAX);
        let head = Message::new(keys::tools::convert::DROPPED)
            .with("count", count)
            .text(language);
        let mut paths = dropped[..dropped.len().min(NAMED)].join(", ");
        if dropped.len() > NAMED {
            paths.push_str(", …");
        }
        let tail = i18n::translate(language, keys::tools::convert::NO_TOML_NULL, &[]);
        notice(ui, &format!("{head} {paths}. {tail}"), "convert-dropped");
    }
}

impl Tool for ConvertTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.text.trim().is_empty()).then(|| ConvertRequest {
            text: self.text.clone(),
            from: self.from,
            to: self.to,
        });
        self.answer.update(turn, request, api::tools::convert_data);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let text = self.converted()?;
        let asked = self.answer.answered()?;
        Some(ToolResult {
            title: Message::new(keys::tools::convert::NOTE_TITLE)
                .with("from", name_of(asked.from))
                .with("to", name_of(asked.to))
                .text(language),
            language: language_of(asked.to),
            content: text.to_owned(),
            warning: None,
        })
    }

    /// A snippet is read in its own format; the other side gives way if it held that one.
    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
        let Some(format) = note.language.and_then(format_of) else {
            return;
        };
        if format == self.to {
            self.to = self.from;
        }
        self.from = format;
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.from = DataFormat::Json;
        self.to = DataFormat::Toml;
        SAMPLE.clone_into(&mut self.text);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}
