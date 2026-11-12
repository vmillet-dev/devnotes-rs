//! A text or a file hashed by every algorithm chosen, keyed or not, and a digest checked against
//! them. ⚠️ The HMAC key dies with the tool, and nothing in the result carries it.

use std::path::PathBuf;
use std::sync::Arc;

use egui::{CornerRadius, FontFamily, Margin, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::hash::{
    DigestEncoding, DigestShape, DigestValue, DigestVerdict, HashAlgorithm, HashAnswer, HashInput,
    HashRequest,
};
use i18n::{Key, Language, Message, keys};
use serde_json::json;
use zeroize::Zeroize;

use crate::api;
use crate::backend::Pending;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::ui::{
    ResultRow, card, check, columns, hint, label, named, note_import, problem, result_rows,
    section_head, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{self, Button, Segmented, TextField, address, hovered, interact, towards};

const ALGORITHMS: [HashAlgorithm; 6] = [
    HashAlgorithm::Md5,
    HashAlgorithm::Sha1,
    HashAlgorithm::Sha256,
    HashAlgorithm::Sha384,
    HashAlgorithm::Sha512,
    HashAlgorithm::Sha3_256,
];

fn algorithm_name(algorithm: HashAlgorithm) -> &'static str {
    match algorithm {
        HashAlgorithm::Md5 => "MD5",
        HashAlgorithm::Sha1 => "SHA-1",
        HashAlgorithm::Sha256 => "SHA-256",
        HashAlgorithm::Sha384 => "SHA-384",
        HashAlgorithm::Sha512 => "SHA-512",
        HashAlgorithm::Sha3_256 => "SHA3-256",
    }
}

fn hmac_name(algorithm: HashAlgorithm) -> &'static str {
    match algorithm {
        HashAlgorithm::Md5 => "HMAC-MD5",
        HashAlgorithm::Sha1 => "HMAC-SHA1",
        HashAlgorithm::Sha256 => "HMAC-SHA256",
        HashAlgorithm::Sha384 => "HMAC-SHA384",
        HashAlgorithm::Sha512 => "HMAC-SHA512",
        HashAlgorithm::Sha3_256 => "HMAC-SHA3-256",
    }
}

fn is_obsolete(algorithm: HashAlgorithm) -> bool {
    matches!(algorithm, HashAlgorithm::Md5 | HashAlgorithm::Sha1)
}

/// SHA-256 of `hello world`, so the check finds its algorithm.
const SAMPLE: (&str, &str) = (
    "hello world",
    "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Text,
    File,
}

pub(super) struct HashTool {
    source: Source,
    text: String,
    path: Option<String>,
    chosen: Vec<HashAlgorithm>,
    encoding: DigestEncoding,
    hmac: bool,
    key: String,
    key_shown: bool,
    expected: String,
    answer: Live<HashRequest, HashAnswer>,
    wants_file: bool,
    picking: Option<Pending<Option<PathBuf>>>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(HashTool {
        source: Source::Text,
        text: String::new(),
        path: None,
        chosen: ALGORITHMS.to_vec(),
        encoding: DigestEncoding::Hex,
        hmac: false,
        key: String::new(),
        key_shown: false,
        expected: String::new(),
        answer: Live::default(),
        wants_file: false,
        picking: None,
    })
}

/// « 64 hexadecimal characters: SHA-256 or SHA3-256 », once translated.
fn shape_words(shape: &DigestShape, language: Language) -> String {
    use keys::tools::hash::shapes;
    let or = i18n::translate(language, keys::tools::hash::OR, &[]);
    let (key, none, count, algorithms): (Key, Key, u32, &[HashAlgorithm]) = match shape {
        DigestShape::Hex {
            characters,
            algorithms,
        } => (shapes::HEX, shapes::HEX_NONE, *characters, algorithms),
        DigestShape::Base64 { bytes, algorithms } => {
            (shapes::BASE64, shapes::BASE64_NONE, *bytes, algorithms)
        }
        DigestShape::Unknown => return i18n::translate(language, shapes::UNKNOWN, &[]),
    };
    let names: Vec<&str> = algorithms.iter().map(|it| algorithm_name(*it)).collect();
    let key = if names.is_empty() { none } else { key };
    Message::new(key)
        .with("count", count)
        .with("names", names.join(&format!(" {or} ")))
        .text(language)
}

