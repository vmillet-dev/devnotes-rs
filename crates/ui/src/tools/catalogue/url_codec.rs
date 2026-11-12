//! One text, two tabs. Encoding: both fields are typed into, the one typed in last is the text and
//! the other its translation. Analysing reads that same text.

use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::encoding::{CodecDirection, UrlCodecAnswer, UrlCodecRequest, UrlScope};
use engine::tools::url_parts::{UrlAnswer, UrlParts};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    CopyValue, ResultRow, card, columns, hint, named, note_import, problem, result_rows,
    section_head, section_title, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Segmented};

/// Its query is encoded: decoded on one tab, its parameters read on the other.
const SAMPLE: &str = "https://ada:secret@api.example.com:8443/v2/search?q=caf%C3%A9%20cr%C3%A8me&tag=a&tag=b#results";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Decoded,
    Encoded,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Codec,
    Parse,
}

pub(super) struct UrlCodecTool {
    tab: Tab,
    text: String,
    side: Side,
    scope: UrlScope,
    answer: Live<UrlCodecRequest, UrlCodecAnswer>,
    parsed: Live<String, UrlAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(UrlCodecTool {
        tab: Tab::Codec,
        text: String::new(),
        side: Side::Decoded,
        scope: UrlScope::Component,
        answer: Live::default(),
        parsed: Live::default(),
    })
}

/// Written as the URL gives them: a password is never read back, only said to be there.
fn part_rows(parts: &UrlParts) -> Vec<(&'static str, String)> {
    let hidden = parts.password.as_ref().map(|_| "••••••".to_owned());
    let port = parts.port.map(|port| port.to_string());
    let rows = [
        ("scheme", Some(parts.scheme.clone())),
        ("username", parts.username.clone()),
        ("password", hidden),
        ("host", parts.host.clone()),
        ("port", port),
        ("path", Some(parts.path.clone())),
        ("query", parts.query.clone()),
        ("fragment", parts.fragment.clone()),
    ];
    rows.into_iter()
        .filter_map(|(id, value)| Some((id, value.filter(|it| !it.is_empty())?)))
        .collect()
}

impl UrlCodecTool {
    fn direction(&self) -> CodecDirection {
        match self.side {
            Side::Decoded => CodecDirection::Encode,
            Side::Encoded => CodecDirection::Decode,
        }
    }

    /// An answer going the other way is not shown: typed on the other side, it is the reverse.
    fn current(&self) -> Option<&UrlCodecAnswer> {
        let asked = self.answer.answered()?;
        (asked.direction == self.direction())
            .then(|| self.answer.value())
            .flatten()
    }

    fn translated(&self) -> &str {
        match self.current() {
            Some(UrlCodecAnswer::Done { text }) => text,
            _ => "",
        }
    }

    fn of(&self, side: Side) -> &str {
        if self.side == side {
            &self.text
        } else {
            self.translated()
        }
    }

    fn parts(&self) -> Option<&UrlParts> {
        match self.parsed.value()? {
            UrlAnswer::Parsed { parts } => Some(parts),
            UrlAnswer::Invalid { .. } => None,
        }
    }

