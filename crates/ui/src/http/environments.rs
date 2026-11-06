//! The globals and the environments, in place of the requests while it is open: the list on the
//! left, the set chosen on the right — its name, colour and protection, and its variables,
//! written as they are typed.

use egui::{
    Color32, CornerRadius, FontFamily, Layout, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2,
};
use engine::folders::model::FolderColour;
use engine::http::environments::{Environment, Variable, variable_counts};
use i18n::keys;

use super::{Action, variables_table};
use crate::action::{Action as Asked, Actions, AppAction};
use crate::notes::ui::folder_hue;
use crate::state::AppState;
use crate::state::http::environments::{Asked as Ask, Target};
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button, Checkbox, Kind, TextField, interact};

const LIST: f32 = 260.0;
const HEAD: f32 = 56.0;
const ENTRY: f32 = 52.0;

/// What only the page's drawing sees.
#[derive(Default)]
pub(crate) struct PageView {
    /// The name being typed for a new environment.
    creating: Option<String>,
    /// The name being typed for the set shown: its id, and the text.
    renaming: Option<(String, String)>,
    confirming_delete: bool,
    /// The set a secret was shown in: one shown in a set is not shown in the next.
    revealed_in: Target,
    revealed: variables_table::Revealed,
}

fn ask(actions: &mut Actions, asked: Ask) {
    actions.push(Asked::Http(Action::Environments(asked)));
}

pub(super) fn show(
    ui: &mut Ui,
    region: Rect,
    state: &AppState,
    view: &mut PageView,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    ui.painter().rect_filled(region, CornerRadius::ZERO, p.bg_0);
    let head_rect = Rect::from_min_size(region.min, Vec2::new(region.width(), HEAD));
    let mut top = ui.new_child(UiBuilder::new().max_rect(head_rect.shrink2(Vec2::new(20.0, 0.0))));
    head(&mut top, state, view, actions);
    ui.painter().hline(
        region.x_range(),
        head_rect.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
    let body = Rect::from_min_max(pos2(region.left(), head_rect.bottom()), region.max);
    let list_rect = Rect::from_min_size(body.min, Vec2::new(LIST, body.height()));
    let mut list_ui = ui.new_child(UiBuilder::new().max_rect(list_rect.shrink(10.0)));
    list(&mut list_ui, state, view, actions);
    ui.painter().vline(
        list_rect.right(),
        body.y_range(),
        Stroke::new(1.0, p.line_soft),
    );
    let main_rect = Rect::from_min_max(pos2(list_rect.right(), body.top()), body.max);
    let mut main = ui.new_child(UiBuilder::new().max_rect(main_rect.shrink(20.0)));
    egui::ScrollArea::vertical()
        .id_salt("http-environment-main")
        .auto_shrink([false, false])
        .show(&mut main, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            set(ui, state, view, actions);
        });
}

fn head(ui: &mut Ui, state: &AppState, view: &mut PageView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let toggle = i18n::translate(language, keys::sidebar::TOGGLE, &[]);
        if widgets::SquareButton::new(Icon::Sidebar, &toggle, "http-rail-toggle")
            .pressed(state.settings.rail_shown)
            .show(ui)
            .clicked()
        {
            actions.push(Asked::App(AppAction::ToggleRail));
        }
        let title = tr(keys::http::environments::TITLE);
        let words = widgets::text(ui, &title, 15.0, 600.0, FontFamily::Proportional, p.text_0);
        let (rect, heading) = ui.allocate_exact_size(words.size(), Sense::hover());
        ui.painter().galley(rect.min, words, p.text_0);
        heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title));
        widgets::address(ui, &heading, "http-environments");
        let order = egui::RichText::new(tr(keys::http::environments::ORDER))
            .size(12.0)
            .color(p.text_2);
        ui.add(egui::Label::new(order).truncate());
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            create(ui, language, view, actions);
        });
    });
}