impl HashTool {
    fn input_of(&self) -> Option<HashInput> {
        match self.source {
            Source::File => self.path.clone().map(|path| HashInput::File { path }),
            Source::Text => (!self.text.is_empty()).then(|| HashInput::Text {
                text: self.text.clone(),
            }),
        }
    }

    /// A pasted digest alone is still asked about: Rust reads its shape.
    fn request(&self) -> Option<HashRequest> {
        let input = self.input_of();
        let expected = (!self.expected.trim().is_empty()).then(|| self.expected.clone());
        if input.is_none() && expected.is_none() {
            return None;
        }
        // Keyed only once there is a key: a switch on over an empty field hashes plainly.
        let keyed = input.is_some() && self.hmac && !self.key.is_empty();
        Some(HashRequest {
            input,
            algorithms: self.chosen.clone(),
            encoding: self.encoding,
            key: keyed.then(|| self.key.clone()),
            expected,
        })
    }

    fn name_of(&self, algorithm: HashAlgorithm) -> &'static str {
        let keyed = self.answer.answered().is_some_and(|it| it.key.is_some());
        if keyed {
            hmac_name(algorithm)
        } else {
            algorithm_name(algorithm)
        }
    }

    fn digests(&self) -> &[DigestValue] {
        match self.answer.value() {
            Some(HashAnswer::Hashed { digests, .. }) => digests,
            _ => &[],
        }
    }

    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let sources = [
            (Source::Text, tr(keys::tools::hash::sources::TEXT)),
            (Source::File, tr(keys::tools::hash::sources::FILE)),
        ];
        let sources = sources.each_ref().map(|(id, name)| (*id, name.as_str()));
        let source = &mut self.source;
        section_head(ui, &tr(keys::tools::INPUT), |ui| {
            Segmented::new(&sources, "hash-source").show(ui, source);
            if *source == Source::Text {
                note_import(ui, drawn, "text");
            }
        });
        match self.source {
            Source::Text => {
                let name = tr(keys::tools::hash::INPUT_LABEL);
                let placeholder = tr(keys::tools::hash::PLACEHOLDER);
                text_area(ui, &mut self.text, (&name, "hash-input"), &placeholder, 5);
            }
            Source::File => self.file(ui, language),
        }
        self.measure(ui, language);
        self.hmac_card(ui, language);
        self.verify(ui, language);
    }

    fn file(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let words = tr(keys::tools::base64::PICK_FILE);
            if Button::new(&words, "hash-pick-file").show(ui).clicked() {
                self.wants_file = true;
            }
            let shown = self.path.as_deref().map_or_else(
                || tr(keys::tools::hash::FILE_HINT),
                |path| path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned(),
            );
            hint(ui, &shown);
        });
    }

    fn measure(&self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        match self.answer.value() {
            Some(HashAnswer::Hashed {
                bytes,
                ends_with_newline,
                ..
            }) => {
                let count = Message::new(keys::tools::hash::BYTES)
                    .with("count", *bytes)
                    .text(language);
                let utf8 = if self.source == Source::Text {
                    " · UTF-8"
                } else {
                    ""
                };
                let newline = if *ends_with_newline {
                    tr(keys::tools::hash::WITH_NEWLINE)
                } else {
                    format!(
                        "{} {}",
                        tr(keys::tools::hash::WITHOUT_NEWLINE),
                        tr(keys::tools::hash::NO_NEWLINE_COMMAND)
                    )
                };
                let said = hint(ui, &format!("{count}{utf8} · {newline}"));
                address(ui, &said, "hash-measure");
            }
            Some(HashAnswer::Failed { problem: reason }) => {
                let said = named(language, "tools.files", &json!(reason));
                problem(ui, &said, "hash-problem");
            }
            _ => {}
        }
    }

    fn hmac_card(&mut self, ui: &mut Ui, language: Language) {
        let p = palette(ui.ctx());
        let tr = |key| i18n::translate(language, key, &[]);
        egui::Frame::new()
            .fill(p.bg_1)
            .stroke(Stroke::new(1.0, p.line_soft))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    crate::tools::ui::switch(ui, &mut self.hmac, "HMAC", "hash-hmac");
                    hint(ui, &tr(keys::tools::hash::KEY_NEVER_KEPT));
                });
                if self.hmac {
                    let name = tr(keys::tools::hash::KEY);
                    label(ui, &name);
                    TextField::new(&mut self.key, &name, "hash-key")
                        .password(!self.key_shown)
                        .show(ui);
                    let words = tr(keys::tools::hash::SHOW_KEY);
                    check(ui, &mut self.key_shown, &words, "hash-key-reveal");
                }
            });
    }

    fn verify(&mut self, ui: &mut Ui, language: Language) {
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(keys::tools::hash::VERIFY);
        label(ui, &name);
        let placeholder = tr(keys::tools::hash::EXPECTED_PLACEHOLDER);
        let field = text_area(
            ui,
            &mut self.expected,
            (&name, "hash-expected"),
            &placeholder,
            2,
        );
        if self.expected.trim().is_empty() {
            return;
        }
        let p = palette(ui.ctx());
        match self.answer.value() {
            Some(HashAnswer::Hashed {
                verdict:
                    Some(DigestVerdict::Matches {
                        algorithm,
                        encoding,
                    }),
                ..
            }) => {
                let encoding = named(language, "tools.hash.encodings", &json!(encoding));
                let words = Message::new(keys::tools::hash::MATCHES)
                    .with("name", self.name_of(*algorithm))
                    .with("encoding", encoding)
                    .text(language);
                // Over the field's own frame: what was pasted is what answered.
                ui.painter().rect_stroke(
                    field.rect.expand2(Vec2::new(14.0, 12.0)),
                    CornerRadius::same(10),
                    Stroke::new(1.0, p.green),
                    egui::StrokeKind::Inside,
                );
                let said = crate::tools::ui::pill(ui, &words, p.green);
                address(ui, &said, "hash-verdict");
            }
            Some(HashAnswer::Hashed {
                verdict: Some(DigestVerdict::NoMatch { shape }),
                ..
            }) => {
                let words = tr(keys::tools::hash::NO_MATCH);
                let said = crate::tools::ui::paragraph(ui, &words, (12.5, 500.0), p.red);
                address(ui, &said, "hash-verdict");
                let said = hint(ui, &shape_words(shape, language));
                address(ui, &said, "hash-shape");
                hint(ui, &tr(keys::tools::hash::NO_MATCH_HINTS));
            }
            Some(HashAnswer::Shaped { shape }) => {
                let said = hint(ui, &shape_words(shape, language));
                address(ui, &said, "hash-shape");
                hint(ui, &tr(keys::tools::hash::HMAC_SHAPE));
            }
            _ => {}
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            for algorithm in ALGORITHMS {
                let on = self.chosen.contains(&algorithm);
                if pill(ui, algorithm, on).clicked() {
                    self.toggle(algorithm);
                }
            }
        });
        let encodings = [
            (DigestEncoding::Hex, tr(keys::tools::hash::encodings::HEX)),
            (
                DigestEncoding::Base64,
                tr(keys::tools::hash::encodings::BASE64),
            ),
        ];
        let encodings = encodings.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&encodings, "hash-encoding").show(ui, &mut self.encoding);
        self.results(ui, language);
        if self.chosen.iter().any(|it| is_obsolete(*it)) {
            hint(ui, &tr(keys::tools::hash::WEAK));
        }
    }

    fn results(&self, ui: &mut Ui, language: Language) {
        let digests = self.digests();
        if digests.is_empty() {
            hint(
                ui,
                &i18n::translate(language, keys::tools::hash::EMPTY, &[]),
            );
            return;
        }
        let matched = match self.answer.value() {
            Some(HashAnswer::Hashed {
                verdict: Some(DigestVerdict::Matches { algorithm, .. }),
                ..
            }) => Some(*algorithm),
            _ => None,
        };
        let obsolete = i18n::translate(language, keys::tools::hash::OBSOLETE, &[]);
        let metas: Vec<String> = digests
            .iter()
            .map(|digest| {
                Message::new(keys::tools::hash::BITS)
                    .with("count", digest.bits)
                    .text(language)
            })
            .collect();
        card(ui, "hash-results", |ui| {
            let rows: Vec<_> = digests
                .iter()
                .zip(&metas)
                .map(|(digest, meta)| {
                    ResultRow::new(self.name_of(digest.algorithm), &digest.value)
                        .tag(is_obsolete(digest.algorithm).then_some(obsolete.as_str()))
                        .meta(meta)
                        .matched(matched == Some(digest.algorithm))
                })
                .collect();
            result_rows(ui, &rows, language);
        });
    }

    /// Kept in the order they are listed, whichever was chosen first.
    fn toggle(&mut self, algorithm: HashAlgorithm) {
        let on = self.chosen.contains(&algorithm);
        let chosen = &self.chosen;
        self.chosen = ALGORITHMS
            .into_iter()
            .filter(|it| {
                if *it == algorithm {
                    !on
                } else {
                    chosen.contains(it)
                }
            })
            .collect();
    }
}

