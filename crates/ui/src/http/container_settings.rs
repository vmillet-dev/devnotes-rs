//! A collection's or a folder's variables, auth, headers and transport: what every request under
//! it inherits. The draft is the state's, written on « Save ».

use egui::{Align, FontFamily, Layout, Sense, Ui};
use engine::http::model::{ContainerSettings, HttpItemKind};
use engine::http::transport::Transport;
use i18n::keys;

use super::request::{auth, table, transport};
use super::{Action, modal, variables_table};
use crate::action::{Action as Asked, Actions};
use crate::state::AppState;
use crate::state::http::containers::Form;
use crate::theme::palette;
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 760.0;

/// What only the dialog's drawing sees, for as long as it is open.
#[derive(Default)]
pub(crate) struct DialogView {
    revealed: bool,
    numbers: transport::Typed,
    secrets_shown: variables_table::Revealed,
}

pub(super) fn show(ui: &Ui, state: &AppState, view: &mut DialogView, actions: &mut Actions) {
    let Some(form) = state.http.container() else {
        *view = DialogView::default();
        return;
    };
    let shown = modal(ui, "http-settings-dialog", |ui| {
        body(ui, state, form, view, actions);
    });
    if shown.should_close() {
        actions.push(Asked::Http(Action::CloseSettings));
    }
}

fn body(ui: &mut Ui, state: &AppState, form: &Form, view: &mut DialogView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.set_width(WIDTH.min(ui.ctx().content_rect().width() * 0.9));
    ui.spacing_mut().item_spacing.y = 10.0;
    let container = &form.container;
    let key = if container.item.kind == HttpItemKind::Folder {
        keys::http::settings::title::FOLDER
    } else {
        keys::http::settings::title::COLLECTION
    };
    let title = i18n::translate(language, key, &[("name", container.name.as_str().into())]);
    let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
    let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
    ui.painter().galley(rect.min, words, p.text_0);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, &title));
    widgets::address(ui, &heading, "http-settings-dialog");
    let hint = egui::RichText::new(tr(keys::http::settings::HINT))
        .size(12.5)
        .color(p.text_2);
    ui.add(egui::Label::new(hint).wrap());
    if let Some(settings) = &form.settings {
        let height = ui.ctx().content_rect().height() * 0.65;
        egui::ScrollArea::vertical()
            .id_salt("http-settings")
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                if let Some(settings) = fields(ui, form, settings, view, language) {
                    actions.push(Asked::Http(Action::EditSettings(Box::new(settings))));
                }
            });
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if Button::new(&tr(keys::http::editor::SAVE), "http-settings-save")
            .kind(Kind::Filled)
            .enabled(form.settings.is_some())
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Http(Action::SaveSettings));
        }
        if Button::new(&tr(keys::http::rail::CANCEL), "http-settings-cancel")
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Http(Action::CloseSettings));
        }
    });
}

fn heading(ui: &mut Ui, words: &str, hint: Option<String>) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        let title = egui::RichText::new(words)
            .size(12.5)
            .strong()
            .color(p.text_1);
        ui.add(egui::Label::new(title));
        if let Some(hint) = hint {
            let hint = format!("· {hint}");
            let hint = egui::RichText::new(hint).size(12.0).color(p.text_2);
            ui.add(egui::Label::new(hint));
        }
    });
}

/// The settings as edited this frame.
fn fields(
    ui: &mut Ui,
    form: &Form,
    settings: &ContainerSettings,
    view: &mut DialogView,
    language: i18n::Language,
) -> Option<ContainerSettings> {
    let tr = |key| i18n::translate(language, key, &[]);
    let folder = form.container.item.kind == HttpItemKind::Folder;
    let mut changed = None;
    let variables = tr(keys::http::settings::VARIABLES);
    heading(
        ui,
        &variables,
        Some(tr(keys::http::settings::VARIABLES_HINT)),
    );
    let table = variables_table::Table {
        kind: "settings",
        label: &variables,
    };
    let rows = &settings.variables;
    if let Some(variables_table::Changed::Rows(variables)) =
        variables_table::show(ui, &table, rows, &mut view.secrets_shown, language)
    {
        changed = Some(ContainerSettings {
            variables,
            ..settings.clone()
        });
    }
    heading(ui, &tr(keys::http::editor::sections::AUTH), None);
    let field = auth::AuthField {
        auth: &settings.auth,
        above: form.above.as_ref(),
        allow_inherit: folder,
    };
    if let Some(auth) = auth::show(ui, &field, &mut view.revealed, language) {
        changed = Some(ContainerSettings {
            auth,
            ..settings.clone()
        });
    }
    let headers = tr(keys::http::editor::sections::HEADERS);
    heading(ui, &headers, None);
    let table = table::Table {
        kind: "settings-headers",
        label: &headers,
        headers: true,
    };
    if let Some(headers) = table::show(ui, &table, &settings.headers, language) {
        changed = Some(ContainerSettings {
            headers,
            ..settings.clone()
        });
    }
    heading(ui, &tr(keys::http::editor::sections::SETTINGS), None);
    let field = transport::TransportField {
        settings: settings.transport,
        base: form
            .above
            .as_ref()
            .map_or_else(Transport::default, |above| above.transport),
        above_is_default: !folder,
    };
    if let Some(transport) = transport::show(ui, &field, &mut view.numbers, language) {
        changed = Some(ContainerSettings {
            transport,
            ..settings.clone()
        });
    }
    changed
}
