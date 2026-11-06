//! What is left once the keys and the library's protection have pages of their own: how the
//! application looks, how it behaves, and what it says. Everything here edits the draft.

use egui::{Align, CornerRadius, FontFamily, Layout, RichText, Role, Sense, Stroke, Ui, Vec2};
use engine::app_info::METADATA;
use engine::notes::kind::NoteKind;
use i18n::{Key, Language, Message, keys};

use super::Ask;
use crate::action::{Action as Asked, Actions};
use crate::notes::calc::NUMBER_FORMATS;
use crate::services::settings::{
    CALC_LANGUAGE, CalcDate, IndentChoice, LocaleChoice, Settings, SettingsDraft,
};
use crate::state::AppState;
use crate::theme::{Appearance, Density, palette};
use crate::titlebar::Action;
use crate::widgets::{self, ChoiceMenu, Segmented};

const ROW: f32 = 40.0;
const CONTROL: f32 = 26.0;
/// What each locale makes of one figure, beside its name: the choice is seen, not guessed.
const SAMPLE: &str = "1578.13";

pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    draft: &mut SettingsDraft,
    actions: &mut Actions,
) {
    let language = state.language;
    let mut settings = draft.draft().clone();
    ui.spacing_mut().item_spacing.y = 0.0;
    appearance(ui, language, &mut settings);
    editor(ui, language, &mut settings);
    calculation(ui, language, &mut settings);
    behaviour(ui, language, &mut settings);
    group(ui, language, keys::settings::quick_paste::TITLE);
    switch_row(
        ui,
        language,
        keys::settings::quick_paste::SHOW_PINNED_FIRST,
        "setting-pinned-first",
        &mut settings.show_pinned_first,
    );
    notifications(ui, language, &mut settings);
    let look = (settings.theme, settings.density) != {
        let shown = draft.draft();
        (shown.theme, shown.density)
    };
    draft.edit(|edited| *edited = settings);
    if look {
        actions.push(Asked::Menu(Action::Preferences(Ask::Preview)));
    }
}

fn appearance(ui: &mut Ui, language: Language, settings: &mut Settings) {
    let tr = |key: Key| i18n::translate(language, key, &[]);
    group(ui, language, keys::settings::appearance::TITLE);
    let locales = [
        (LocaleChoice::System, tr(keys::locale::SYSTEM)),
        (LocaleChoice::Fr, tr(keys::locale::FR)),
        (LocaleChoice::En, tr(keys::locale::EN)),
    ];
    let locales: Vec<(LocaleChoice, &str)> = locales
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    let label = tr(keys::settings::appearance::LANGUAGE);
    row(ui, &label, |ui| {
        ChoiceMenu::new(&label, "setting-locale", &locales).show(ui, &mut settings.locale);
    });
    let themes = [
        (Appearance::System, tr(keys::settings::theme::SYSTEM)),
        (Appearance::Dark, tr(keys::settings::theme::DARK)),
        (Appearance::Light, tr(keys::settings::theme::LIGHT)),
    ];
    let themes: Vec<(Appearance, &str)> = themes
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    row(ui, &tr(keys::settings::appearance::THEME), |ui| {
        Segmented::new(&themes, "setting-theme").show(ui, &mut settings.theme);
    });
    let densities = [
        (
            Density::Comfortable,
            tr(keys::settings::density::COMFORTABLE),
        ),
        (Density::Compact, tr(keys::settings::density::COMPACT)),
    ];
    let densities: Vec<(Density, &str)> = densities
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    row(ui, &tr(keys::settings::appearance::DENSITY), |ui| {
        Segmented::new(&densities, "setting-density").show(ui, &mut settings.density);
    });
}

