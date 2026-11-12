//! A person drawn by Rust, every number computed there; this draws the card. ⚠️ It evokes an
//! identity card and imitates none: a monogram where a photo would be, no emblem, no flag, no
//! machine-readable zone. Nothing here is a secret: the person is an output.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate};
use egui::{CornerRadius, FontId, Margin, Sense, Stroke, Ui, Vec2};
use engine::notes::language::Language as NoteLanguage;
use engine::tools::identity::{
    AGES, Country, CsvSeparator, Identifier, IdentifierKind, Identity, IdentityAnswer,
    IdentityOutcome, IdentityRequest, MAX_PEOPLE, PeopleBatch, PeopleExport, PeopleFormat, Sex,
    UnreadableList,
};
use i18n::{Language, Message, keys};
use serde_json::json;

use crate::api;
use crate::backend::Pending;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::tools::live::{Live, Turn};
use crate::tools::ui::{
    CopyValue, ResultRow, card, columns, group_title, hint, label, line, named, number_field,
    paragraph, problem, result_rows, section_head, section_title,
};
use crate::tools::{Action, Drawn, Tool, ToolAction, ToolResult};
use crate::widgets::{
    self, Button, Chip, ChoiceMenu, IconButton, Segmented, TextField, address, tooltip,
};

/// What the options are back to on « Clear », and what a first visit opens on.
const AGES_AT_FIRST: (u32, u32) = (18, 80);
const PEOPLE_AT_FIRST: u32 = 10;
/// The seed a sample draws from: the same person for whoever opens the tool.
const SAMPLE_SEED: &str = "devnotes";
const MONOGRAM: f32 = 104.0;
/// A compact copy button and the gap before it.
const COPY_ROOM: f32 = 36.0;
const FACT_GAP: f32 = 10.0;

/// The request, and which draw of it: a new draw with the same options is a new request.
type Asked = (IdentityRequest, u32);

/// What a developer does differently with the value, said on its row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    MayRing,
    Unkeyed,
}

impl Mark {
    fn word(self) -> i18n::Key {
        match self {
            Self::MayRing => keys::tools::identity::marks::MAY_RING,
            Self::Unkeyed => keys::tools::identity::marks::UNKEYED,
        }
    }

    fn reason(self) -> i18n::Key {
        match self {
            Self::MayRing => keys::tools::identity::marks::MAY_RING_TITLE,
            Self::Unkeyed => keys::tools::identity::marks::UNKEYED_TITLE,
        }
    }
}

struct Row {
    name: String,
    value: String,
    mark: Option<Mark>,
}

impl Row {
    fn new(language: Language, name: i18n::Key, value: &str) -> Self {
        Self {
            name: i18n::translate(language, name, &[]),
            value: value.to_owned(),
            mark: None,
        }
    }

    fn marked(mut self, mark: Option<Mark>) -> Self {
        self.mark = mark;
        self
    }
}

/// The batch asked for: where it goes, then its writing.
enum Exporting {
    Choosing(Pending<Option<PathBuf>>),
    Writing(Pending<PeopleExport>),
}

pub(super) struct IdentityTool {
    country: Country,
    /// `None` draws it.
    sex: Option<Sex>,
    age_min: String,
    age_max: String,
    /// Empty draws one: the answer says which, and the pin writes it here.
    seed: String,
    people: String,
    format: PeopleFormat,
    /// Kept with the application: a spreadsheet's separator does not change.
    separator: CsvSeparator,
    /// What a batch adds to the card's own country, which is always in it.
    batch: Vec<Country>,
    draw: u32,
    answer: Live<Asked, IdentityOutcome>,
    wants_export: bool,
    exporting: Option<Exporting>,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(IdentityTool {
        country: Country::Fr,
        sex: None,
        age_min: AGES_AT_FIRST.0.to_string(),
        age_max: AGES_AT_FIRST.1.to_string(),
        seed: String::new(),
        people: PEOPLE_AT_FIRST.to_string(),
        format: PeopleFormat::Json,
        separator: CsvSeparator::Comma,
        batch: Vec::new(),
        draw: 0,
        answer: Live::default(),
        wants_export: false,
        exporting: None,
    })
}

