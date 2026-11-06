//! The two things that decide who can read the library — the phrase that unwraps its key, and
//! whether a copy is taken at each launch — and the dialog that changes the phrase. A phrase is
//! typed in a `PassphraseField` and leaves the view for the core by `take`, never by an `Action`.

use egui::{Id, Layout, RichText, Ui};
use engine::backup::copies::Backup;
use engine::vault::MINIMUM_LENGTH;
use i18n::{Key, Language, Message, keys};
use zeroize::Zeroizing;

use super::general::{group, note, row, switch_row};
use super::{Ask, View};
use crate::action::{Action as Asked, Actions};
use crate::gate::passphrase::{self, PassphraseField};
use crate::services::settings::Settings;
use crate::state::AppState;
use crate::theme::palette;
use crate::titlebar::Action;
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 440.0;

/// What the dialog holds while it is typed.
#[derive(Default)]
pub(super) struct ChangeView {
    current: Zeroizing<String>,
    next: Zeroizing<String>,
    confirmation: Zeroizing<String>,
}

impl ChangeView {
    fn too_short(&self) -> bool {
        !self.next.is_empty() && length(&self.next) < MINIMUM_LENGTH
    }

    fn mismatched(&self) -> bool {
        !self.confirmation.is_empty() && *self.confirmation != *self.next
    }

    fn can_submit(&self) -> bool {
        !self.current.is_empty()
            && length(&self.next) >= MINIMUM_LENGTH
            && *self.confirmation == *self.next
    }

    /// The two phrases, the three fields left empty: nothing typed outlives the attempt.
    pub(super) fn take(&mut self) -> (Zeroizing<String>, Zeroizing<String>) {
        self.confirmation = Zeroizing::new(String::new());
        (
            passphrase::take(&mut self.current),
            passphrase::take(&mut self.next),
        )
    }
}

fn length(phrase: &str) -> u32 {
    u32::try_from(phrase.chars().count()).unwrap_or(u32::MAX)
}

pub(crate) fn change_id() -> Id {
    Id::unique("change-passphrase")
}

pub(super) fn page(
    ui: &mut Ui,
    state: &AppState,
    (settings, reveal): (&mut Settings, &mut bool),
    actions: &mut Actions,
) {
    use keys::settings::security;
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    group(ui, language, security::ACCESS);
    row(ui, &tr(security::PASSPHRASE), |ui| {
        if Button::new(&tr(security::CHANGE), "setting-change-passphrase")
            .show(ui)
            .clicked()
        {
            actions.push(Asked::Menu(Action::Preferences(Ask::ChangePassphrase)));
        }
    });
    note(ui, &tr(security::NOTE));
    note(ui, &tr(security::COPIES_NOTE));
    group(ui, language, security::BACKUPS);
    switch_row(
        ui,
        language,
        security::AUTOMATIC_BACKUPS,
        "setting-automatic-backups",
        &mut settings.automatic_backups,
    );
    note(ui, &tr(security::BACKUPS_NOTE));
    note(ui, &tr(security::BACKUPS_LIMIT));
    copies(ui, state, reveal, actions);
}

fn copies(ui: &mut Ui, state: &AppState, reveal: &mut bool, actions: &mut Actions) {
    let language = state.language;
    let tr = |key: Key| i18n::translate(language, key, &[]);
    ui.add_space(6.0);
    if state.backups.is_loading() {
        note(ui, &tr(keys::backups::LOADING));
        return;
    }
    if state.backups.copies().is_empty() {
        let none = ui.label(
            RichText::new(tr(keys::backups::NONE))
                .size(11.5)
                .color(palette(ui.ctx()).text_2),
        );
        widgets::address(ui, &none, "backup-empty");
        return;
    }
    for copy in state.backups.copies() {
        ui.push_id(&copy.id, |ui| {
            copy_row(ui, state, copy, reveal, actions);
            if state.backups.pending() == Some(copy.id.as_str()) {
                confirm_restore(ui, state, copy, reveal, actions);
            }
        });
    }
}

