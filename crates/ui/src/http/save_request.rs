//! Where a request made from « New request » goes the first time it is saved, and under which
//! name. It creates what it lacks: the first request of a project needs no trip to the rail.

use egui::{Align, FontFamily, Layout, Sense, Ui};
use i18n::keys;

use super::{Action, modal};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::tabs::{SaveTarget, Tab, ToCreate};
use crate::state::http::tree::SavePlace;
use crate::theme::palette;
use crate::widgets::{self, Button, ChoiceMenu, Kind, TextField};

const WIDTH: f32 = 440.0;

/// What is missing from the form, said under the fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missing {
    Name,
    Collection,
    Folder,
}

/// The form while it is open, for the tab it was asked on.
pub(crate) struct SaveForm {
    key: String,
    name: String,
    /// By its rank; the rank after the last is a new collection.
    place: usize,
    collection: String,
    /// A folder to create under the place chosen.
    folder: Option<String>,
    missing: Option<Missing>,
}

impl SaveForm {
    fn of(tab: &Tab) -> Self {
        Self {
            key: tab.key.clone(),
            name: tab.draft.name.clone(),
            place: 0,
            collection: String::new(),
            folder: None,
            missing: None,
        }
    }

    /// What the form asks for, or what it lacks.
    fn target(&self, places: &[SavePlace]) -> Result<SaveTarget, Missing> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(Missing::Name);
        }
        let Some(place) = places.get(self.place) else {
            let collection = self.collection.trim();
            if collection.is_empty() {
                return Err(Missing::Collection);
            }
            return Ok(SaveTarget {
                collection_id: String::new(),
                folder_id: None,
                name: name.to_owned(),
                create: ToCreate::Collection(collection.to_owned()),
            });
        };
        let create = match self.folder.as_deref().map(str::trim) {
            None => ToCreate::Nothing,
            Some("") => return Err(Missing::Folder),
            Some(folder) => ToCreate::Folder(folder.to_owned()),
        };
        Ok(SaveTarget {
            collection_id: place.collection_id.clone(),
            folder_id: place.folder_id.clone(),
            name: name.to_owned(),
            create,
        })
    }
}

pub(super) fn show(ui: &Ui, state: &AppState, form: &mut Option<SaveForm>, actions: &mut Actions) {
    let Some(tab) = state.http.tabs().asking_place() else {
        *form = None;
        return;
    };
    let open = match form {
        Some(open) if open.key == tab.key => open,
        _ => form.insert(SaveForm::of(tab)),
    };
    let places = state.http.save_places();
    let shown = modal(ui, "http-save-dialog", |ui| {
        body(ui, open, &places, state.language)
    });
    let key = tab.key.clone();
    match shown.inner {
        Some(true) => match open.target(&places) {
            Ok(target) => actions.push(Asked::Http(Action::SaveTo { key, target })),
            Err(missing) => open.missing = Some(missing),
        },
        Some(false) => actions.push(Asked::Http(Action::CancelSaveTo)),
        None if shown.should_close() => actions.push(Asked::Http(Action::CancelSaveTo)),
        None => {}
    }
}

/// `true` on « Save », `false` on « Cancel ».
fn body(
    ui: &mut Ui,
    form: &mut SaveForm,
    places: &[SavePlace],
    language: i18n::Language,
) -> Option<bool> {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let title = tr(keys::http::save::TITLE);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "http-save-dialog");
    let label = tr(keys::http::save::NAME);
    caption(ui, &label);
    TextField::new(&mut form.name, &label, "http-save-name").show(ui);
    caption(ui, &tr(keys::http::save::PLACE));
    place(ui, form, places, language);
    if let Some(missing) = form.missing {
        let words = tr(match missing {
            Missing::Name => keys::http::save::missing::NAME,
            Missing::Collection => keys::http::save::missing::COLLECTION,
            Missing::Folder => keys::http::save::missing::FOLDER,
        });
        let words = egui::RichText::new(words).size(12.0).color(p.red);
        let said = ui.add(egui::Label::new(words).wrap());
        widgets::address(ui, &said, "http-save-missing");
    }
    let mut chosen = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::editor::SAVE), "http-save-submit")
            .kind(Kind::Filled)
            .show(ui)
            .clicked()
        {
            chosen = Some(true);
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-save-cancel")
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

/// The place, and what it lacks: named here, no trip to the rail and back.
fn place(ui: &mut Ui, form: &mut SaveForm, places: &[SavePlace], language: i18n::Language) {
    let tr = |key| i18n::translate(language, key, &[]);
    let new_collection = tr(keys::http::save::NEW_COLLECTION);
    let mut options: Vec<(usize, &str)> = places
        .iter()
        .enumerate()
        .map(|(at, place)| (at, place.path.as_str()))
        .collect();
    options.push((places.len(), &new_collection));
    form.place = form.place.min(places.len());
    if !places.is_empty() {
        let label = tr(keys::http::save::PLACE);
        let menu = ChoiceMenu::new(&label, "http-save-place", &options).show(ui, &mut form.place);
        if menu.changed() {
            form.folder = None;
            form.missing = None;
        }
    }
    if form.place == places.len() {
        let label = tr(keys::http::save::COLLECTION_NAME);
        TextField::new(&mut form.collection, &label, "http-save-collection")
            .hint(&label)
            .show(ui);
        return;
    }
    let Some(folder) = &mut form.folder else {
        if Button::new(&tr(keys::http::save::NEW_FOLDER), "http-save-new-folder")
            .show(ui)
            .clicked()
        {
            form.folder = Some(String::new());
        }
        return;
    };
    let label = tr(keys::http::save::FOLDER_NAME);
    TextField::new(folder, &label, "http-save-folder")
        .hint(&label)
        .show(ui);
    if Button::new(&tr(keys::http::save::NO_FOLDER), "http-save-folder-cancel")
        .show(ui)
        .clicked()
    {
        form.folder = None;
    }
}