/// The age typed, or the one in force while the field is being written.
fn age(typed: &str, otherwise: u32) -> u32 {
    typed
        .trim()
        .parse()
        .map_or(otherwise, |age: u32| age.clamp(AGES.0, AGES.1))
}

/// The whole years between a birth date and today, read as calendar dates.
fn age_on(birth: NaiveDate, today: NaiveDate) -> u32 {
    let before = (today.month(), today.day()) < (birth.month(), birth.day());
    u32::try_from(today.year() - birth.year() - i32::from(before)).unwrap_or(0)
}

/// Where a photo would be: the first letters of the first given name and of the surname.
fn monogram(person: &Identity) -> String {
    let given = person.given_names.first().map_or("", String::as_str);
    [given, person.surname.as_str()]
        .iter()
        .filter_map(|name| name.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

fn country_name(language: Language, country: Country) -> String {
    named(language, "tools.identity.countries", &json!(country))
}

fn list_problem(language: Language, list: &UnreadableList) -> Option<String> {
    let path = format!("tools.identity.lists.{}", json!(list.problem).as_str()?);
    let said = Message::new(i18n::Key::find(&path)?).with("file", list.file.as_str());
    Some(said.text(language))
}

/// ⚠️ The lists are files beside the application: one that cannot be read is said, in place of a
/// card.
fn unreadable(ui: &mut Ui, list: &UnreadableList, language: Language) {
    if let Some(said) = list_problem(language, list) {
        problem(ui, &said, "identity-unreadable");
    }
    let said =
        Message::new(keys::tools::identity::lists::WHERE).with("folder", list.folder.as_str());
    hint(ui, &said.text(language));
}

fn identifier_rows(
    identifiers: &[Identifier],
    unkeyed: &[IdentifierKind],
    language: Language,
) -> Vec<Row> {
    identifiers
        .iter()
        .map(|identifier| Row {
            name: named(
                language,
                "tools.identity.identifiers",
                &json!(identifier.kind),
            ),
            value: identifier.value.clone(),
            mark: unkeyed.contains(&identifier.kind).then_some(Mark::Unkeyed),
        })
        .collect()
}

fn contact_rows(answer: &IdentityAnswer, language: Language) -> Vec<Row> {
    use keys::tools::identity::fields;
    let person = &answer.person;
    let phones = (!answer.fiction_phones).then_some(Mark::MayRing);
    vec![
        Row::new(language, fields::ADDRESS, &person.address.formatted),
        Row::new(language, fields::PHONE, &person.phone).marked(phones),
        Row::new(language, fields::MOBILE, &person.mobile).marked(phones),
        Row::new(language, fields::EMAIL, &person.email),
    ]
}

/// A country without an IBAN shows what it writes instead, and no empty row.
fn bank_rows(answer: &IdentityAnswer, language: Language) -> Vec<Row> {
    use keys::tools::identity::fields;
    let bank = &answer.person.bank;
    let mut rows = vec![Row::new(language, fields::BANK, &bank.name)];
    if let Some(iban) = &bank.iban {
        rows.push(Row::new(language, fields::IBAN, iban));
    }
    if let Some(bic) = &bank.bic {
        rows.push(Row::new(language, fields::BIC, bic));
    }
    rows.extend(identifier_rows(&bank.local, &answer.unkeyed, language));
    rows
}

fn company_rows(answer: &IdentityAnswer, language: Language) -> Vec<Row> {
    let company = &answer.person.company;
    let name = keys::tools::identity::fields::COMPANY;
    let mut rows = vec![Row::new(language, name, &company.name)];
    rows.extend(identifier_rows(
        &company.identifiers,
        &answer.unkeyed,
        language,
    ));
    rows
}

/// Why a row is marked, once a block: egui has no title to hover on a row's word.
fn mark_reasons(ui: &mut Ui, rows: &[Row], language: Language) {
    for mark in [Mark::MayRing, Mark::Unkeyed] {
        if rows.iter().any(|row| row.mark == Some(mark)) {
            let said = hint(ui, &i18n::translate(language, mark.reason(), &[]));
            address(ui, &said, "identity-mark");
        }
    }
}

fn block(ui: &mut Ui, id: &str, title: i18n::Key, rows: &[Row], language: Language) {
    let marks: Vec<String> = rows
        .iter()
        .map(|row| {
            row.mark.map_or_else(String::new, |mark| {
                i18n::translate(language, mark.word(), &[])
            })
        })
        .collect();
    card(ui, &format!("identity-block-{id}"), |ui| {
        group_title(ui, &i18n::translate(language, title, &[]));
        let shown: Vec<_> = rows
            .iter()
            .zip(&marks)
            .map(|(row, mark)| ResultRow::new(&row.name, &row.value).meta(mark))
            .collect();
        result_rows(ui, &shown, language);
    });
    mark_reasons(ui, rows, language);
}

fn fact_name(ui: &mut Ui, name: &str, mark: Option<&str>) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        paragraph(ui, &name.to_uppercase(), (10.5, 500.0), p.text_2);
        if let Some(mark) = mark {
            paragraph(ui, mark, (10.5, 500.0), p.amber_text);
        }
    });
}