fn copy_row(
    ui: &mut Ui,
    state: &AppState,
    copy: &Backup,
    reveal: &mut bool,
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let when = i18n::time::relative(copy.taken_at, state.clock.now()).text(language);
    let detail = format!("{} · {}", copy.id, human_size(copy.bytes, language));
    let shown = ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        ui.label(RichText::new(when).size(12.5).color(p.text_1));
        let words = ui.label(RichText::new(detail).size(11.0).monospace().color(p.text_2));
        widgets::address(ui, &words, "backup-detail");
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if !copy.openable {
                let said = i18n::translate(language, keys::backups::UNOPENABLE, &[]);
                ui.label(RichText::new(said).size(11.5).color(p.red));
            } else if state.backups.pending() != Some(copy.id.as_str()) {
                let label = i18n::translate(language, keys::backups::RESTORE, &[]);
                if Button::new(&label, "backup-restore").show(ui).clicked() {
                    let asked = Ask::Restore(copy.id.clone());
                    actions.push(Asked::Menu(Action::Preferences(asked)));
                    *reveal = true;
                }
            }
        });
    });
    widgets::address(ui, &shown.response, "backup-row");
}

/// The trigger gives way to a sentence naming what will happen, and the confirm sits elsewhere.
fn confirm_restore(
    ui: &mut Ui,
    state: &AppState,
    copy: &Backup,
    reveal: &mut bool,
    actions: &mut Actions,
) {
    let language = state.language;
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let said = Message::new(keys::backups::CONFIRM)
        .with("when", copy.id.as_str())
        .text(language);
    let words = ui.add(egui::Label::new(RichText::new(said).size(12.0).color(p.red)).wrap());
    widgets::address(ui, &words, "backup-confirm");
    let working = state.gate.is_working();
    let answers = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut ask = |ask| actions.push(Asked::Menu(Action::Preferences(ask)));
        if Button::new(&tr(keys::backups::CANCEL), "backup-cancel")
            .show(ui)
            .clicked()
        {
            ask(Ask::KeepLibrary);
        }
        let action = if working {
            keys::backups::RESTORING
        } else {
            keys::backups::CONFIRM_RESTORE
        };
        if Button::new(&tr(action), "backup-confirm-restore")
            .kind(Kind::Destructive)
            .enabled(!working)
            .show(ui)
            .clicked()
        {
            ask(Ask::ConfirmRestore);
        }
    });
    // Until it is in view, and no further: the strip replaces the trigger further down a panel
    // that scrolls, and the panel learns its new height a frame late.
    if *reveal {
        answers.response.scroll_to_me(None);
        *reveal = !ui.clip_rect().contains_rect(answers.response.rect);
    }
}

/// Rounded to the unit that reads: a copy is megabytes, not 4 233 981 bytes.
fn human_size(bytes: u64, language: Language) -> String {
    #[expect(clippy::cast_precision_loss, reason = "a size to read")]
    let mega = bytes as f64 / (1024.0 * 1024.0);
    if mega >= 1.0 {
        return Message::new(keys::http::response::MEGABYTES)
            .with("value", i18n::format::fixed(mega, 1, language))
            .text(language);
    }
    Message::new(keys::http::response::KILOBYTES)
        .with("value", (bytes / 1024).max(1).to_string())
        .text(language)
}

pub(super) fn dialog(
    ctx: &egui::Context,
    state: &AppState,
    view: &mut View,
    actions: &mut Actions,
) {
    let working = state.security.is_changing();
    let shown = widgets::dialog(ctx, &state.dialogs, change_id(), WIDTH, |ui| {
        body(ui, state, &mut view.change, actions);
    });
    if shown.is_some_and(|shown| shown.should_close()) && !working {
        actions.push(Asked::Menu(Action::Preferences(Ask::KeepPassphrase)));
    }
}

