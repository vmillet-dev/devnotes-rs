//! Identifiers drawn in Rust, and one pasted read back: its kind, its instant, what is random.

use chrono::{DateTime, Local, Utc};
use egui::text::LayoutJob;
use egui::{Key, Modifiers, Role, Sense, TextFormat, Ui, Vec2, WidgetInfo};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::identifiers::{
    IdInspection, IdKind, IdentifiersRequest, NanoAlphabet, RandomPart,
};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    CopyValue, ResultRow, card, check, hint, label, named, number_field, problem, result_rows,
    section_title,
};
use crate::tools::{Drawn, Tool, ToolResult};
use crate::widgets::{Button, Kind, Segmented, TextField, address, virtual_list};

const KINDS: [(IdKind, &str); 7] = [
    (IdKind::UuidV4, "UUID v4"),
    (IdKind::UuidV7, "UUID v7"),
    (IdKind::Ulid, "ULID"),
    (IdKind::NanoId, "NanoID"),
    (IdKind::Cuid2, "CUID2"),
    (IdKind::ObjectId, "ObjectId"),
    (IdKind::Ksuid, "KSUID"),
];

const ROW: f32 = 30.0;
const LIST_MAX: f32 = 300.0;
const COUNT_BOUNDS: (u32, u32) = (1, 500);

/// The ULID of its own specification: the instant it carries is read back.
const SAMPLE_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// A new draw with the same options is part of what is asked.
type Asked = (IdentifiersRequest, u32);

pub(super) struct IdentifiersTool {
    kind: IdKind,
    count: String,
    uppercase: bool,
    without_hyphens: bool,
    nano_length: String,
    nano_alphabet: NanoAlphabet,
    checked: String,
    draw: u32,
    generated: Live<Asked, Vec<String>>,
    inspection: Live<String, IdInspection>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(IdentifiersTool {
        kind: IdKind::UuidV4,
        count: "5".to_owned(),
        uppercase: false,
        without_hyphens: false,
        nano_length: "21".to_owned(),
        nano_alphabet: NanoAlphabet::UrlSafe,
        checked: String::new(),
        draw: 0,
        generated: Live::default(),
        inspection: Live::default(),
    })
}

fn name_of(kind: IdKind) -> &'static str {
    KINDS
        .iter()
        .find(|(it, _)| *it == kind)
        .map_or("", |(_, name)| name)
}

/// The kinds whose letters carry no meaning in their case: hexadecimal and Crockford's base 32.
fn is_caseless(kind: IdKind) -> bool {
    matches!(
        kind,
        IdKind::UuidV4 | IdKind::UuidV7 | IdKind::Ulid | IdKind::ObjectId
    )
}

fn is_uuid(kind: IdKind) -> bool {
    matches!(kind, IdKind::UuidV4 | IdKind::UuidV7)
}

/// A UUID's hyphens dimmed and its version digit picked out. Presentation only: the 8-4-4-4-12
/// layout is fixed, so the digit is the 13th hexadecimal one with or without the hyphens.
fn uuid_job(ui: &Ui, id: &str) -> LayoutJob {
    let p = palette(ui.ctx());
    let version = if id.contains('-') { 14 } else { 12 };
    let mut job = LayoutJob::default();
    let format = |ink| TextFormat::simple(egui::FontId::monospace(12.5), ink);
    for (at, character) in id.char_indices() {
        let ink = match character {
            '-' => p.text_2,
            _ if at == version => p.amber_text,
            _ => p.text_0,
        };
        job.append(&character.to_string(), 0.0, format(ink));
    }
    job
}

/// « 2024-09-25 23:05:01.488 UTC »: the instant as Rust wrote it, read in any language.
fn readable(iso: &str) -> String {
    iso.replace('T', " ").replace('Z', " UTC")
}

/// « 2016-07-31 01:54:10.259 (UTC+02:00) »: the same instant on this machine's clock.
fn local_reading(iso: &str) -> Option<String> {
    let at: DateTime<Utc> = iso.parse().ok()?;
    let local = at.with_timezone(&Local);
    Some(local.format("%Y-%m-%d %H:%M:%S%.3f (UTC%:z)").to_string())
}

fn id_row(ui: &mut Ui, id: &str, uuid: bool, language: Language) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, id));
    address(ui, &response, "identifiers-row");
    let inner = rect.shrink2(Vec2::new(14.0, 0.0));
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id)
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let job = if uuid {
        uuid_job(&row, id)
    } else {
        let p = palette(row.ctx());
        LayoutJob::simple_singleline(id.to_owned(), egui::FontId::monospace(12.5), p.text_0)
    };
    row.add(egui::Label::new(job).selectable(true));
    row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        CopyValue::new(id, id, language).compact().show(ui);
    });
}

