//! A token decoded and verified in Rust; how long ago or how far off is the page's, so it ages.
//! ⚠️ A token is a credential and a secret a secret: gone with the tool.

use chrono::{DateTime, Utc};
use egui::Ui;
use engine::notes::language::Language as NoteLanguage;
use engine::tools::jwt::{DatedClaim, JwtAnswer, JwtProblem, JwtRequest, TokenState, Verification};
use i18n::{Language, Message, keys};
use serde_json::json;
use zeroize::Zeroize;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    CopyValue, ResultRow, card, check, columns, hint, label, named, paragraph, problem,
    result_rows, section_head, section_title, text_area,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{TextField, address};

/// Signed with the sample's made-up key, « secret ».
const SAMPLE_JWT: &str = concat!(
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.",
    "eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkFkYSBMb3ZlbGFjZSIsInJvbGUiOiJhZG1pbiIsImlhdCI6MTc2NzIyNTYwMCwiZXhwIjo0MTAyNDQ0ODAwfQ.",
    "FPzqUtgp811f7rY_4UkLBOZ5GJrOJ-YNDba9VITtvXU"
);
const SAMPLE_KEY: &str = "secret";

#[derive(Default)]
pub(super) struct JwtTool {
    token: String,
    secret: String,
    secret_shown: bool,
    secret_is_base64: bool,
    answer: Live<JwtRequest, JwtAnswer>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::<JwtTool>::default()
}

/// « 3 days ago », from now.
fn span(claim: &DatedClaim, now: DateTime<Utc>) -> Option<Message> {
    #[expect(clippy::cast_possible_truncation, reason = "milliseconds of a date")]
    let at = DateTime::from_timestamp_millis(claim.epoch_milliseconds as i64)?;
    Some(i18n::time::span(at, now))
}

fn malformed_words(answer: &JwtAnswer, language: Language) -> Option<String> {
    use keys::tools::jwt::problems;
    let JwtAnswer::Malformed {
        problem: reason,
        segment,
        segments,
        at,
    } = answer
    else {
        return None;
    };
    let tr = |key| i18n::translate(language, key, &[]);
    let segment = named(language, "tools.jwt.segments", &json!(segment));
    Some(match (reason, at) {
        (JwtProblem::SegmentCount, _) if *segments == 5 => tr(problems::JWE),
        (JwtProblem::SegmentCount, _) => Message::new(problems::SEGMENT_COUNT)
            .with("count", *segments)
            .text(language),
        (JwtProblem::NotBase64, Some(at)) => Message::new(problems::NOT_BASE64_AT)
            .with("segment", segment)
            .with("at", *at)
            .text(language),
        (JwtProblem::NotBase64, None) => Message::new(problems::NOT_BASE64)
            .with("segment", segment)
            .text(language),
        (JwtProblem::NotJson, _) => Message::new(problems::NOT_JSON)
            .with("segment", segment)
            .text(language),
        (JwtProblem::NotObject, _) => Message::new(problems::NOT_OBJECT)
            .with("segment", segment)
            .text(language),
    })
}

/// A block of JSON, as it reads in the token.
fn json_block(ui: &mut Ui, text: &str, author_id: &str) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(egui::Stroke::new(1.0, p.line_soft))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let words = egui::RichText::new(text)
                .font(egui::FontId::monospace(12.0))
                .color(p.text_0);
            let shown = ui.add(egui::Label::new(words).selectable(true));
            address(ui, &shown, author_id);
        });
}