fn create(ui: &mut Ui, language: i18n::Language, view: &mut PageView, actions: &mut Actions) {
    let tr = |key| i18n::translate(language, key, &[]);
    let Some(name) = &mut view.creating else {
        if Button::new(
            &tr(keys::http::environments::NEW),
            "http-environment-create-open",
        )
        .kind(Kind::Filled)
        .show(ui)
        .clicked()
        {
            view.creating = Some(String::new());
        }
        return;
    };
    let mut close = Button::new(
        &tr(keys::http::rail::CANCEL),
        "http-environment-create-cancel",
    )
    .show(ui)
    .clicked();
    let mut submit = Button::new(
        &tr(keys::http::environments::CREATE),
        "http-environment-create-submit",
    )
    .kind(Kind::Filled)
    .show(ui)
    .clicked();
    ui.allocate_ui(Vec2::new(220.0, 0.0), |ui| {
        ui.set_width(220.0);
        let label = tr(keys::http::environments::NEW_NAME);
        let field = TextField::new(name, &label, "http-environment-create-input")
            .hint(&label)
            .compact()
            .show(ui);
        if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            submit = true;
        }
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if !field.has_focus() && !submit && !close && name.is_empty() {
            field.request_focus();
        }
    });
    if submit && !name.trim().is_empty() {
        ask(actions, Ask::Create(name.trim().to_owned()));
        close = true;
    }
    if close || submit {
        view.creating = None;
    }
}

fn count_words(rows: &[Variable], language: i18n::Language) -> String {
    let counts = variable_counts(rows);
    let count = i18n::translate(
        language,
        keys::http::environments::COUNT,
        &[("count", counts.count.into())],
    );
    if counts.secrets == 0 {
        return count;
    }
    let secrets = i18n::translate(
        language,
        keys::http::environments::SECRETS,
        &[("count", counts.secrets.into())],
    );
    format!("{count} · {secrets}")
}

fn list(ui: &mut Ui, state: &AppState, view: &mut PageView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let environments = state.http.environments();
    egui::ScrollArea::vertical()
        .id_salt("http-environment-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let meta = i18n::translate(
                language,
                keys::http::environments::GLOBALS_META,
                &[(
                    "count",
                    variable_counts(environments.globals()).count.into(),
                )],
            );
            let globals = Entry {
                name: &tr(keys::http::environments::GLOBALS),
                meta: &meta,
                badge: None,
                hue: p.text_2,
                on: *environments.target() == Target::Globals,
            };
            if entry(ui, &globals).clicked() {
                select(view, actions, Target::Globals);
            }
            let title = tr(keys::http::environments::LIST_TITLE).to_uppercase();
            let title = egui::RichText::new(title)
                .size(10.5)
                .strong()
                .color(p.text_2);
            ui.add(egui::Label::new(title));
            for environment in environments.all() {
                let on = environments.shown().map(|shown| &shown.id) == Some(&environment.id);
                let active = environments.active_id() == Some(environment.id.as_str());
                let badge = if active {
                    Some(tr(keys::http::environments::ACTIVE))
                } else {
                    environment
                        .protected
                        .then(|| tr(keys::http::environments::PROTECTED_BADGE))
                };
                let meta = count_words(&environment.variables, language);
                let shown = Entry {
                    name: &environment.name,
                    meta: &meta,
                    badge: badge.as_deref(),
                    hue: folder_hue(ui, environment.colour),
                    on,
                };
                if entry(ui, &shown).clicked() {
                    select(view, actions, Target::Environment(environment.id.clone()));
                }
            }
            if environments.is_loaded() && environments.all().is_empty() {
                let words = egui::RichText::new(tr(keys::http::environments::EMPTY))
                    .size(12.0)
                    .color(p.text_2);
                let said = ui.add(egui::Label::new(words).wrap());
                widgets::address(ui, &said, "http-environments-empty");
            }
        });
}

fn select(view: &mut PageView, actions: &mut Actions, target: Target) {
    view.confirming_delete = false;
    view.renaming = None;
    ask(actions, Ask::Select(target));
}

/// An entry of the list.
struct Entry<'a> {
    name: &'a str,
    meta: &'a str,
    badge: Option<&'a str>,
    hue: Color32,
    on: bool,
}

