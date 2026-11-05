//! One dialog for an answer and for generated code: it asks where the note goes and what it is
//! called; what it holds, and in which language, the caller has already decided.

use egui::{Align, FontFamily, Layout, Sense, Ui};
use i18n::keys;

use super::{Action, modal};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::kept::{NoteAsked, Placement};
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, Kind, TextField};

const WIDTH: f32 = 440.0;

/// The form while it is open.
pub(crate) struct NoteForm {
    title: String,
    tags: String,
    space: usize,
    /// By its rank in the space; `0` is no folder.
    folder: usize,
}

impl NoteForm {
    fn of(asked: &NoteAsked, state: &AppState) -> Self {
        let spaces = state.spaces.spaces();
        let active = state.spaces.active_space();
        let space = spaces
            .iter()
            .position(|space| Some(space.id.as_str()) == active)
            .unwrap_or(0);
        Self {
            title: asked.title.clone(),
            tags: asked.tags.clone(),
            space,
            folder: 0,
        }
    }
}

/// Commas only: a tag may hold a space, and Rust normalises what it keeps.
fn tags_of(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(super) fn show(ui: &Ui, state: &AppState, form: &mut Option<NoteForm>, actions: &mut Actions) {
    let Some(asked) = state.http.kept().asked() else {
        *form = None;
        return;
    };
    let open = form.get_or_insert_with(|| NoteForm::of(asked, state));
    let shown = modal(ui, "save-as-note", |ui| body(ui, state, asked, open));
    match shown.inner {
        Some(true) => {
            if let Some(placement) = placement(state, open) {
                actions.push(Asked::Http(Action::SaveNote(placement)));
            }
        }
        Some(false) => actions.push(Asked::Http(Action::CancelNote)),
        None if shown.should_close() => actions.push(Asked::Http(Action::CancelNote)),
        None => {}
    }
}

fn folders_of<'a>(state: &'a AppState, space_id: &str) -> Vec<&'a engine::folders::model::Folder> {
    let all = state.folders.list();
    all.iter().filter(|it| it.space_id == space_id).collect()
}

fn placement(state: &AppState, form: &NoteForm) -> Option<Placement> {
    let space = state.spaces.spaces().get(form.space)?;
    let folder = form
        .folder
        .checked_sub(1)
        .and_then(|at| folders_of(state, &space.id).get(at).copied());
    let place = folder.map_or_else(
        || space.name.clone(),
        |folder| format!("{} › {}", space.name, folder.name),
    );
    Some(Placement {
        title: form.title.trim().to_owned(),
        space_id: space.id.clone(),
        folder_id: folder.map(|folder| folder.id.clone()),
        tags: tags_of(&form.tags),
        place,
    })
}

/// `true` on « Save », `false` on « Cancel ».
fn body(ui: &mut Ui, state: &AppState, asked: &NoteAsked, form: &mut NoteForm) -> Option<bool> {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = tr(keys::tools::save_as_note::TITLE);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "save-as-note");
    let what = i18n::translate(
        language,
        asked.what,
        &[("language", asked.language.as_str().into())],
    );
    let what = egui::RichText::new(what).size(12.5).color(p.text_2);
    let said = ui.add(egui::Label::new(what).wrap());
    widgets::address(ui, &said, "save-as-note-what");
    let label = tr(keys::tools::save_as_note::NOTE_TITLE);
    caption(ui, &label);
    TextField::new(&mut form.title, &label, "save-as-note-title").show(ui);
    place(ui, state, form);
    let label = tr(keys::tools::save_as_note::TAGS);
    caption(ui, &label);
    TextField::new(&mut form.tags, &label, "save-as-note-tags")
        .hint(&tr(keys::tools::save_as_note::TAGS_PLACEHOLDER))
        .show(ui);
    let mut chosen = None;
    let can_save = !form.title.trim().is_empty() && !state.spaces.spaces().is_empty();
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(
            &tr(keys::tools::save_as_note::SUBMIT),
            "save-as-note-submit",
        )
        .kind(Kind::Filled)
        .enabled(can_save)
        .show(ui)
        .clicked()
        {
            chosen = Some(true);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "save-as-note-cancel")
            .show(ui)
            .clicked()
        {
            chosen = Some(false);
        }
    });
    chosen
}

fn caption(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(words).size(12.0).color(p.text_2);
    ui.add(egui::Label::new(words));
}

fn place(ui: &mut Ui, state: &AppState, form: &mut NoteForm) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let spaces = state.spaces.spaces();
    let label = tr(keys::tools::save_as_note::SPACE);
    caption(ui, &label);
    let options: Vec<(usize, &str)> = spaces
        .iter()
        .enumerate()
        .map(|(at, space)| (at, space.name.as_str()))
        .collect();
    let menu = ChoiceMenu::new(&label, "save-as-note-space", &options).show(ui, &mut form.space);
    if menu.changed() {
        form.folder = 0;
    }
    let Some(space) = spaces.get(form.space) else {
        return;
    };
    let label = tr(keys::tools::save_as_note::FOLDER);
    caption(ui, &label);
    let none = tr(keys::tools::save_as_note::NO_FOLDER);
    let folders = folders_of(state, &space.id);
    let mut options: Vec<(usize, &str)> = vec![(0, none.as_str())];
    options.extend(
        folders
            .iter()
            .enumerate()
            .map(|(at, folder)| (at + 1, folder.name.as_str())),
    );
    ChoiceMenu::new(&label, "save-as-note-folder", &options).show(ui, &mut form.folder);
}