fn body(ui: &mut Ui, state: &AppState, view: &mut ChangeView, actions: &mut Actions) {
    use keys::settings::security;
    let language = state.language;
    let p = palette(ui.ctx());
    let tr = |key: Key| i18n::translate(language, key, &[]);
    let working = state.security.is_changing();
    ui.spacing_mut().item_spacing.y = 10.0;
    let heading = ui.label(
        RichText::new(tr(security::DIALOG_TITLE))
            .size(15.0)
            .variation(*b"wght", 600.0)
            .color(p.text_0),
    );
    widgets::address(ui, &heading, "change-passphrase");
    ui.add(
        egui::Label::new(
            RichText::new(tr(security::DIALOG_LEAD))
                .size(12.0)
                .color(p.text_2),
        )
        .wrap(),
    );
    let copies = ui.add(
        egui::Label::new(
            RichText::new(tr(security::COPIES_NOTE))
                .size(11.5)
                .color(p.text_2),
        )
        .wrap(),
    );
    widgets::address(ui, &copies, "change-passphrase-copies");
    let mut submitted = false;
    let (field, typed) = phrase_field(
        ui,
        (&tr(security::CURRENT), "change-passphrase-current"),
        &mut view.current,
        (state.security.refused(), working),
    );
    if typed.changed {
        actions.push(Asked::Menu(Action::Preferences(Ask::TypedCurrent)));
    }
    if !working && ui.memory(|memory| memory.focused().is_none()) {
        field.request_focus();
    }
    submitted |= typed.submitted;
    let (_, typed) = phrase_field(
        ui,
        (&tr(security::NEXT), "change-passphrase-next"),
        &mut view.next,
        (false, working),
    );
    submitted |= typed.submitted;
    let (_, typed) = phrase_field(
        ui,
        (
            &tr(keys::vault::CONFIRMATION),
            "change-passphrase-confirmation",
        ),
        &mut view.confirmation,
        (false, working),
    );
    submitted |= typed.submitted;
    let problem = problem(state, view, language);
    let said = ui.label(RichText::new(problem).size(11.9).color(p.red));
    widgets::address(ui, &said, "change-passphrase-problem");
    let ready = view.can_submit() && !working;
    let mut ask = |ask| actions.push(Asked::Menu(Action::Preferences(ask)));
    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let action = if working {
            security::CHANGING
        } else {
            security::CHANGE
        };
        let submit = Button::new(&tr(action), "change-passphrase-submit")
            .kind(Kind::Filled)
            .enabled(ready)
            .show(ui);
        if ready && (submit.clicked() || submitted) {
            ask(Ask::SubmitPassphrase);
        }
        if Button::new(&tr(security::CANCEL), "change-passphrase-cancel")
            .enabled(!working)
            .show(ui)
            .clicked()
        {
            ask(Ask::KeepPassphrase);
        }
    });
}

fn phrase_field(
    ui: &mut Ui,
    (name, author_id): (&str, &str),
    value: &mut Zeroizing<String>,
    (invalid, working): (bool, bool),
) -> (egui::Response, passphrase::Typed) {
    let p = palette(ui.ctx());
    ui.label(RichText::new(name).size(11.2).color(p.text_2));
    PassphraseField::new(value, name, author_id)
        .invalid(invalid)
        .enabled(!working)
        .show(ui)
}

fn problem(state: &AppState, view: &ChangeView, language: Language) -> String {
    if state.security.refused() {
        i18n::translate(language, keys::errors::WRONG_PASSPHRASE, &[])
    } else if view.too_short() {
        Message::new(keys::vault::TOO_SHORT)
            .with("length", MINIMUM_LENGTH)
            .text(language)
    } else if view.mismatched() {
        i18n::translate(language, keys::vault::MISMATCHED, &[])
    } else {
        String::new()
    }
}