fn pill(ui: &mut Ui, algorithm: HashAlgorithm, on: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let name = algorithm_name(algorithm);
    let ink = if on { p.amber_text } else { p.text_2 };
    let galley = widgets::text(ui, name, 11.5, 400.0, FontFamily::Monospace, ink);
    let corner = CornerRadius::same(12);
    let id = json!(algorithm);
    let id = id.as_str().unwrap_or_default();
    let response = interact(
        ui,
        galley.size() + Vec2::new(20.0, 8.0),
        Sense::click(),
        &format!("hash-algorithm-{id}"),
        || WidgetInfo::selected(Role::Button, true, on, name),
        corner,
    );
    let rect = response.rect;
    let edge = if on {
        p.amber_dim
    } else {
        towards(p.line_soft, p.amber_dim, hovered(ui, &response))
    };
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ink);
    response
}

impl Tool for HashTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        if std::mem::take(&mut self.wants_file) {
            let dialog = Arc::clone(turn.dialog);
            self.picking = Some(turn.backend.compute(move |_| Ok(dialog.open())));
        }
        if let Some(picking) = &mut self.picking
            && let Some(answer) = picking.poll()
        {
            self.picking = None;
            if let Ok(Some(path)) = answer {
                self.path = Some(path.display().to_string());
            }
        }
        let request = self.request();
        self.answer.update(turn, request, api::tools::hash);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let digests = self.digests();
        let width = digests
            .iter()
            .map(|digest| self.name_of(digest.algorithm).len())
            .max()?;
        let content: Vec<String> = digests
            .iter()
            .map(|it| format!("{:width$}  {}", self.name_of(it.algorithm), it.value))
            .collect();
        Some(ToolResult {
            title: i18n::translate(language, keys::tools::hash::NOTE_TITLE, &[]),
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
        self.source = Source::Text;
        SAMPLE.0.clone_into(&mut self.text);
        SAMPLE.1.clone_into(&mut self.expected);
    }

    fn clear(&mut self) {
        self.text.clear();
        self.path = None;
        self.expected.clear();
        self.key.zeroize();
    }

    fn left(&mut self) {
        self.key.zeroize();
        self.key_shown = false;
    }
}