/// A fact of the card with its name above it; `copied` is what its button gives.
fn fact(
    ui: &mut Ui,
    (name, value): (&str, &str),
    style: (f32, f32),
    copied: Option<&str>,
    author_id: &str,
    language: Language,
) {
    fact_name(ui, name, None);
    let ink = palette(ui.ctx()).text_0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let copy_room = if copied.is_some() { COPY_ROOM } else { 0.0 };
        let room = ui.available_width() - copy_room;
        ui.allocate_ui(Vec2::new(room, 0.0), |ui| {
            let said = paragraph(ui, value, style, ink);
            address(ui, &said, author_id);
        });
        if let Some(copied) = copied {
            CopyValue::new(copied, name, language).compact().show(ui);
        }
    });
    ui.add_space(FACT_GAP);
}

/// A number of the card, in the code face: its groups are read digit by digit.
fn number(ui: &mut Ui, row: &Row, author_id: &str, language: Language) {
    let mark = row.mark.map(|it| i18n::translate(language, it.word(), &[]));
    fact_name(ui, &row.name, mark.as_deref());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let shown = egui::RichText::new(&row.value)
            .font(FontId::monospace(13.0))
            .color(palette(ui.ctx()).text_0);
        let said = ui.add(egui::Label::new(shown).selectable(true));
        address(ui, &said, author_id);
        CopyValue::new(&row.value, &row.name, language)
            .compact()
            .show(ui);
    });
    ui.add_space(FACT_GAP);
}

fn draw_monogram(ui: &mut Ui, letters: &str) {
    let p = palette(ui.ctx());
    let side = Vec2::new(MONOGRAM, MONOGRAM * 1.25);
    let (rect, response) = ui.allocate_exact_size(side, Sense::hover());
    let corner = CornerRadius::same(10);
    let edge = Stroke::new(1.0, p.line);
    ui.painter()
        .rect(rect, corner, p.bg_2, edge, egui::StrokeKind::Inside);
    let shown = widgets::wrapped(ui, letters, (38.0, 700.0), p.text_2, MONOGRAM);
    let at = rect.center() - shown.size() / 2.0;
    shown.paint(ui, at, p.text_2);
    address(ui, &response, "identity-monogram");
}

fn card_head(ui: &mut Ui, person: &Identity, language: Language) {
    let p = palette(ui.ctx());
    let kind = i18n::translate(language, keys::tools::identity::CARD_KIND, &[]);
    egui::Frame::new()
        .fill(p.amber_text.gamma_multiply(0.06))
        .corner_radius(CornerRadius {
            nw: 14,
            ne: 14,
            ..CornerRadius::ZERO
        })
        .inner_margin(Margin::symmetric(18, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                paragraph(ui, &kind.to_uppercase(), (11.0, 700.0), p.amber_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = country_name(language, person.country);
                    let said = paragraph(ui, &name, (12.0, 400.0), p.text_1);
                    address(ui, &said, "identity-card-country");
                });
            });
        });
}

