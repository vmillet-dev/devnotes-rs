//! A text in six encodings at once, and back from whichever one Rust recognises.

use std::path::PathBuf;
use std::sync::Arc;

use egui::{ColorImage, TextureHandle, TextureOptions, Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::bases::{
    Base64FileAnswer, ByteEncoding, DecodeAnswer, DecodeRequest, DecodedBytes, DecodedImage,
    EncodeRequest, Encodings, ImagePreview, SavedBytes,
};
use engine::tools::image::ImageFormat;
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::backend::Pending;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    CopyValue, ResultRow, card, check, columns, hint, named, note_import, problem, result_rows,
    section_head, section_title, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Segmented, address};

/// What the page draws of a long result; the copy button still copies it whole.
const SHOWN_CHARACTERS: usize = 4000;
/// Under this many pixels on its longer side, a picture is drawn enlarged.
const SMALL: f32 = 64.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Encode,
    Decode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Text,
    File,
}

/// A file to find, for reading it or for writing what was decoded.
enum Asked {
    Reading(Pending<Option<PathBuf>>),
    Destination(Pending<Option<PathBuf>>),
    Saving(Pending<SavedBytes>),
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "two options, two buttons pressed"
)]
pub(super) struct Base64Tool {
    direction: Direction,
    source: Source,
    text: String,
    path: Option<String>,
    spaced: bool,
    uppercase: bool,
    reading: Option<ByteEncoding>,
    encoded_text: Live<EncodeRequest, Encodings>,
    encoded_file: Live<String, Base64FileAnswer>,
    decoded: Live<DecodeRequest, DecodeAnswer>,
    wants_file: bool,
    wants_save: bool,
    asked: Option<Asked>,
    /// The data URL last drawn and its picture, decoded off the window's thread.
    drawing: Option<(String, Pending<Option<ColorImage>>)>,
    picture: Option<(String, TextureHandle)>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(Base64Tool {
        direction: Direction::Encode,
        source: Source::Text,
        text: String::new(),
        path: None,
        spaced: true,
        uppercase: false,
        reading: None,
        encoded_text: Live::default(),
        encoded_file: Live::default(),
        decoded: Live::default(),
        wants_file: false,
        wants_save: false,
        asked: None,
        drawing: None,
        picture: None,
    })
}

fn shortened(text: &str) -> String {
    match text.char_indices().nth(SHOWN_CHARACTERS) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_owned(),
    }
}

/// Their own names in every language.
fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::Webp => "WebP",
        ImageFormat::Svg => "SVG",
    }
}

/// The key is also the extension a file of it is saved under.
fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Gif => "gif",
        ImageFormat::Webp => "webp",
        ImageFormat::Svg => "svg",
    }
}

