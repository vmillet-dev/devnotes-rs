//! A payload from a form, its code drawn by Rust. ⚠️ The Wi-Fi password dies with the tool, and a
//! note keeping it is warned against.

use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, ColorImage, CornerRadius, Margin, TextureHandle, TextureOptions, Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::qr::{
    QrAnswer, QrContent, QrCorrection, QrFormat, QrRequest, SavedCode, WifiSecurity,
};
use i18n::{Language, Message, keys};
use serde_json::json;
use zeroize::Zeroize;

use crate::api;
use crate::backend::Pending;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    ResultRow, card, check, columns, hint, label, line, note_import, notice, number_field, problem,
    result_rows, section_title, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Kind, Segmented, TextField, address};

const MAX_MARGIN: u32 = 16;
/// The preview's side, at most.
const PLATE: f32 = 320.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ContentKind {
    Text,
    Url,
    Wifi,
    Email,
    Contact,
}

/// The file asked for: where it goes, then its writing.
enum Saving {
    Choosing(QrFormat, Pending<Option<PathBuf>>),
    Writing(Pending<SavedCode>),
}

/// What is typed, a field a form.
#[derive(Default)]
struct Forms {
    text: String,
    url: String,
    ssid: String,
    password: String,
    to: String,
    subject: String,
    body: String,
    name: String,
    phone: String,
    email: String,
    organisation: String,
}

pub(super) struct QrTool {
    kind: ContentKind,
    forms: Forms,
    security: WifiSecurity,
    hidden: bool,
    correction: QrCorrection,
    margin: String,
    answer: Live<QrRequest, api::tools::QrDrawing>,
    texture: Option<(QrRequest, TextureHandle)>,
    wants_file: Option<QrFormat>,
    wants_copy: bool,
    saving: Option<Saving>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(QrTool {
        kind: ContentKind::Url,
        forms: Forms::default(),
        security: WifiSecurity::Wpa,
        hidden: false,
        correction: QrCorrection::Medium,
        margin: "4".to_owned(),
        answer: Live::default(),
        texture: None,
        wants_file: None,
        wants_copy: false,
        saving: None,
    })
}

fn extension(format: QrFormat) -> &'static str {
    match format {
        QrFormat::Svg => "svg",
        QrFormat::Png => "png",
    }
}

fn image_of(side: u32, rgba: &[u8]) -> Option<ColorImage> {
    let side = usize::try_from(side).ok()?;
    Some(ColorImage::from_rgba_unmultiplied([side, side], rgba))
}

impl QrTool {
    fn content(&self) -> QrContent {
        let forms = &self.forms;
        match self.kind {
            ContentKind::Text => QrContent::Text {
                text: forms.text.clone(),
            },
            ContentKind::Url => QrContent::Url {
                url: forms.url.clone(),
            },
            ContentKind::Wifi => QrContent::Wifi {
                ssid: forms.ssid.clone(),
                password: forms.password.clone(),
                security: self.security,
                hidden: self.hidden,
            },
            ContentKind::Email => QrContent::Email {
                to: forms.to.clone(),
                subject: forms.subject.clone(),
                body: forms.body.clone(),
            },
            ContentKind::Contact => QrContent::Contact {
                name: forms.name.clone(),
                phone: forms.phone.clone(),
                email: forms.email.clone(),
                organisation: forms.organisation.clone(),
            },
        }
    }

    fn request(&self) -> Option<QrRequest> {
        let margin = self.margin.trim().parse::<u32>().ok()?.min(MAX_MARGIN);
        Some(QrRequest {
            content: self.content(),
            correction: self.correction,
            margin,
        })
    }