fn card_facts(ui: &mut Ui, answer: &IdentityAnswer, today: NaiveDate, language: Language) {
    use keys::tools::identity as words;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.spacing_mut().item_spacing.y = 2.0;
    let person = &answer.person;
    let (given, copied) = (person.given_names.join(", "), person.given_names.join(" "));
    let (name, shown) = (tr(words::fields::SURNAME), person.surname.to_uppercase());
    let surname = (name.as_str(), shown.as_str());
    fact(
        ui,
        surname,
        (22.0, 700.0),
        Some(&person.surname),
        "identity-surname",
        language,
    );
    let names = tr(words::fields::GIVEN_NAMES);
    let names = (names.as_str(), given.as_str());
    fact(
        ui,
        names,
        (16.0, 600.0),
        Some(&copied),
        "identity-given",
        language,
    );
    let sex = named(language, "tools.identity.sexes", &json!(person.sex));
    let nationality = named(
        language,
        "tools.identity.nationalities",
        &json!(person.country),
    );
    let place = format!("{} ({})", person.birth.city, person.birth.region);
    let born = person.birth.date.parse().ok().map_or_else(
        || person.birth.date.clone(),
        |birth| {
            let aged = Message::new(words::AGED)
                .with("count", age_on(birth, today))
                .text(language);
            format!("{} {aged}", i18n::format::date_long(birth, language))
        },
    );
    let short = (13.0, 500.0);
    ui.columns(2, |halves| {
        let name = tr(words::SEX);
        fact(
            &mut halves[0],
            (&name, &sex),
            short,
            None,
            "identity-card-sex",
            language,
        );
        let name = tr(words::fields::NATIONALITY);
        let shown = (name.as_str(), nationality.as_str());
        fact(
            &mut halves[1],
            shown,
            short,
            None,
            "identity-nationality",
            language,
        );
        let name = tr(words::fields::BORN_ON);
        let copied = Some(person.birth.date.as_str());
        fact(
            &mut halves[0],
            (&name, &born),
            short,
            copied,
            "identity-birth",
            language,
        );
        let name = tr(words::fields::BORN_IN);
        fact(
            &mut halves[1],
            (&name, &place),
            short,
            None,
            "identity-birth-place",
            language,
        );
    });
    for row in identifier_rows(&person.identifiers, &answer.unkeyed, language) {
        number(ui, &row, "identity-personal", language);
    }
    let document = Row::new(language, words::fields::DOCUMENT, &person.document);
    number(ui, &document, "identity-document", language);
}

fn person_card(ui: &mut Ui, answer: &IdentityAnswer, today: NaiveDate, language: Language) {
    let p = palette(ui.ctx());
    let shown = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            card_head(ui, &answer.person, language);
            egui::Frame::new()
                .inner_margin(Margin::same(18))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 18.0;
                        draw_monogram(ui, &monogram(&answer.person));
                        ui.vertical(|ui| card_facts(ui, answer, today, language));
                    });
                });
        });
    address(ui, &shown.response, "identity-card");
}

impl IdentityTool {
    fn request(&self) -> IdentityRequest {
        let asked = self.answer.answered().map(|(request, _)| request);
        let seed = self.seed.trim();
        IdentityRequest {
            country: self.country,
            sex: self.sex,
            age_min: age(
                &self.age_min,
                asked.map_or(AGES_AT_FIRST.0, |it| it.age_min),
            ),
            age_max: age(
                &self.age_max,
                asked.map_or(AGES_AT_FIRST.1, |it| it.age_max),
            ),
            seed: (!seed.is_empty()).then(|| seed.to_owned()),
        }
    }

    fn drawn(&self) -> Option<&IdentityAnswer> {
        match self.answer.value()? {
            IdentityOutcome::Drawn { answer } => Some(answer),
            IdentityOutcome::Unreadable { .. } => None,
        }
    }