impl Base64Tool {
    /// The rows of a text's encodings, or a file's two Base64: the id and the value.
    fn rows(&self) -> Vec<(&'static str, &str)> {
        if self.source == Source::File {
            return match self.encoded_file.value() {
                Some(Base64FileAnswer::Encoded {
                    base64, base64_url, ..
                }) => vec![
                    ("base64", base64.as_str()),
                    ("base64Url", base64_url.as_str()),
                ],
                _ => Vec::new(),
            };
        }
        self.encoded_text.value().map_or_else(Vec::new, |it| {
            vec![
                ("base64", it.base64.as_str()),
                ("base64Url", it.base64_url.as_str()),
                ("base32", it.base32.as_str()),
                ("hex", it.hex.as_str()),
                ("binary", it.binary.as_str()),
                ("decimal", it.decimal.as_str()),
            ]
        })
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::base64::{directions, sources};
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (Direction::Encode, tr(directions::ENCODE)),
            (Direction::Decode, tr(directions::DECODE)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        let origins = [
            (Source::Text, tr(sources::TEXT)),
            (Source::File, tr(sources::FILE)),
        ];
        let origins = origins.each_ref().map(|(id, name)| (*id, name.as_str()));
        let (direction, source) = (&mut self.direction, &mut self.source);
        section_head(ui, &tr(keys::tools::INPUT), |ui| {
            if *direction == Direction::Decode || *source == Source::Text {
                note_import(ui, drawn, "text");
            }
            if *direction == Direction::Encode {
                Segmented::new(&origins, "base64-source").show(ui, source);
            }
            Segmented::new(&options, "base64-direction").show(ui, direction);
        });
        if self.direction == Direction::Encode && self.source == Source::File {
            self.file(ui, language);
        } else {
            let placeholder = if self.direction == Direction::Encode {
                tr(keys::tools::base64::ENCODE_PLACEHOLDER)
            } else {
                tr(keys::tools::base64::DECODE_PLACEHOLDER)
            };
            let name = tr(keys::tools::base64::INPUT_LABEL);
            text_area(ui, &mut self.text, (&name, "base64-input"), &placeholder, 6);
        }
        match self.direction {
            Direction::Encode => self.encode_facts(ui, language),
            Direction::Decode => self.reading_line(ui, language),
        }
        crate::tools::ui::notice(ui, &tr(keys::tools::base64::EXPLAIN), "base64-explain");
    }

    fn file(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let words = tr(keys::tools::base64::PICK_FILE);
        if Button::new(&words, "base64-pick-file").show(ui).clicked() {
            self.wants_file = true;
        }
        match self.encoded_file.value() {
            Some(Base64FileAnswer::Encoded { name, bytes, .. }) => {
                let words = Message::new(keys::tools::base64::FILE_READ)
                    .with("name", name.as_str())
                    .with("count", *bytes)
                    .text(language);
                let said = hint(ui, &words);
                address(ui, &said, "base64-file");
            }
            Some(Base64FileAnswer::Failed { problem: reason }) => {
                let words = named(language, "tools.files", &json!(reason));
                problem(ui, &words, "base64-file-problem");
            }
            None => {
                hint(ui, &tr(keys::tools::base64::FILE_HINT));
            }
        }
    }

    fn encode_facts(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        if self.source != Source::Text {
            return;
        }
        if let Some(facts) = self.encoded_text.value() {
            let characters = Message::new(keys::tools::base64::CHARACTERS)
                .with("count", facts.characters)
                .text(language);
            let bytes = Message::new(keys::tools::base64::BYTES)
                .with("count", facts.bytes)
                .text(language);
            let mut words = format!("{characters} · {bytes}");
            if let Some(wide) = &facts.wide {
                let said = Message::new(keys::tools::base64::WIDE)
                    .with("character", wide.character.as_str())
                    .with("bytes", wide.bytes)
                    .text(language);
                words = format!("{words} {said}");
            }
            let said = hint(ui, &words);
            address(ui, &said, "base64-facts");
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let words = tr(keys::tools::base64::SPACED);
            check(ui, &mut self.spaced, &words, "base64-spaced");
            let words = tr(keys::tools::base64::UPPERCASE);
            check(ui, &mut self.uppercase, &words, "base64-uppercase");
        });
    }

