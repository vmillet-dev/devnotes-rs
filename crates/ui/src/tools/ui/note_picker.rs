//! The notes of every space an input accepts, searched like the canvas searches; the one chosen
//! read whole.

use egui::{CornerRadius, Id, Key, Margin, Modifiers, Role, Sense, Ui, Vec2, WidgetInfo};
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::DisplayNote;
use engine::notes::view::{NoteFilter, NoteOrder, NotesQuery, NotesView};
use i18n::{Message, keys};

use crate::api;
use crate::backend::Pending;
use crate::notes::language;
use crate::notes::ui::kind_badge;
use crate::state::AppState;
use crate::theme::palette;
use crate::tools::catalogue::NoteInput;
use crate::tools::live::{Live, Turn};
use crate::widgets::{self, Button, TextField, address, hovered, interact, towards};

/// Enough to find one by its title: a search narrows it further.
const SHOWN: usize = 30;
const WIDTH: f32 = 520.0;

/// What a tool receives from a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedNote {
    pub title: String,
    pub content: String,
    /// A snippet's alone: the other kinds have none.
    pub language: Option<Language>,
}

/// The dialog, while it is open.
pub struct Picker {
    /// Which input of the tool it fills.
    pub slot: &'static str,
    accepts: NoteInput,
    search: String,
    /// The field takes the focus when the dialog opens.
    fresh: bool,
    found: Live<String, NotesView>,
    chosen: Option<String>,
    reading: Option<Pending<DisplayNote>>,
    results: Vec<Id>,
    closed: bool,
}

impl Picker {
    pub fn new(slot: &'static str, accepts: NoteInput) -> Self {
        Self {
            slot,
            accepts,
            search: String::new(),
            fresh: true,
            found: Live::default(),
            chosen: None,
            reading: None,
            results: Vec::new(),
            closed: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// A list sends previews: the body is read whole, and a todo list gives its Markdown.
    pub fn update(&mut self, turn: &mut Turn<'_>) -> Option<PickedNote> {
        let accepts = self.accepts;
        let now = turn.now;
        let search = self.search.trim().to_owned();
        self.found.update(turn, Some(search), |backend, search| {
            api::notes::query(backend, query(search, accepts, now))
        });
        if let Some(id) = self.chosen.take() {
            self.reading = Some(api::notes::get(turn.backend, id));
        }
        let answer = self.reading.as_mut()?.poll()?;
        self.reading = None;
        match answer {
            Ok(note) => Some(picked(&note)),
            Err(error) => {
                turn.errors.report(keys::errors::NOTE_READ_FAILED, &error);
                None
            }
        }
    }
}

pub(super) fn query(
    search: String,
    accepts: NoteInput,
    now: chrono::DateTime<chrono::Utc>,
) -> NotesQuery {
    NotesQuery {
        space_id: None,
        folder_id: None,
        search,
        filter: NoteFilter::All,
        tags: Vec::new(),
        languages: accepts.languages.to_vec(),
        kinds: accepts.kinds.to_vec(),
        priorities: Vec::new(),
        order: NoteOrder::default(),
        grouping: engine::notes::view::Grouping::default(),
        ranked: false,
        now,
        tz_offset_minutes: crate::state::notes_query::tz_offset_minutes(now),
        pinned_first: false,
    }
}

fn picked(note: &DisplayNote) -> PickedNote {
    PickedNote {
        title: note.title.clone(),
        content: note
            .copy_text
            .clone()
            .unwrap_or_else(|| note.content.clone()),
        language: (note.kind == NoteKind::Snippet).then_some(note.language),
    }
}

pub fn show(ui: &Ui, state: &AppState, picker: &mut Picker) {
    let shown = crate::http::modal(ui, "note-picker", |ui| body(ui, state, picker));
    if shown.should_close() {
        picker.closed = true;
    }
}

fn body(ui: &mut Ui, state: &AppState, picker: &mut Picker) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = tr(keys::tools::from_note::TITLE);
    let heading = super::paragraph(ui, &title, (15.0, 600.0), p.text_0);
    address(ui, &heading, "note-picker");
    let accepted = super::paragraph(
        ui,
        &accepted(picker.accepts, language),
        (12.0, 400.0),
        p.text_2,
    );
    address(ui, &accepted, "note-picker-accepted");
    let label = tr(keys::tools::from_note::SEARCH);
    let field = TextField::new(&mut picker.search, &label, "note-picker-search")
        .hint(&label)
        .show(ui);
    if std::mem::take(&mut picker.fresh) {
        field.request_focus();
    }
    let notes: Vec<&engine::notes::view::Card> = picker
        .found
        .value()
        .into_iter()
        .flat_map(|view| view.sections.iter())
        .flat_map(|section| section.cards.iter())
        .take(SHOWN)
        .collect();
    let first = notes.first().map(|card| card.id.clone());
    let mut results = Vec::with_capacity(notes.len());
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("note-picker-list")
        .max_height(320.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for card in &notes {
                let row = option(ui, state, card);
                results.push(row.id);
                if row.clicked() {
                    chosen = Some(card.id.clone());
                }
            }
            if notes.is_empty() && !picker.found.is_pending() {
                let key = if picker.search.trim().is_empty() {
                    keys::tools::from_note::NONE
                } else {
                    keys::tools::from_note::NONE_FOUND
                };
                let none = super::paragraph(ui, &tr(key), (12.5, 400.0), p.text_2);
                address(ui, &none, "note-picker-none");
            }
        });
    if field.lost_focus() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
        chosen = chosen.or(first);
    }
    super::walk(ui, &results, &field);
    picker.results = results;
    if chosen.is_some() {
        picker.chosen = chosen;
    }
    ui.horizontal(|ui| {
        let hint = tr(keys::tools::from_note::HINT);
        ui.allocate_ui(Vec2::new(ui.available_width() - 90.0, 0.0), |ui| {
            super::hint(ui, &hint);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if Button::new(&tr(keys::placeholders::CANCEL), "note-picker-cancel")
                .show(ui)
                .clicked()
            {
                picker.closed = true;
            }
        });
    });
}