    fn options(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::identity as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        section_title(ui, &tr(words::OPTIONS));
        let countries = Country::ALL.map(|it| (it, country_name(language, it)));
        let countries = countries.each_ref().map(|(id, name)| (*id, name.as_str()));
        let name = tr(words::COUNTRY);
        label(ui, &name);
        ChoiceMenu::new(&name, "identity-country", &countries)
            .language(language)
            .show(ui, &mut self.country);
        label(ui, &tr(words::SEX));
        let sexes = [
            (None, tr(words::sexes::ANY)),
            (Some(Sex::Female), tr(words::sexes::F)),
            (Some(Sex::Male), tr(words::sexes::M)),
        ];
        let sexes = sexes.each_ref().map(|(id, name)| (*id, name.as_str()));
        Segmented::new(&sexes, "identity-sex").show(ui, &mut self.sex);
        label(ui, &tr(words::AGE));
        line(ui, |ui| {
            number_field(
                ui,
                &mut self.age_min,
                &tr(words::AGE_MIN),
                "identity-age-min",
            );
            hint(ui, "-");
            number_field(
                ui,
                &mut self.age_max,
                &tr(words::AGE_MAX),
                "identity-age-max",
            );
            hint(ui, &tr(words::YEARS));
        });
        self.seed_row(ui, language);
        self.export_options(ui, drawn);
    }