impl JwtTool {
    fn input(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::INPUT));
        let name = tr(keys::tools::jwt::TOKEN_LABEL);
        let placeholder = tr(keys::tools::jwt::TOKEN_PLACEHOLDER);
        text_area(ui, &mut self.token, (&name, "jwt-token"), &placeholder, 7);
        let name = tr(keys::tools::jwt::SECRET);
        label(ui, &name);
        TextField::new(&mut self.secret, &name, "jwt-secret")
            .hint(&tr(keys::tools::jwt::SECRET_PLACEHOLDER))
            .password(!self.secret_shown)
            .show(ui);
        let words = tr(keys::tools::jwt::SHOW);
        check(ui, &mut self.secret_shown, &words, "jwt-secret-reveal");
        let words = tr(keys::tools::jwt::SECRET_BASE64);
        check(ui, &mut self.secret_is_base64, &words, "jwt-secret-base64");
        hint(ui, &tr(keys::tools::jwt::FORGOTTEN));
        if let Some(words) = self
            .answer
            .value()
            .and_then(|it| malformed_words(it, language))
        {
            problem(ui, &words, "jwt-problem");
        }
    }

    fn output(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        let now = drawn.now;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(keys::tools::RESULT));
        let Some(JwtAnswer::Decoded {
            header,
            payload,
            signature,
            algorithm,
            family,
            dates,
            state,
            verification,
            ..
        }) = self.answer.value()
        else {
            if self.answer.value().is_none() {
                hint(ui, &tr(keys::tools::jwt::EMPTY));
            }
            return;
        };
        let p = palette(ui.ctx());
        let family_name = named(language, "tools.jwt.families", &json!(family));
        let mut words = named(language, "tools.jwt.states", &json!(state));
        let claim = match state {
            TokenState::Expired | TokenState::Valid => Some("exp"),
            TokenState::NotYetValid => Some("nbf"),
            TokenState::Undated => None,
        };
        let when = dates.iter().find(|it| Some(it.name.as_str()) == claim);
        if let Some(span) = when.and_then(|it| span(it, now)) {
            words = format!("{words} · {}", span.text(language));
        }
        let said = paragraph(ui, &words, (12.5, 600.0), p.text_0);
        address(ui, &said, "jwt-state");
        let ink = match verification {
            Verification::Valid => p.green,
            Verification::Invalid | Verification::Unsigned => p.red,
            _ => p.text_1,
        };
        let key = i18n::Key::find(&format!(
            "tools.jwt.verification.{}",
            json!(verification).as_str().unwrap_or_default()
        ));
        let verdict = key.map_or_else(String::new, |key| {
            Message::new(key)
                .with("family", family_name.as_str())
                .text(language)
        });
        let said = paragraph(ui, &verdict, (12.5, 400.0), ink);
        address(ui, &said, "jwt-verification");
        let algorithm = format!("{} · {family_name}", algorithm.as_deref().unwrap_or("—"));
        let title = tr(keys::tools::jwt::ALGORITHM);
        let names: Vec<String> = dates
            .iter()
            .map(|it| named(language, "tools.jwt.claims", &json!(it.name)))
            .collect();
        let values: Vec<String> = dates
            .iter()
            .map(|it| match span(it, now) {
                Some(span) => format!("{}  {}", it.iso, span.text(language)),
                None => it.iso.clone(),
            })
            .collect();
        card(ui, "jwt-facts", |ui| {
            let mut rows = vec![ResultRow::new(&title, &algorithm).reading()];
            for (name, value) in names.iter().zip(&values) {
                rows.push(ResultRow::new(name, value).reading());
            }
            result_rows(ui, &rows, language);
        });
        for (key, text, id) in [
            (keys::tools::jwt::HEADER, header, "jwt-header"),
            (keys::tools::jwt::PAYLOAD, payload, "jwt-payload"),
        ] {
            let name = tr(key);
            section_head(ui, &name, |ui| {
                CopyValue::new(text, &name, language).show(ui);
            });
            json_block(ui, text, id);
        }
        section_title(ui, &tr(keys::tools::jwt::SIGNATURE));
        json_block(ui, signature, "jwt-signature");
    }
}

impl Tool for JwtTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = (!self.token.trim().is_empty()).then(|| JwtRequest {
            token: self.token.clone(),
            secret: self.secret.clone(),
            secret_is_base64: self.secret_is_base64,
        });
        self.answer.update(turn, request, api::tools::decode_jwt);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::input, Self::output));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let Some(JwtAnswer::Decoded {
            document,
            algorithm,
            ..
        }) = self.answer.value()
        else {
            return None;
        };
        Some(ToolResult {
            title: Message::new(keys::tools::jwt::NOTE_TITLE)
                .with("algorithm", algorithm.as_deref().unwrap_or_default())
                .text(language),
            language: NoteLanguage::Json,
            content: document.clone(),
            warning: Some(keys::tools::jwt::WARNING),
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        SAMPLE_JWT.clone_into(&mut self.token);
        SAMPLE_KEY.clone_into(&mut self.secret);
        self.secret_is_base64 = false;
    }

    fn clear(&mut self) {
        self.left();
    }

    fn left(&mut self) {
        self.token.zeroize();
        self.secret.zeroize();
        self.secret_shown = false;
        self.answer = Live::default();
    }
}