    fn bar(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::url_codec::{scopes, tabs};
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [(Tab::Codec, tr(tabs::CODEC)), (Tab::Parse, tr(tabs::PARSE))];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            Segmented::new(&options, "url-tab").show(ui, &mut self.tab);
            if self.tab == Tab::Codec {
                let options = [
                    (UrlScope::Component, tr(scopes::COMPONENT)),
                    (UrlScope::Whole, tr(scopes::WHOLE)),
                ];
                let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
                crate::tools::ui::label(ui, &tr(keys::tools::url_codec::SCOPE));
                Segmented::new(&options, "url-codec-scope").show(ui, &mut self.scope);
                hint(
                    ui,
                    &tr(match self.scope {
                        UrlScope::Component => keys::tools::url_codec::scope_hints::COMPONENT,
                        UrlScope::Whole => keys::tools::url_codec::scope_hints::WHOLE,
                    }),
                );
            }
        });
    }

    fn codec_field(&mut self, ui: &mut Ui, side: Side, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let (name, id, placeholder) = match side {
            Side::Decoded => (
                tr(keys::tools::url_codec::DECODED),
                "url-codec-decoded",
                tr(keys::tools::url_codec::DECODED_PLACEHOLDER),
            ),
            Side::Encoded => (
                tr(keys::tools::url_codec::ENCODED),
                "url-codec-encoded",
                tr(keys::tools::url_codec::ENCODED_PLACEHOLDER),
            ),
        };
        let mut shown = self.of(side).to_owned();
        if text_area(ui, &mut shown, (&name, id), &placeholder, 8).changed() {
            self.side = side;
            self.text = shown;
        }
    }

    fn decoded(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let name = i18n::translate(language, keys::tools::url_codec::DECODED, &[]);
        let value = self.of(Side::Decoded).to_owned();
        section_head(ui, &name, |ui| {
            CopyValue::new(&value, &name, language).show(ui);
            note_import(ui, drawn, "text");
        });
        self.codec_field(ui, Side::Decoded, language);
        let offers =
            self.parts().is_some() && self.parsed.answered().map(String::as_str) == Some(&*value);
        if offers {
            let words = i18n::translate(language, keys::tools::url_codec::ANALYSE_THIS, &[]);
            if Button::new(&words, "url-analyse-this").show(ui).clicked() {
                self.text = value;
                self.side = Side::Decoded;
                self.tab = Tab::Parse;
            }
        }
    }

    fn encoded(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let name = i18n::translate(language, keys::tools::url_codec::ENCODED, &[]);
        let value = self.of(Side::Encoded).to_owned();
        section_head(ui, &name, |ui| {
            CopyValue::new(&value, &name, language).show(ui);
        });
        self.codec_field(ui, Side::Encoded, language);
        let said = match self.current() {
            Some(UrlCodecAnswer::MalformedEscape { at }) => Some(
                Message::new(keys::tools::url_codec::MALFORMED)
                    .with("at", *at + 1)
                    .text(language),
            ),
            Some(UrlCodecAnswer::NotUtf8) => Some(i18n::translate(
                language,
                keys::tools::url_codec::NOT_UTF8,
                &[],
            )),
            _ => None,
        };
        if let Some(said) = said {
            problem(ui, &said, "url-codec-problem");
        }
    }

    fn parse_input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::INPUT));
        let name = tr(keys::tools::url_codec::INPUT_LABEL);
        let placeholder = tr(keys::tools::url_codec::PLACEHOLDER);
        text_area(
            ui,
            &mut self.text,
            (&name, "url-parser-input"),
            &placeholder,
            8,
        );
        if let Some(UrlAnswer::Invalid { problem: reason }) = self.parsed.value() {
            let said = named(language, "tools.url-codec.problems", &json!(reason));
            problem(ui, &said, "url-parser-problem");
        }
    }

    fn parse_output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::url_codec::PARTS));
        let Some(parts) = self.parts() else {
            if self.parsed.value().is_none() {
                hint(ui, &tr(keys::tools::url_codec::EMPTY));
            }
            return;
        };
        let rows = part_rows(parts);
        let names: Vec<String> = rows
            .iter()
            .map(|(id, _)| named(language, "tools.url-codec.labels", &json!(id)))
            .collect();
        let aside = tr(keys::tools::url_codec::DEFAULT_PORT);
        card(ui, "url-parser-parts", |ui| {
            let shown: Vec<_> = rows
                .iter()
                .zip(&names)
                .map(|((id, value), name)| {
                    let row = ResultRow::new(name, value);
                    let row = if *id == "password" {
                        row.reading()
                    } else {
                        row
                    };
                    if *id == "port" && parts.port_is_default {
                        row.meta(&aside)
                    } else {
                        row
                    }
                })
                .collect();
            result_rows(ui, &shown, language);
        });
        if parts.parameters.is_empty() {
            return;
        }
        let count = u32::try_from(parts.parameters.len()).unwrap_or(u32::MAX);
        let title = Message::new(keys::tools::url_codec::PARAMETERS)
            .with("count", count)
            .text(language);
        section_title(ui, &title);
        card(ui, "url-parser-parameters", |ui| {
            let shown: Vec<_> = parts
                .parameters
                .iter()
                .map(|parameter| ResultRow::new(&parameter.name, &parameter.value))
                .collect();
            result_rows(ui, &shown, language);
        });
    }

    /// As JSON, so the note reads the same in any language; the password stays behind.
    fn parts_result(&self, language: Language) -> Option<ToolResult> {
        let parts = self.parts()?;
        let parameters: Vec<_> = parts
            .parameters
            .iter()
            .map(|it| json!({ "name": it.name, "value": it.value }))
            .collect();
        let kept = json!({
            "scheme": parts.scheme,
            "username": parts.username,
            "host": parts.host,
            "port": parts.port,
            "portIsDefault": parts.port_is_default,
            "path": parts.path,
            "query": parts.query,
            "fragment": parts.fragment,
            "parameters": parameters,
        });
        let host = parts.host.as_deref().unwrap_or(&parts.scheme);
        Some(ToolResult {
            title: Message::new(keys::tools::url_codec::NOTE_PARSED)
                .with("host", host)
                .text(language),
            language: NoteLanguage::Json,
            content: serde_json::to_string_pretty(&kept).ok()?,
            warning: None,
        })
    }

    fn codec_result(&self, language: Language) -> Option<ToolResult> {
        let translated = self.translated();
        if translated.is_empty() {
            return None;
        }
        let encoding = self.answer.answered()?.direction == CodecDirection::Encode;
        let title = if encoding {
            keys::tools::url_codec::NOTE_ENCODED
        } else {
            keys::tools::url_codec::NOTE_DECODED
        };
        Some(ToolResult {
            title: i18n::translate(language, title, &[]),
            language: NoteLanguage::Txt,
            content: translated.to_owned(),
            warning: None,
        })
    }
}

impl Tool for UrlCodecTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.text.is_empty()).then(|| UrlCodecRequest {
            text: self.text.clone(),
            direction: self.direction(),
            scope: self.scope,
        });
        self.answer.update(turn, request, api::tools::url_codec);
        // On the codec's tab, the decoded side is read too: a URL there is offered to the parser.
        let read = match self.tab {
            Tab::Parse => self.text.clone(),
            Tab::Codec => self.of(Side::Decoded).to_owned(),
        };
        let read = (!read.trim().is_empty()).then_some(read);
        self.parsed.update(turn, read, api::tools::parse_url);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        self.bar(ui, drawn.language);
        ui.add_space(12.0);
        match self.tab {
            Tab::Codec => columns(ui, self, drawn, (Self::decoded, Self::encoded)),
            Tab::Parse => columns(ui, self, drawn, (Self::parse_input, Self::parse_output)),
        }
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        match self.tab {
            Tab::Codec => self.codec_result(language),
            Tab::Parse => self.parts_result(language),
        }
    }

    /// Into the decoded side, which is where a text is typed.
    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.side = Side::Decoded;
        self.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.side = Side::Encoded;
        SAMPLE.clone_into(&mut self.text);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}