fn facts(ui: &mut Ui, inspection: &IdInspection, text: &str, language: Language) {
    use keys::tools::identifiers as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let id = json!(inspection);
    let id = id.get("kind").cloned().unwrap_or_default();
    let recognised = named(language, "tools.identifiers.recognised", &id);
    let path = format!(
        "tools.identifiers.formats.{}",
        id.as_str().unwrap_or_default()
    );
    let characters = u32::try_from(text.trim().chars().count()).unwrap_or(u32::MAX);
    let format = i18n::Key::find(&path).map_or_else(String::new, |key| {
        Message::new(key).with("count", characters).text(language)
    });
    let mut rows = vec![(tr(words::FORMAT), format!("{recognised} · {format}"))];
    if let IdInspection::Uuid {
        version,
        variant,
        nil,
        ..
    } = inspection
    {
        let shown = if *nil {
            tr(words::NIL)
        } else {
            format!("v{version}")
        };
        rows.push((tr(words::VERSION), shown));
        let shown = named(language, "tools.identifiers.variants", &json!(variant));
        rows.push((tr(words::VARIANT), shown));
    }
    let (created, random): (Option<&str>, Option<&RandomPart>) = match inspection {
        IdInspection::Uuid {
            created, random, ..
        } => (created.as_deref(), random.as_ref()),
        IdInspection::Ulid { created, random }
        | IdInspection::ObjectId {
            created, random, ..
        }
        | IdInspection::Ksuid { created, random } => (Some(created), Some(random)),
        IdInspection::CuidV1 { created } => (Some(created), None),
        _ => (None, None),
    };
    if let Some(created) = created {
        rows.push((tr(words::CREATED), readable(created)));
        if let Some(local) = local_reading(created) {
            rows.push((tr(words::CREATED_LOCALLY), local));
        }
    }
    if let Some(random) = random {
        let shown = Message::new(words::RANDOM_PART)
            .with("text", random.text.as_str())
            .with("bits", random.bits)
            .text(language);
        rows.push((tr(words::RANDOM), shown));
    }
    if let IdInspection::ObjectId { counter, .. } = inspection {
        rows.push((tr(words::COUNTER), counter.to_string()));
    }
    card(ui, "identifiers-facts", |ui| {
        let lines: Vec<_> = rows
            .iter()
            .map(|(name, value)| ResultRow::new(name, value).reading())
            .collect();
        result_rows(ui, &lines, language);
    });
}

impl IdentifiersTool {
    fn request(&self) -> Option<Asked> {
        let request = IdentifiersRequest {
            kind: self.kind,
            count: self.count.trim().parse().ok()?,
            uppercase: self.uppercase,
            hyphens: !self.without_hyphens,
            nano_length: self.nano_length.trim().parse().ok()?,
            nano_alphabet: self.nano_alphabet,
        };
        Some((request, self.draw))
    }

    fn step(&mut self, by: i32) {
        let count = self.count.trim().parse().unwrap_or(COUNT_BOUNDS.0);
        let next = count.saturating_add_signed(by);
        self.count = next.clamp(COUNT_BOUNDS.0, COUNT_BOUNDS.1).to_string();
    }