    /// One place for the seed: the one typed, or the one just drawn and the pin that fixes it.
    fn seed_row(&mut self, ui: &mut Ui, language: Language) {
        use keys::tools::identity as words;
        let tr = |key| i18n::translate(language, key, &[]);
        let name = tr(words::SEED);
        let used = self.drawn().map(|answer| answer.seed.clone());
        let pinned = !self.seed.trim().is_empty();
        label(ui, &name);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.scope(|ui| {
                ui.set_max_width(160.0);
                TextField::new(&mut self.seed, &name, "identity-seed")
                    .hint(&tr(words::SEED_PLACEHOLDER))
                    .compact()
                    .show(ui);
            });
            if !pinned && let Some(used) = &used {
                let said = Message::new(words::SEED_USED)
                    .with("seed", used.as_str())
                    .text(language);
                let said = hint(ui, &said);
                address(ui, &said, "identity-seed-used");
            }
            if pinned || used.is_some() {
                let pin = IconButton::new(Icon::Pin, &tr(words::PIN_SEED), "identity-pin-seed")
                    .pressed(pinned)
                    .show(ui);
                let why = tr(if pinned {
                    words::FREE_SEED
                } else {
                    words::KEEP_SEED
                });
                if tooltip(pin, &why).clicked() {
                    self.seed = if pinned {
                        String::new()
                    } else {
                        used.clone().unwrap_or_default()
                    };
                }
            }
        });
        if pinned {
            let said = hint(ui, &tr(words::SEED_PINNED));
            address(ui, &said, "identity-seed-pinned");
        }
    }

    fn export_options(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::identity as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        ui.add_space(8.0);
        section_title(ui, &tr(words::BATCH));
        let formats = [
            (PeopleFormat::Json, tr(words::formats::JSON)),
            (PeopleFormat::Csv, tr(words::formats::CSV)),
        ];
        let formats = formats.each_ref().map(|(id, name)| (*id, name.as_str()));
        line(ui, |ui| {
            let name = tr(words::PEOPLE);
            label(ui, &name);
            number_field(ui, &mut self.people, &name, "identity-people");
            Segmented::new(&formats, "identity-format").show(ui, &mut self.format);
        });
        if self.format == PeopleFormat::Csv {
            self.separator_choice(ui, drawn);
        }
        if Button::new(&tr(words::EXPORT), "identity-export")
            .show(ui)
            .clicked()
        {
            self.wants_export = true;
        }
        label(ui, &tr(words::BATCH_COUNTRIES));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            for code in Country::ALL {
                self.batch_chip(ui, code, language);
            }
        });
        hint(ui, &tr(words::BATCH_HINT));
    }

    fn separator_choice(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::identity::separators;
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        let options = [
            (CsvSeparator::Comma, tr(separators::COMMA)),
            (CsvSeparator::Semicolon, tr(separators::SEMICOLON)),
        ];
        let options = options.each_ref().map(|(id, name)| (*id, name.as_str()));
        let was = self.separator;
        Segmented::new(&options, "identity-separator").show(ui, &mut self.separator);
        if self.separator != was {
            drawn.push(Action::SetCsvSeparator(self.separator));
        }
    }

    /// The card's own country is always of the batch: it is the first person of the file.
    fn batch_chip(&mut self, ui: &mut Ui, code: Country, language: Language) {
        let own = code == self.country;
        let lit = own || self.batch.contains(&code);
        let id = format!("identity-batch-country-{}", code.as_str());
        let chip = Chip::new(code.as_str(), &id, lit).show(ui);
        if tooltip(chip, &country_name(language, code)).clicked() && !own {
            if lit {
                self.batch.retain(|it| *it != code);
            } else {
                self.batch.push(code);
            }
        }
    }

    fn person(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        use keys::tools::identity as words;
        let language = drawn.language;
        let tr = |key| i18n::translate(language, key, &[]);
        let json = self.drawn().map(|it| it.json.clone()).unwrap_or_default();
        section_head(ui, &tr(words::PERSON), |ui| {
            CopyValue::new(&json, &tr(words::PERSON), language)
                .label(words::COPY_JSON)
                .show(ui);
        });
        if let Some(IdentityOutcome::Unreadable { list }) = self.answer.value() {
            unreadable(ui, list, language);
        }
        let Some(answer) = self.drawn() else { return };
        let today = drawn.now.with_timezone(&Local).date_naive();
        person_card(ui, answer, today, language);
        block(
            ui,
            "contact",
            words::blocks::CONTACT,
            &contact_rows(answer, language),
            language,
        );
        block(
            ui,
            "bank",
            words::blocks::BANK,
            &bank_rows(answer, language),
            language,
        );
        let rows = company_rows(answer, language);
        block(ui, "company", words::blocks::COMPANY, &rows, language);
        if answer.person.company.identifiers.is_empty() {
            let said = hint(ui, &tr(words::NO_FIRM_NUMBER));
            address(ui, &said, "identity-no-firm-number");
        }
    }

    fn batch_of(&self) -> PeopleBatch {
        let people = self.people.trim().parse().unwrap_or(PEOPLE_AT_FIRST);
        PeopleBatch {
            countries: self.batch.clone(),
            count: people.clamp(1, MAX_PEOPLE),
            format: self.format,
            separator: self.separator,
        }
    }

    /// The seed of the card on screen: the file opens on that very person.
    fn start_export(&self, turn: &Turn<'_>, path: &std::path::Path) -> Pending<PeopleExport> {
        let mut request = self.request();
        if request.seed.is_none() {
            request.seed = self.drawn().map(|answer| answer.seed.clone());
        }
        let path = path.display().to_string();
        api::tools::export_identities(turn.backend, request, self.batch_of(), path)
    }

    fn follow_export(&mut self, turn: &mut Turn<'_>) {
        if std::mem::take(&mut self.wants_export) {
            let dialog = Arc::clone(turn.dialog);
            let name = format!("identities.{}", self.format.as_str());
            let pending = turn.backend.compute(move |_| Ok(dialog.save(&name)));
            self.exporting = Some(Exporting::Choosing(pending));
        }
        match self.exporting.as_mut() {
            Some(Exporting::Choosing(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.exporting = None;
                if let Ok(Some(path)) = answer {
                    let written = self.start_export(turn, &path);
                    self.exporting = Some(Exporting::Writing(written));
                }
            }
            Some(Exporting::Writing(pending)) => {
                let Some(answer) = pending.poll() else { return };
                self.exporting = None;
                match answer {
                    Ok(PeopleExport::Written { people }) => turn.status.report(
                        Message::new(keys::tools::identity::EXPORTED).with("count", people),
                    ),
                    Ok(PeopleExport::Failed { problem: reason }) => {
                        let path = format!("tools.files.{}", json!(reason).as_str().unwrap_or(""));
                        if let Some(key) = i18n::Key::find(&path) {
                            turn.errors.notify(Message::new(key));
                        }
                    }
                    Ok(PeopleExport::Unreadable { list }) => {
                        let path = format!(
                            "tools.identity.lists.{}",
                            json!(list.problem).as_str().unwrap_or("")
                        );
                        if let Some(key) = i18n::Key::find(&path) {
                            let said = Message::new(key).with("file", list.file.as_str());
                            turn.errors.notify(said);
                        }
                    }
                    Err(error) => turn.errors.report(keys::errors::TOOL_FAILED, &error),
                }
            }
            None => {}
        }
    }
}