fn entry(ui: &mut Ui, shown: &Entry<'_>) -> Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let corner = CornerRadius::same(8);
    let response = interact(
        ui,
        Vec2::new(width, ENTRY),
        Sense::click(),
        "http-environment-entry",
        || WidgetInfo::selected(Role::Button, true, shown.on, shown.name),
        corner,
    );
    let rect = response.rect;
    if shown.on {
        ui.painter().rect_filled(rect, corner, p.bg_2);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, corner, p.bg_1);
    }
    let first = rect.top() + 18.0;
    ui.painter()
        .circle_filled(pos2(rect.left() + 14.0, first), 4.0, shown.hue);
    let name = widgets::text(
        ui,
        shown.name,
        13.0,
        500.0,
        FontFamily::Proportional,
        p.text_0,
    );
    let width = name.size().x;
    let at = pos2(rect.left() + 26.0, first - name.size().y / 2.0);
    ui.painter().galley(at, name, p.text_0);
    if let Some(badge) = shown.badge {
        let badge = widgets::text(
            ui,
            badge,
            10.5,
            600.0,
            FontFamily::Proportional,
            p.amber_text,
        );
        let at = pos2(at.x + width + 8.0, first - badge.size().y / 2.0);
        ui.painter().galley(at, badge, p.amber_text);
    }
    let meta = widgets::text(
        ui,
        shown.meta,
        11.0,
        400.0,
        FontFamily::Proportional,
        p.text_2,
    );
    ui.painter()
        .galley(pos2(rect.left() + 26.0, rect.top() + 28.0), meta, p.text_2);
    response
}

fn set(ui: &mut Ui, state: &AppState, view: &mut PageView, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    let environments = state.http.environments();
    if view.revealed_in != *environments.target() {
        view.revealed_in = environments.target().clone();
        view.revealed.clear();
    }
    match environments.shown() {
        Some(shown) => {
            set_head(ui, state, shown, view, actions);
            if shown.protected {
                let words = egui::RichText::new(tr(keys::http::environments::PROTECTED_NOTE))
                    .size(12.0)
                    .color(p.amber_text);
                let said = ui.add(egui::Label::new(words).wrap());
                widgets::address(ui, &said, "http-environment-protected-note");
            }
        }
        None => globals_head(ui, language),
    }
    let label = tr(keys::http::environments::VARIABLES);
    let table = variables_table::Table {
        kind: "environment",
        label: &label,
    };
    let rows = environments.rows();
    match variables_table::show(ui, &table, rows, &mut view.revealed, language) {
        Some(variables_table::Changed::Rows(rows)) => ask(actions, Ask::Rows(rows)),
        Some(variables_table::Changed::Filler(filler)) => {
            if let Some(id) = filler.request_id {
                actions.push(Asked::Http(Action::OpenRequest(id)));
            }
        }
        None => {}
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
        icons::paint(ui, rect, Icon::Lock, p.text_2);
        let words = egui::RichText::new(tr(keys::http::environments::SECRETS_CAPTION))
            .size(12.0)
            .color(p.text_2);
        ui.add(egui::Label::new(words));
    });
}

fn globals_head(ui: &mut Ui, language: i18n::Language) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let title = egui::RichText::new(tr(keys::http::environments::GLOBALS))
            .size(15.0)
            .strong()
            .color(p.text_0);
        ui.add(egui::Label::new(title));
        let hint = egui::RichText::new(tr(keys::http::environments::GLOBALS_HINT))
            .size(12.0)
            .color(p.text_2);
        ui.add(egui::Label::new(hint));
    });
}

fn set_head(
    ui: &mut Ui,
    state: &AppState,
    shown: &Environment,
    view: &mut PageView,
    actions: &mut Actions,
) {
    let language = state.language;
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        name(ui, shown, view, actions, language);
        let active = state.http.environments().active_id() == Some(shown.id.as_str());
        if active {
            let words = egui::RichText::new(tr(keys::http::environments::ACTIVE))
                .size(11.0)
                .color(palette(ui.ctx()).amber_text);
            ui.add(egui::Label::new(words));
        } else if Button::new(&tr(keys::http::environments::USE), "http-environment-use")
            .show(ui)
            .clicked()
        {
            ask(actions, Ask::Use(shown.id.clone()));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        colours(ui, shown, language, actions);
        let label = tr(keys::http::environments::PROTECTED);
        let mut protect = shown.protected;
        if Checkbox::new(&mut protect, &label, "http-environment-protected")
            .show(ui)
            .changed()
        {
            let id = shown.id.clone();
            ask(actions, Ask::Protect { id, protect });
        }
        let words = egui::RichText::new(&label)
            .size(12.5)
            .color(palette(ui.ctx()).text_1);
        ui.add(egui::Label::new(words));
        if Button::new(
            &tr(keys::http::environments::DUPLICATE),
            "http-environment-duplicate",
        )
        .show(ui)
        .clicked()
        {
            let name = i18n::translate(
                language,
                keys::http::environments::COPY_NAME,
                &[("name", shown.name.as_str().into())],
            );
            let id = shown.id.clone();
            ask(actions, Ask::Duplicate { id, name });
        }
        delete(ui, shown, view, actions, language);
    });
}