    fn answered(&self) -> Option<&QrAnswer> {
        self.answer.value().map(|(answer, _)| answer)
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::qr as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::CONTENT));
        let kinds = [
            (ContentKind::Text, tr(words::kinds::TEXT)),
            (ContentKind::Url, tr(words::kinds::URL)),
            (ContentKind::Wifi, tr(words::kinds::WIFI)),
            (ContentKind::Email, tr(words::kinds::EMAIL)),
            (ContentKind::Contact, tr(words::kinds::CONTACT)),
        ];
        let kinds = kinds.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&kinds, "qr-kind").show(ui, &mut self.kind);
        match self.kind {
            ContentKind::Text => {
                let name = tr(words::fields::TEXT);
                let hint_words = tr(words::placeholders::TEXT);
                text_area(ui, &mut self.forms.text, (&name, "qr-text"), &hint_words, 4);
                note_import(ui, drawn, "text");
            }
            ContentKind::Url => self.url_form(ui, language),
            ContentKind::Wifi => self.wifi_form(ui, language),
            ContentKind::Email => self.email_form(ui, language),
            ContentKind::Contact => self.contact_form(ui, language),
        }
        self.options(ui, language);
        self.facts(ui, language);
        notice(ui, &tr(words::ADVICE), "qr-advice");
    }

    fn url_form(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::qr as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::fields::URL);
        TextField::new(&mut self.forms.url, &name, "qr-url")
            .hint(&tr(words::placeholders::URL))
            .show(ui);
        if matches!(
            self.answered(),
            Some(QrAnswer::Code {
                missing_scheme: true,
                ..
            })
        ) {
            let said = hint(ui, &tr(words::MISSING_SCHEME));
            address(ui, &said, "qr-missing-scheme");
        }
    }

    fn wifi_form(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::qr as words;
        let tr = |key| i18n::translate(language, key, &[]);
        labelled(
            ui,
            &tr(words::fields::SSID),
            &mut self.forms.ssid,
            "qr-ssid",
        );
        if self.security != WifiSecurity::Open {
            let name = tr(words::fields::PASSWORD);
            label(ui, &name);
            TextField::new(&mut self.forms.password, &name, "qr-password")
                .password(true)
                .show(ui);
        }
        let options = [
            (WifiSecurity::Wpa, tr(words::securities::WPA)),
            (WifiSecurity::Wep, tr(words::securities::WEP)),
            (WifiSecurity::Open, tr(words::securities::OPEN)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            Segmented::new(&options, "qr-security").show(ui, &mut self.security);
            let words = tr(words::fields::HIDDEN);
            check(ui, &mut self.hidden, &words, "qr-hidden");
        });
    }

    fn email_form(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::qr::fields;
        let tr = |key| i18n::translate(language, key, &[]);
        labelled(ui, &tr(fields::TO), &mut self.forms.to, "qr-to");
        labelled(
            ui,
            &tr(fields::SUBJECT),
            &mut self.forms.subject,
            "qr-subject",
        );
        let name = tr(fields::BODY);
        label(ui, &name);
        text_area(ui, &mut self.forms.body, (&name, "qr-body"), "", 3);
    }

    fn contact_form(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::qr::fields;
        let tr = |key| i18n::translate(language, key, &[]);
        labelled(ui, &tr(fields::NAME), &mut self.forms.name, "qr-name");
        labelled(ui, &tr(fields::PHONE), &mut self.forms.phone, "qr-phone");
        labelled(ui, &tr(fields::EMAIL), &mut self.forms.email, "qr-email");
        let name = tr(fields::ORGANISATION);
        labelled(ui, &name, &mut self.forms.organisation, "qr-organisation");
    }

    fn options(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::qr as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (QrCorrection::Low, tr(words::levels::LOW)),
            (QrCorrection::Medium, tr(words::levels::MEDIUM)),
            (QrCorrection::Quartile, tr(words::levels::QUARTILE)),
            (QrCorrection::High, tr(words::levels::HIGH)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        line(ui, |ui| {
            Segmented::new(&options, "qr-correction").show(ui, &mut self.correction);
            let name = tr(words::MARGIN);
            label(ui, &name);
            number_field(ui, &mut self.margin, &name, "qr-margin");
        });
    }

    fn facts(&self, ui: &mut Ui, language: Language) {
        use keys::tools::qr as words;
        let tr = |key| i18n::translate(language, key, &[]);
        match self.answered() {
            Some(QrAnswer::Code {
                version,
                modules,
                bytes,
                capacity,
                recoverable,
                ..
            }) => {
                let value = Message::new(words::VERSION_VALUE)
                    .with("version", *version)
                    .with("modules", *modules)
                    .text(language);
                let room = Message::new(words::CAPACITY_VALUE)
                    .with("bytes", *bytes)
                    .with("capacity", *capacity)
                    .text(language);
                let spare = Message::new(words::RECOVERABLE_VALUE)
                    .with("percent", *recoverable)
                    .text(language);
                let names = (
                    tr(words::VERSION),
                    tr(words::CAPACITY),
                    tr(words::RECOVERABLE),
                );
                card(ui, "qr-facts", |ui| {
                    let rows = [
                        ResultRow::new(&names.0, &value).reading(),
                        ResultRow::new(&names.1, &room).reading(),
                        ResultRow::new(&names.2, &spare).reading(),
                    ];
                    result_rows(ui, &rows, language);
                });
            }
            Some(QrAnswer::TooLong { bytes, maximum }) => {
                let said = Message::new(words::TOO_LONG)
                    .with("bytes", *bytes)
                    .with("maximum", *maximum)
                    .text(language);
                problem(ui, &said, "qr-too-long");
            }
            _ => {}
        }
    }

    fn preview(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::qr as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::PREVIEW));
        let code = matches!(self.answered(), Some(QrAnswer::Code { .. }));
        self.plate(ui, code, language);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let mut button = |key, id, kind| {
                let words = tr(key);
                Button::new(&words, id)
                    .kind(kind)
                    .enabled(code)
                    .show(ui)
                    .clicked()
            };
            if button(words::DOWNLOAD_SVG, "qr-svg", Kind::Filled) {
                self.wants_file = Some(QrFormat::Svg);
            }
            if button(words::PNG, "qr-png", Kind::Outline) {
                self.wants_file = Some(QrFormat::Png);
            }
            if button(words::COPY_IMAGE, "qr-copy", Kind::Outline) {
                self.wants_copy = true;
            }
        });
    }

    /// Black on white in both themes: an inverted code is not read by every scanner.
    fn plate(&self, ui: &mut Ui, code: bool, language: Language) {
        let p = palette(ui.ctx());
        let fill = if code { Color32::WHITE } else { p.bg_1 };
        let shown = egui::Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                let side = ui.available_width().min(PLATE);
                if let Some((_, texture)) = self.texture.as_ref().filter(|_| code) {
                    let sized = (texture.id(), Vec2::splat(side));
                    let drawn = ui.add(egui::Image::new(sized));
                    address(ui, &drawn, "qr-code");
                } else {
                    let words = keys::tools::qr::NOTHING;
                    hint(ui, &i18n::translate(language, words, &[]));
                }
            });
        address(ui, &shown.response, "qr-preview");
    }

    fn copy_image(&self, turn: &mut Turn<'_>) {
        let Some((_, Some((side, rgba)))) = self.answer.value() else {
            return;
        };
        if let Some(image) = image_of(*side, rgba) {
            turn.ctx.copy_image(image);
            turn.status.report(Message::new(keys::tools::qr::COPIED));
        }
    }

    fn follow_file(&mut self, turn: &mut Turn<'_>) {
        if let Some(format) = self.wants_file.take() {
            let dialog = Arc::clone(turn.dialog);
            let name = format!("qr-code.{}", extension(format));
            let pending = turn.backend.compute(move |_| Ok(dialog.save(&name)));
            self.saving = Some(Saving::Choosing(format, pending));
        }
        match self.saving.as_mut() {
            Some(Saving::Choosing(format, pending)) => {
                let format = *format;
                let Some(answer) = pending.poll() else { return };
                self.saving = None;
                // The code on screen, not the one being typed: the file is the preview.
                if let (Ok(Some(path)), Some(request)) = (answer, self.answer.answered()) {
                    let path = path.display().to_string();
                    let saved =
                        api::tools::save_qr_code(turn.backend, request.clone(), format, path);
                    self.saving = Some(Saving::Writing(saved));
                }
            }
            Some(Saving::Writing(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.saving = None;
                match answer {
                    Ok(SavedCode::Saved { bytes }) => turn
                        .status
                        .report(Message::new(keys::tools::qr::SAVED).with("count", bytes)),
                    Ok(SavedCode::Failed { problem: reason }) => {
                        let path = format!("tools.files.{}", json!(reason).as_str().unwrap_or(""));
                        if let Some(key) = i18n::Key::find(&path) {
                            turn.errors.notify(Message::new(key));
                        }
                    }
                    Ok(SavedCode::Nothing) => {}
                    Err(error) => turn.errors.report(keys::errors::TOOL_FAILED, &error),
                }
            }
            None => {}
        }
    }

    /// Nearest: a module is a square, and stays one at any size.
    fn follow_texture(&mut self, turn: &Turn<'_>) {
        let (Some((_, pixels)), Some(asked)) = (self.answer.value(), self.answer.answered()) else {
            self.texture = None;
            return;
        };
        if self
            .texture
            .as_ref()
            .is_some_and(|(drawn, _)| drawn == asked)
        {
            return;
        }
        let image = pixels
            .as_ref()
            .and_then(|(side, rgba)| image_of(*side, rgba));
        self.texture = image.map(|image| {
            let texture = turn
                .ctx
                .load_texture("qr-code", image, TextureOptions::NEAREST);
            (asked.clone(), texture)
        });
    }
}