/// « JSON, YAML, TOML or XML snippets »: what the list holds, before anything is typed.
fn accepted(accepts: NoteInput, locale: i18n::Language) -> String {
    if accepts.languages.is_empty() {
        return i18n::translate(locale, keys::tools::from_note::ANY_NOTE, &[]);
    }
    let names: Vec<&str> = accepts
        .languages
        .iter()
        .map(|it| language::name(*it))
        .collect();
    Message::new(keys::tools::from_note::SNIPPETS)
        .with("languages", i18n::format::or_list(&names, locale))
        .text(locale)
}

fn option(ui: &mut Ui, state: &AppState, card: &engine::notes::view::Card) -> egui::Response {
    let p = palette(ui.ctx());
    let note: &engine::notes::model::Note = card;
    let locale = state.language;
    let title = if note.title.is_empty() {
        i18n::translate(locale, keys::notes::UNTITLED, &[])
    } else {
        note.title.clone()
    };
    let corner = CornerRadius::same(7);
    let width = ui.available_width();
    let response = interact(
        ui,
        Vec2::new(width, 34.0),
        Sense::click(),
        "note-picker-option",
        || WidgetInfo::labeled(Role::Button, true, &title),
        corner,
    );
    let rect = response.rect;
    let fill = towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response));
    ui.painter().rect_filled(rect, corner, fill);
    let inner = rect - Margin::symmetric(8, 0);
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 8.0;
    let badge = kind_badge(&mut row, note.kind, note.language, locale);
    let place = where_of(state, card);
    let tail = widgets::one_line(&row, &place, (11.5, 400.0), p.text_2, inner.width() * 0.35);
    let room = inner.right() - badge.rect.right() - tail.size().x - 24.0;
    let name = widgets::one_line(&row, &title, (13.0, 400.0), p.text_0, room);
    let (at, _) = row.allocate_exact_size(name.size(), Sense::hover());
    row.painter().galley(at.min, name, p.text_0);
    let right = egui::pos2(
        inner.right() - tail.size().x,
        rect.center().y - tail.size().y / 2.0,
    );
    row.painter().galley(right, tail, p.text_2);
    response
}

fn where_of(state: &AppState, card: &engine::notes::view::Card) -> String {
    let space = state
        .spaces
        .spaces()
        .iter()
        .find(|space| space.id == card.space_id)
        .map_or("", |space| space.name.as_str());
    let folder = state
        .folders
        .list()
        .iter()
        .find(|folder| Some(&folder.id) == card.folder_id.as_ref());
    match folder {
        Some(folder) => format!("{space} › {}", folder.name),
        None => space.to_owned(),
    }
}