fn editor(ui: &mut Ui, language: Language, settings: &mut Settings) {
    let tr = |key: Key| i18n::translate(language, key, &[]);
    group(ui, language, keys::settings::editor::TITLE);
    let kinds = [
        (NoteKind::Note, tr(keys::settings::note_kind::NOTE)),
        (NoteKind::Snippet, tr(keys::settings::note_kind::SNIPPET)),
        (
            NoteKind::Checklist,
            tr(keys::settings::note_kind::CHECKLIST),
        ),
        (NoteKind::Calc, tr(keys::settings::note_kind::CALC)),
    ];
    let kinds: Vec<(NoteKind, &str)> = kinds
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    row(ui, &tr(keys::settings::editor::DEFAULT_KIND), |ui| {
        Segmented::new(&kinds, "setting-note-kind").show(ui, &mut settings.default_note_kind);
    });
    let indents = [
        (IndentChoice::Language, tr(keys::settings::indent::LANGUAGE)),
        (
            IndentChoice::TwoSpaces,
            tr(keys::settings::indent::TWO_SPACES),
        ),
        (
            IndentChoice::FourSpaces,
            tr(keys::settings::indent::FOUR_SPACES),
        ),
        (IndentChoice::Tab, tr(keys::settings::indent::TAB)),
    ];
    let indents: Vec<(IndentChoice, &str)> = indents
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    let label = tr(keys::settings::editor::INDENT);
    row(ui, &label, |ui| {
        ChoiceMenu::new(&label, "setting-indent", &indents).show(ui, &mut settings.code_indent);
    });
    switch_row(
        ui,
        language,
        keys::settings::editor::WRAP,
        "setting-code-wrap",
        &mut settings.code_wrap,
    );
}

fn calculation(ui: &mut Ui, language: Language, settings: &mut Settings) {
    let tr = |key: Key| i18n::translate(language, key, &[]);
    group(ui, language, keys::settings::calc::TITLE);
    let auto = tr(keys::settings::calc::DECIMALS_AUTO);
    let figures: Vec<String> = (0..=15_u8).map(|places| places.to_string()).collect();
    let mut decimals: Vec<(Option<u8>, &str)> = vec![(None, auto.as_str())];
    decimals.extend((0..=15_u8).zip(&figures).map(|(places, name)| {
        let name: &str = name;
        (Some(places), name)
    }));
    let label = tr(keys::settings::calc::DECIMALS);
    row(ui, &label, |ui| {
        ChoiceMenu::new(&label, "setting-calc-decimals", &decimals)
            .show(ui, &mut settings.calc_decimals);
    });

    let formats = number_formats(language);
    let names: Vec<(usize, &str)> = formats
        .iter()
        .enumerate()
        .map(|(at, (_, name))| (at, name.as_str()))
        .collect();
    let mut chosen = formats
        .iter()
        .position(|(id, _)| *id == settings.calc_number_format)
        .unwrap_or(0);
    let label = tr(keys::settings::calc::NUMBER_FORMAT);
    row(ui, &label, |ui| {
        ChoiceMenu::new(&label, "setting-calc-number-format", &names).show(ui, &mut chosen);
    });
    formats[chosen]
        .0
        .clone_into(&mut settings.calc_number_format);

    let dates = [
        (CalcDate::Numeric, tr(keys::settings::calc::dates::NUMERIC)),
        (CalcDate::Short, tr(keys::settings::calc::dates::SHORT)),
        (CalcDate::Long, tr(keys::settings::calc::dates::LONG)),
    ];
    let dates: Vec<(CalcDate, &str)> = dates
        .iter()
        .map(|(choice, name)| (*choice, name.as_str()))
        .collect();
    row(ui, &tr(keys::settings::calc::DATE_FORMAT), |ui| {
        Segmented::new(&dates, "setting-calc-date-format").show(ui, &mut settings.calc_date_format);
    });
}

/// `language` first, then each locale, by its name and what it makes of the sample.
fn number_formats(language: Language) -> Vec<(&'static str, String)> {
    let own = crate::notes::calc::sample(SAMPLE, CALC_LANGUAGE, language);
    let name = i18n::translate(language, keys::settings::calc::LANGUAGE, &[]);
    let mut formats = vec![(CALC_LANGUAGE, format!("{name} · {own}"))];
    formats.extend(NUMBER_FORMATS.iter().map(|format| {
        let name = i18n::Key::find(&format!("settings.calc.locales.{}", format.id)).map_or_else(
            || format.id.to_owned(),
            |key| i18n::translate(language, key, &[]),
        );
        let sample = crate::notes::calc::sample(SAMPLE, format.id, language);
        (format.id, format!("{name} · {sample}"))
    }));
    formats
}