    fn controls(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::identifiers as words;
        let tr = |key| i18n::translate(language, key, &[]);
        Segmented::new(&KINDS, "identifiers-kind").show(ui, &mut self.kind);
        let id = json!(self.kind);
        let path = format!(
            "tools.identifiers.hints.{}",
            id.as_str().unwrap_or_default()
        );
        let about = i18n::Key::find(&path).map_or_else(String::new, |key| {
            Message::new(key)
                .with("name", name_of(self.kind))
                .text(language)
        });
        let said = hint(ui, &about);
        address(ui, &said, "identifiers-hint");
        if self.kind == IdKind::NanoId {
            self.nano(ui, language);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if Button::new("-", "identifiers-fewer").show(ui).clicked() {
                self.step(-1);
            }
            let name = tr(words::COUNT);
            number_field(ui, &mut self.count, &name, "identifiers-count");
            if Button::new("+", "identifiers-more").show(ui).clicked() {
                self.step(1);
            }
            if is_caseless(self.kind) {
                let words = tr(words::UPPERCASE);
                check(ui, &mut self.uppercase, &words, "identifiers-uppercase");
            }
            if is_uuid(self.kind) {
                let words = tr(words::WITHOUT_HYPHENS);
                check(
                    ui,
                    &mut self.without_hyphens,
                    &words,
                    "identifiers-without-hyphens",
                );
            }
            let again = format!("{} (R)", tr(words::AGAIN));
            if Button::new(&again, "identifiers-generate")
                .kind(Kind::Outline)
                .show(ui)
                .clicked()
            {
                self.draw += 1;
            }
        });
    }

    fn nano(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::identifiers as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let options = [
            (NanoAlphabet::UrlSafe, tr(words::alphabets::URL_SAFE)),
            (
                NanoAlphabet::Alphanumeric,
                tr(words::alphabets::ALPHANUMERIC),
            ),
            (NanoAlphabet::HexLower, tr(words::alphabets::HEX_LOWER)),
            (NanoAlphabet::Digits, tr(words::alphabets::DIGITS)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let name = tr(words::LENGTH);
            label(ui, &name);
            number_field(ui, &mut self.nano_length, &name, "identifiers-nano-length");
            Segmented::new(&options, "identifiers-alphabet").show(ui, &mut self.nano_alphabet);
        });
    }

    fn list(&self, ui: &mut Ui, language: Language) {
        use keys::tools::identifiers as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let Some(ids) = self.generated.value() else {
            return;
        };
        let uuid = self
            .generated
            .answered()
            .is_some_and(|(it, _)| is_uuid(it.kind));
        let count = u32::try_from(ids.len()).unwrap_or(u32::MAX);
        let counted = Message::new(words::COUNTED)
            .with("count", count)
            .text(language);
        let all = ids.join("\n");
        crate::tools::ui::section_head(ui, &counted, |ui| {
            CopyValue::new(&all, &tr(words::ALL), language)
                .label(keys::tools::COPY_ALL)
                .show(ui);
        });
        #[expect(clippy::cast_precision_loss, reason = "rows on a screen")]
        let height = (ids.len() as f32 * ROW).min(LIST_MAX);
        card(ui, "identifiers-list", |ui| {
            ui.set_height(height);
            virtual_list(ui, "identifiers-ids", ROW, ids.len(), |ui, at| {
                id_row(ui, &ids[at], uuid, language);
            });
        });
    }

    fn check_side(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::identifiers as words;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::CHECK));
        hint(ui, &tr(words::CHECK_HINT));
        let name = tr(words::CHECK_LABEL);
        TextField::new(&mut self.checked, &name, "identifiers-checked")
            .hint(&tr(words::CHECK_PLACEHOLDER))
            .show(ui);
        match self.inspection.value() {
            Some(IdInspection::Possible { kinds }) => {
                let names: Vec<&str> = kinds.iter().map(|it| name_of(*it)).collect();
                let said = Message::new(words::POSSIBLE)
                    .with("kinds", names.join(" / "))
                    .text(language);
                let said = hint(ui, &said);
                address(ui, &said, "identifiers-possible");
            }
            Some(IdInspection::OutOfRange { id }) => {
                let said = Message::new(words::OUT_OF_RANGE)
                    .with("kind", name_of(*id))
                    .text(language);
                problem(ui, &said, "identifiers-out-of-range");
            }
            Some(IdInspection::Unrecognised) => {
                problem(ui, &tr(words::UNRECOGNISED), "identifiers-unrecognised");
            }
            Some(inspection) => {
                let text = self.inspection.answered().map_or("", String::as_str);
                facts(ui, inspection, text, language);
            }
            None => {}
        }
    }
}

impl Tool for IdentifiersTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        let request = self.request();
        self.generated.update(turn, request, |backend, (asked, _)| {
            api::tools::generate_identifiers(backend, asked)
        });
        let read = (!self.checked.trim().is_empty()).then(|| self.checked.clone());
        self.inspection
            .update(turn, read, api::tools::inspect_identifier);
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let language = drawn.language;
        egui::ScrollArea::vertical()
            .id_salt("identifiers")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                self.controls(ui, language);
                self.list(ui, language);
                ui.add_space(8.0);
                self.check_side(ui, language);
            });
        // Never while typing: an `r` in a field is an `r`.
        let free = !ui.ctx().egui_wants_keyboard_input();
        if free && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::R)) {
            self.draw += 1;
        }
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let ids = self.generated.value().filter(|it| !it.is_empty())?;
        let (asked, _) = self.generated.answered()?;
        Some(ToolResult {
            title: Message::new(keys::tools::identifiers::NOTE_TITLE)
                .with("kind", name_of(asked.kind))
                .text(language),
            language: NoteLanguage::Txt,
            content: ids.join("\n"),
            warning: None,
        })
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        self.kind = IdKind::UuidV7;
        "5".clone_into(&mut self.count);
        SAMPLE_ID.clone_into(&mut self.checked);
    }

    fn clear(&mut self) {
        self.checked.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instant_reads_in_any_language() {
        assert_eq!(
            readable("2024-09-25T23:05:01.488Z"),
            "2024-09-25 23:05:01.488 UTC"
        );
    }

    #[test]
    fn what_is_no_instant_has_no_local_reading() {
        assert!(local_reading("yesterday").is_none());
        assert!(local_reading("2024-09-25T23:05:01.488Z").is_some());
    }
}