fn labelled(ui: &mut Ui, name: &str, value: &mut String, author_id: &str) {
    label(ui, name);
    TextField::new(value, name, author_id).show(ui);
}

impl Tool for QrTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        self.follow_file(turn);
        if std::mem::take(&mut self.wants_copy) {
            self.copy_image(turn);
        }
        let request = self.request();
        self.answer.update(turn, request, api::tools::draw_qr_code);
        self.follow_texture(turn);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::preview));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let Some(QrAnswer::Code { payload, .. }) = self.answered() else {
            return None;
        };
        let secret = self.answer.answered().is_some_and(|asked| {
            matches!(
                &asked.content,
                QrContent::Wifi { password, security, .. }
                    if *security != WifiSecurity::Open && !password.is_empty()
            )
        });
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::qr::NOTE_TITLE, &[]),
            language: NoteLanguage::Txt,
            content: payload.clone(),
            warning: secret.then_some(keys::tools::qr::SAVE_WARNING),
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.forms.text.clone_from(&note.content);
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.kind = ContentKind::Url;
        "https://exemple.fr/doc".clone_into(&mut self.forms.url);
    }

    fn clear(&mut self) {
        self.left();
        self.forms = Forms::default();
    }

    fn left(&mut self) {
        self.forms.password.zeroize();
    }
}