/// The name, written on leaving the field: blank or the same writes nothing.
fn name(
    ui: &mut Ui,
    shown: &Environment,
    view: &mut PageView,
    actions: &mut Actions,
    language: i18n::Language,
) {
    let mut text = match &view.renaming {
        Some((id, text)) if *id == shown.id => text.clone(),
        _ => shown.name.clone(),
    };
    let label = i18n::translate(language, keys::http::environments::NAME_LABEL, &[]);
    let field = ui
        .allocate_ui(Vec2::new(260.0, 0.0), |ui| {
            ui.set_width(260.0);
            TextField::new(&mut text, &label, "http-environment-name").show(ui)
        })
        .inner;
    if field.changed() {
        view.renaming = Some((shown.id.clone(), text.clone()));
    }
    if field.lost_focus() {
        let typed = text.trim();
        if !typed.is_empty() && typed != shown.name {
            let (id, name) = (shown.id.clone(), typed.to_owned());
            ask(actions, Ask::Rename { id, name });
        }
        view.renaming = None;
    }
}

fn colour_name(colour: FolderColour) -> i18n::Key {
    match colour {
        FolderColour::Blue => keys::folders::colour::BLUE,
        FolderColour::Amber => keys::folders::colour::AMBER,
        FolderColour::Purple => keys::folders::colour::PURPLE,
        FolderColour::Green => keys::folders::colour::GREEN,
        FolderColour::Red => keys::folders::colour::RED,
    }
}

fn colours(ui: &mut Ui, shown: &Environment, language: i18n::Language, actions: &mut Actions) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for colour in FolderColour::ALL {
            let on = shown.colour == colour;
            let name = i18n::translate(language, colour_name(colour), &[]);
            let response = interact(
                ui,
                Vec2::splat(24.0),
                Sense::click(),
                "http-environment-colour",
                || WidgetInfo::selected(Role::RadioButton, true, on, &name),
                CornerRadius::same(4),
            );
            swatch(ui, &response, folder_hue(ui, colour), on);
            if response.clicked() && !on {
                let id = shown.id.clone();
                ask(actions, Ask::Recolour { id, colour });
            }
        }
    });
}

fn swatch(ui: &Ui, response: &Response, hue: Color32, on: bool) {
    let rect = Rect::from_center_size(response.rect.center(), Vec2::splat(14.0));
    let shown = if on || response.hovered() {
        hue
    } else {
        hue.gamma_multiply(0.45)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), shown);
    if on {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(6),
            Stroke::new(1.0, hue),
            StrokeKind::Inside,
        );
    }
}

/// « Delete », asked in place.
fn delete(
    ui: &mut Ui,
    shown: &Environment,
    view: &mut PageView,
    actions: &mut Actions,
    language: i18n::Language,
) {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    let word = tr(keys::http::environments::DELETE);
    if !view.confirming_delete {
        if Button::new(&word, "http-environment-delete")
            .kind(Kind::Destructive)
            .show(ui)
            .clicked()
        {
            view.confirming_delete = true;
        }
        return;
    }
    let question = i18n::translate(
        language,
        keys::http::environments::CONFIRM_DELETE,
        &[("name", shown.name.as_str().into())],
    );
    let question = egui::RichText::new(question).size(12.5).color(p.text_1);
    let said = ui.add(egui::Label::new(question));
    widgets::address(ui, &said, "http-environment-delete-question");
    if Button::new(&word, "http-environment-delete-confirm")
        .kind(Kind::Destructive)
        .show(ui)
        .clicked()
    {
        view.confirming_delete = false;
        ask(actions, Ask::Delete(shown.id.clone()));
    }
    if Button::new(
        &tr(keys::http::rail::CANCEL),
        "http-environment-delete-cancel",
    )
    .show(ui)
    .clicked()
    {
        view.confirming_delete = false;
    }
}