impl Tool for IdentityTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        self.separator = turn.csv_separator;
        self.follow_export(turn);
        let asked = (self.request(), self.draw);
        self.answer
            .update(turn, Some(asked), |backend, (request, _)| {
                api::tools::generate_identity(backend, request)
            });
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        columns(ui, self, drawn, (Self::options, Self::person));
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        let answer = self.drawn()?;
        let person = &answer.person;
        let given = person.given_names.first().map_or("", String::as_str);
        Some(ToolResult {
            title: Message::new(keys::tools::identity::NOTE_TITLE)
                .with("name", format!("{given} {}", person.surname))
                .text(language),
            language: NoteLanguage::Json,
            content: answer.json.clone(),
            warning: None,
        })
    }

    fn actions(&self) -> Vec<ToolAction> {
        vec![ToolAction {
            id: "draw",
            label: keys::tools::identity::AGAIN,
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

    fn sample(&mut self, _: Language) {
        self.reset_person();
        SAMPLE_SEED.clone_into(&mut self.seed);
    }

    /// Back to the options a first visit has, and a fresh draw.
    fn clear(&mut self) {
        self.reset_person();
        self.batch.clear();
        self.seed.clear();
        self.people = PEOPLE_AT_FIRST.to_string();
        self.draw += 1;
    }
}

impl IdentityTool {
    fn reset_person(&mut self) {
        self.country = Country::Fr;
        self.sex = None;
        self.age_min = AGES_AT_FIRST.0.to_string();
        self.age_max = AGES_AT_FIRST.1.to_string();
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::tools::identity::ListProblem;

    use super::*;
    use crate::testing;

    fn date(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    #[test]
    fn a_list_that_cannot_be_read_is_named_with_where_it_was_looked_for() {
        for (problem, word) in [
            (ListProblem::Missing, "not found"),
            (ListProblem::Damaged, "unreadable"),
        ] {
            let list = UnreadableList {
                problem,
                file: "names.txt".to_owned(),
                folder: "identity/".to_owned(),
            };
            let harness =
                testing::harness().build_ui(move |ui| unreadable(ui, &list, Language::En));

            let words = |node: &egui_kittest::Node<'_>| {
                let node = node.accesskit_node();
                node.value().or_else(|| node.label()).unwrap_or_default()
            };
            let said = harness.get_by(|node| node.author_id() == Some("identity-unreadable"));
            let said = words(&said);
            assert!(
                said.contains("names.txt") && said.to_lowercase().contains(word),
                "{said}"
            );
            let folder = harness.query_all_by(|node| {
                let node = node.value().or_else(|| node.label()).unwrap_or_default();
                node.contains("identity/")
            });
            assert_eq!(folder.count(), 1);
        }
    }

    #[test]
    fn an_age_is_counted_in_whole_years() {
        let birth = date("1990-06-15");
        assert_eq!(age_on(birth, date("2026-06-14")), 35);
        assert_eq!(age_on(birth, date("2026-06-15")), 36);
    }

    #[test]
    fn an_age_stays_in_bounds_and_survives_an_empty_field() {
        assert_eq!(age("200", 18), AGES.1);
        assert_eq!(age("", 18), 18);
    }
}