fn behaviour(ui: &mut Ui, language: Language, settings: &mut Settings) {
    use keys::settings::behavior;
    group(ui, language, behavior::TITLE);
    switch_row(
        ui,
        language,
        behavior::START_WITH_SYSTEM,
        "setting-autostart",
        &mut settings.start_with_system,
    );
    switch_row(
        ui,
        language,
        behavior::MINIMIZE_TO_TRAY,
        "setting-minimize-to-tray",
        &mut settings.minimize_to_tray,
    );
    switch_row(
        ui,
        language,
        behavior::CLOSE_TO_TRAY,
        "setting-close-to-tray",
        &mut settings.close_to_tray,
    );
    let said = Message::new(behavior::TRAY_NOTE)
        .with("app", METADATA.name)
        .text(language);
    note(ui, &said);
}

fn notifications(ui: &mut Ui, language: Language, settings: &mut Settings) {
    use keys::settings::notifications;
    group(ui, language, notifications::TITLE);
    switch_row(
        ui,
        language,
        notifications::COPY_CONFIRMATION,
        "setting-copy-confirmation",
        &mut settings.copy_confirmation,
    );
    let wanted = settings.update_notifications;
    switch_row(
        ui,
        language,
        notifications::UPDATES,
        "setting-update-notifications",
        &mut settings.update_notifications,
    );
    // Asking to be told about updates is exactly what taking a skip back means.
    if settings.update_notifications && !wanted {
        settings.skipped_update.clear();
    }
    if settings.skipped_update.is_empty() {
        return;
    }
    let said = Message::new(notifications::UPDATE_SKIPPED)
        .with("version", settings.skipped_update.as_str())
        .text(language);
    let undo = i18n::translate(language, notifications::UPDATE_SKIPPED_FORGET, &[]);
    ui.horizontal(|ui| {
        let words = ui.label(
            RichText::new(said)
                .size(11.5)
                .color(palette(ui.ctx()).text_2),
        );
        widgets::address(ui, &words, "setting-skipped-update");
        if widgets::Button::new(&undo, "setting-forget-skipped")
            .show(ui)
            .clicked()
        {
            settings.skipped_update.clear();
        }
    });
}

fn group(ui: &mut Ui, language: Language, title: Key) {
    let p = palette(ui.ctx());
    ui.add_space(10.0);
    let title = i18n::translate(language, title, &[]).to_uppercase();
    let words = widgets::text(ui, &title, 11.0, 600.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), words.size().y + 8.0),
        Sense::hover(),
    );
    ui.painter().galley(rect.min, words, p.text_2);
}

/// A label on the left, its control on the right, `line-soft` under them.
fn row(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    let shown = ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), ROW),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_size(Vec2::new(ui.available_width(), ROW));
            ui.label(RichText::new(label).size(12.5).color(p.text_1));
            // A segmented control grows down from where it starts: the row places it, not the
            // layout's centre.
            let room = ui.max_rect().shrink2(Vec2::new(0.0, (ROW - CONTROL) / 2.0));
            let reading = Layout::right_to_left(Align::Min);
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(room).layout(reading),
                control,
            );
        },
    );
    let rect = shown.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, p.line_soft),
    );
}

fn switch_row(ui: &mut Ui, language: Language, label: Key, author_id: &str, on: &mut bool) {
    let label = i18n::translate(language, label, &[]);
    row(ui, &label, |ui| switch(ui, &label, author_id, on));
}

/// The reference's `setting-switch`: a track and its knob, amber when on.
fn switch(ui: &mut Ui, label: &str, author_id: &str, on: &mut bool) {
    let p = palette(ui.ctx());
    let response = widgets::interact(
        ui,
        Vec2::new(34.0, 24.0),
        Sense::click(),
        author_id,
        || egui::WidgetInfo::selected(Role::CheckBox, true, *on, label),
        CornerRadius::same(9),
    );
    if response.clicked() {
        *on = !*on;
    }
    let track = egui::Rect::from_center_size(response.rect.center(), Vec2::new(34.0, 18.0));
    let (fill, knob) = if *on {
        (p.amber_fill.gamma_multiply(0.35), p.amber_text)
    } else {
        (p.bg_3, p.text_2)
    };
    ui.painter().rect(
        track,
        CornerRadius::same(9),
        fill,
        Stroke::new(1.0, p.line_soft),
        egui::StrokeKind::Inside,
    );
    let x = if *on {
        track.right() - 9.0
    } else {
        track.left() + 9.0
    };
    ui.painter()
        .circle_filled(egui::pos2(x, track.center().y), 6.0, knob);
}

fn note(ui: &mut Ui, said: &str) {
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new(said)
                .size(11.5)
                .color(palette(ui.ctx()).text_2),
        )
        .wrap(),
    );
}