    fn reading_line(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::base64::readings;
        let tr = |key| i18n::translate(language, key, &[]);
        if let Some(answer) = self.decoded.value() {
            let read = named(language, "tools.base64.encodings", &json!(answer.read_as));
            let mut words = format!("{} {read}", tr(keys::tools::base64::READ_AS));
            if answer.guessed {
                words = format!("{words} {}", tr(keys::tools::base64::GUESSED));
            }
            if let Some(uri) = &answer.data_uri {
                let about = if uri.is_empty() {
                    tr(keys::tools::base64::DATA_URI_UNTYPED)
                } else {
                    Message::new(keys::tools::base64::DATA_URI)
                        .with("type", uri.as_str())
                        .text(language)
                };
                words = format!("{words} {about}");
            }
            let said = hint(ui, &words);
            address(ui, &said, "base64-reading");
        }
        let options = [
            (None, tr(readings::AUTO)),
            (Some(ByteEncoding::Base64), tr(readings::BASE64)),
            (Some(ByteEncoding::Base64Url), tr(readings::BASE64_URL)),
            (Some(ByteEncoding::Base32), tr(readings::BASE32)),
            (Some(ByteEncoding::Hex), tr(readings::HEX)),
            (Some(ByteEncoding::Binary), tr(readings::BINARY)),
            (Some(ByteEncoding::Decimal), tr(readings::DECIMAL)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&options, "base64-reading-choice").show(ui, &mut self.reading);
        if let Some(answer) = self.decoded.value().filter(|it| !it.also.is_empty()) {
            let others: Vec<String> = answer
                .also
                .iter()
                .map(|it| named(language, "tools.base64.encodings", &json!(it)))
                .collect();
            let words = format!("{} {}.", tr(keys::tools::base64::ALSO), others.join(", "));
            let said = hint(ui, &words);
            address(ui, &said, "base64-also");
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        match self.direction {
            Direction::Encode => {
                section_title(ui, &tr(keys::tools::RESULTS));
                self.encoded(ui, language);
            }
            Direction::Decode => {
                section_title(ui, &tr(keys::tools::RESULT));
                self.decoded_side(ui, language);
                self.image(ui, language);
            }
        }
    }

    fn encoded(&self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let rows = self.rows();
        if rows.is_empty() {
            hint(ui, &tr(keys::tools::base64::ENCODE_EMPTY));
            return;
        }
        let shown: Vec<(String, String, &str)> = rows
            .iter()
            .map(|(id, value)| {
                let name = named(language, "tools.base64.encodings", &json!(id));
                (name, shortened(value), *value)
            })
            .collect();
        card(ui, "base64-encodings", |ui| {
            let rows: Vec<_> = shown
                .iter()
                .map(|(name, value, whole)| ResultRow::new(name, value).copied(whole))
                .collect();
            result_rows(ui, &rows, language);
        });
        if self.source == Source::File {
            hint(ui, &tr(keys::tools::base64::FILE_ONLY_BASE64));
        }
    }

    fn decoded_side(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(answer) = self.decoded.value() else {
            hint(ui, &tr(keys::tools::base64::DECODE_EMPTY));
            return;
        };
        match &answer.decoded {
            DecodedBytes::Text { text, .. } => {
                let name = tr(keys::tools::base64::DECODED_TEXT);
                section_head(ui, &name, |ui| {
                    CopyValue::new(text, &tr(keys::tools::RESULT), language).show(ui);
                });
                let shown = egui::RichText::new(text).font(egui::FontId::monospace(12.5));
                let label = ui.add(egui::Label::new(shown).selectable(true));
                address(ui, &label, "base64-output");
            }
            DecodedBytes::Binary { bytes, preview } => {
                let words = Message::new(keys::tools::base64::BINARY)
                    .with("count", *bytes)
                    .text(language);
                let said = hint(ui, &words);
                address(ui, &said, "base64-binary");
                let shown = egui::RichText::new(preview).font(egui::FontId::monospace(12.0));
                ui.add(egui::Label::new(shown).selectable(true));
                let words = tr(keys::tools::base64::SAVE_FILE);
                if Button::new(&words, "base64-save-file").show(ui).clicked() {
                    self.wants_save = true;
                }
            }
            DecodedBytes::Invalid {
                problem: reason,
                at,
                character,
            } => {
                let words = invalid_words(language, *reason, *at, character.as_deref());
                problem(ui, &words, "base64-problem");
            }
        }
    }

    /// The bytes are a picture: shown, not only offered as a file to open elsewhere.
    fn image(&self, ui: &mut Ui, language: Language) {
        let Some(answer) = self.decoded.value() else {
            return;
        };
        let Some(image) = &answer.image else {
            return;
        };
        match &image.preview {
            ImagePreview::Shown { data_url } => {
                if let Some((_, texture)) = self.picture.as_ref().filter(|(it, _)| it == data_url) {
                    drawn_picture(ui, texture, image);
                }
            }
            ImagePreview::TooLarge { limit } => {
                let words = Message::new(keys::tools::base64::image::TOO_LARGE)
                    .with("limit", (*limit).div_ceil(1024 * 1024))
                    .text(language);
                let said = hint(ui, &words);
                address(ui, &said, "base64-image-too-large");
            }
        }
        let bytes = match &answer.decoded {
            DecodedBytes::Text { bytes, .. } | DecodedBytes::Binary { bytes, .. } => *bytes,
            DecodedBytes::Invalid { .. } => 0,
        };
        let weight = Message::new(keys::tools::base64::image::WEIGHT)
            .with("count", bytes)
            .text(language);
        let size = match (image.width, image.height) {
            (Some(width), Some(height)) => format!(" · {width} × {height} px"),
            _ => String::new(),
        };
        let words = format!("{}{size} · {weight}", format_name(image.format));
        let said = hint(ui, &words);
        address(ui, &said, "base64-image-facts");
    }

    fn follow_files(&mut self, turn: &Turn<'_>) {
        if std::mem::take(&mut self.wants_file) {
            let dialog = Arc::clone(turn.dialog);
            let pending = turn.backend.compute(move |_| Ok(dialog.open()));
            self.asked = Some(Asked::Reading(pending));
        }
        if std::mem::take(&mut self.wants_save) {
            let format = self.decoded.value().and_then(|it| it.image.as_ref());
            let name = format!(
                "decoded.{}",
                format.map_or("bin", |it| extension(it.format))
            );
            let dialog = Arc::clone(turn.dialog);
            let pending = turn.backend.compute(move |_| Ok(dialog.save(&name)));
            self.asked = Some(Asked::Destination(pending));
        }
    }

    fn adopt_files(&mut self, turn: &mut Turn<'_>) {
        match self.asked.as_mut() {
            Some(Asked::Reading(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.asked = None;
                if let Ok(Some(path)) = answer {
                    self.path = Some(path.display().to_string());
                }
            }
            Some(Asked::Destination(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.asked = None;
                if let Ok(Some(path)) = answer {
                    let request = DecodeRequest {
                        text: self.text.clone(),
                        reading: self.reading,
                    };
                    let path = path.display().to_string();
                    let saved = api::tools::save_bytes(turn.backend, request, path);
                    self.asked = Some(Asked::Saving(saved));
                }
            }
            Some(Asked::Saving(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.asked = None;
                match answer {
                    Ok(SavedBytes::Saved { bytes }) => turn
                        .status
                        .report(Message::new(keys::tools::base64::SAVED).with("count", bytes)),
                    Ok(SavedBytes::Failed { problem: reason }) => {
                        let path = format!("tools.files.{}", json!(reason).as_str().unwrap_or(""));
                        if let Some(key) = i18n::Key::find(&path) {
                            turn.errors.notify(Message::new(key));
                        }
                    }
                    Ok(SavedBytes::Invalid) => {}
                    Err(error) => turn.errors.report(keys::errors::TOOL_FAILED, &error),
                }
            }
            None => {}
        }
    }

    /// The picture off the window's thread, once a frame until it is on the screen.
    fn follow_picture(&mut self, turn: &Turn<'_>) {
        let wanted = self.decoded.value().and_then(|it| match &it.image {
            Some(DecodedImage {
                preview: ImagePreview::Shown { data_url },
                ..
            }) => Some(data_url.clone()),
            _ => None,
        });
        let Some(wanted) = wanted else {
            self.picture = None;
            self.drawing = None;
            return;
        };
        let current = self.picture.as_ref().is_some_and(|(it, _)| *it == wanted);
        let asked = self.drawing.as_ref().is_some_and(|(it, _)| *it == wanted);
        if !current && !asked {
            let uri = wanted.clone();
            let pending = turn
                .backend
                .compute(move |_| Ok(crate::state::attachments::decode(&uri)));
            self.drawing = Some((wanted, pending));
        }
        if let Some((uri, pending)) = &mut self.drawing
            && let Some(answer) = pending.poll()
        {
            let uri = std::mem::take(uri);
            self.drawing = None;
            if let Ok(Some(image)) = answer {
                let texture =
                    turn.ctx
                        .load_texture("base64-picture", image, TextureOptions::NEAREST);
                self.picture = Some((uri, texture));
            }
        }
    }
}

fn invalid_words(
    language: Language,
    reason: engine::tools::bases::DecodeProblem,
    at: Option<u32>,
    character: Option<&str>,
) -> String {
    let (Some(at), Some(character)) = (at, character) else {
        return i18n::translate(language, keys::tools::base64::problems::LENGTH, &[]);
    };
    let path = format!(
        "tools.base64.problems.{}",
        json!(reason).as_str().unwrap_or_default()
    );
    i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key)
            .with("character", character)
            .with("at", at + 1)
            .text(language)
    })
}

/// An icon of a few pixels is drawn larger, pixel by pixel: at its own size it is a dot.
fn drawn_picture(ui: &mut Ui, texture: &TextureHandle, image: &DecodedImage) {
    let size = texture.size_vec2();
    let small = size.max_elem() < SMALL && image.width.is_some();
    let scale = if small { 4.0 } else { 1.0 };
    let room = ui.available_width();
    let shown = size * scale * (room / (size.x * scale)).min(1.0);
    let drawn = ui.add(
        egui::Image::new((texture.id(), shown)).fit_to_exact_size(Vec2::new(shown.x, shown.y)),
    );
    address(ui, &drawn, "base64-image-picture");
}

impl Tool for Base64Tool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        self.follow_files(turn);
        self.adopt_files(turn);
        let encoding = self.direction == Direction::Encode;
        let text = (encoding && self.source == Source::Text && !self.text.is_empty()).then(|| {
            EncodeRequest {
                text: self.text.clone(),
                spaced: self.spaced,
                uppercase: self.uppercase,
            }
        });
        self.encoded_text
            .update(turn, text, api::tools::encode_bytes);
        let file = self
            .path
            .clone()
            .filter(|_| encoding && self.source == Source::File);
        self.encoded_file
            .update(turn, file, api::tools::encode_base64_file);
        let decoding = !encoding && !self.text.trim().is_empty();
        let request = decoding.then(|| DecodeRequest {
            text: self.text.clone(),
            reading: self.reading,
        });
        self.decoded.update(turn, request, api::tools::decode_bytes);
        self.follow_picture(turn);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let tr = |key| i18n::translate(language, key, &[]);
        if self.direction == Direction::Decode {
            let Some(DecodedBytes::Text { text, .. }) = self.decoded.value().map(|it| &it.decoded)
            else {
                return None;
            };
            return Some(ToolResult {
                title: tr(keys::tools::base64::NOTE_DECODED),
                language: NoteLanguage::Txt,
                content: text.clone(),
                warning: None,
            });
        }
        let rows = self.rows();
        let width = rows.iter().map(|(id, _)| id.len()).max()?;
        let content: Vec<String> = rows
            .iter()
            .map(|(id, value)| format!("{id:width$}  {value}"))
            .collect();
        Some(ToolResult {
            title: tr(keys::tools::base64::NOTE_ENCODED),
            language: NoteLanguage::Txt,
            content: content.join("\n"),
            warning: None,
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.direction = Direction::Encode;
        self.source = Source::Text;
        "Café".clone_into(&mut self.text);
    }

    fn clear(&mut self) {
        self.text.clear();
        self.path = None;
    }
}
