//! What the requests area asks, and what it keeps between frames.

use super::{AppState, http, notes_query};
use crate::api;
use crate::backend::Backend;

impl AppState {
    pub(super) fn apply_http(
        &mut self,
        action: crate::http::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::http::Action as Http;
        let now = ctx.input(|input| input.time);
        match action {
            Http::Toggle(id) => self.http.toggle(&id),
            Http::Write(write) => self.http.write(backend, write),
            Http::KeyMove { item, name, key } => self.http.key_move(backend, (item, name), key),
            Http::AskDelete(item) => self.http.ask_delete(backend, item),
            Http::CancelDelete => self.http.cancel_delete(),
            Http::OpenRequest(id) => {
                self.http.show_page(backend, http::Page::Requests);
                self.http.tabs_mut().open(backend, id);
            }
            Http::NewRequest(kind) => {
                let name = i18n::translate(self.language, i18n::keys::http::tabs::UNTITLED, &[]);
                self.http.tabs_mut().new_request(kind, name);
            }
            Http::Activate(key) => self.http.tabs_mut().activate(&key),
            Http::MoveTab { key, gap } => self.http.tabs_mut().move_to(&key, gap),
            Http::NudgeTab { key, right } => self.http.tabs_mut().nudge(&key, right),
            Http::AskClose(key) => self.http.ask_close(&key),
            Http::Close(key) => self.http.tabs_mut().close(&key),
            Http::CancelClose => self.http.tabs_mut().cancel_close(),
            Http::Save(key) => self.http.tabs_mut().save(backend, &key),
            Http::SaveAndClose(key) => self.http.tabs_mut().save_and_close(backend, &key),
            Http::Edit { key, edit } => self.http.tabs_mut().edit(&key, |draft| draft.apply(edit)),
            Http::ChooseBody { key, kind } => self.http.tabs_mut().choose_body_kind(&key, kind),
            Http::Rewrite { key, part } => self.http.rewrite(backend, &key, part),
            Http::PickFile { key, target } => {
                self.http.pick_file(backend, &self.dialog, key, target);
            }
            Http::AddExtraction { key, rule } => {
                let variable = rule.variable.clone();
                self.http.add_extraction(&key, rule);
                self.status.report(
                    i18n::Message::new(i18n::keys::http::extract::ADDED).with("variable", variable),
                );
            }
            Http::ChooseEnvironment(id) => self.http.choose_environment(id),
            Http::Environments(asked) => self.http.ask_environments(backend, asked, now),
            Http::Define(name) => self.http.define(backend, name, now),
            Http::SendAnyway(key) => self.http.send_anyway(backend, &key, now),
            Http::SendFile(key, path) => self.http.send_file(backend, &key, path, now),
            Http::StopAsking => self.http.stop_asking(),
            Http::Cookies(asked) => self.http.ask_cookies(backend, asked),
            Http::ToggleConsole => self.http.toggle_console(),
            Http::ClearConsole(key) => self.http.clear_console(&key),
            Http::SaveResponseFile { send_id, name } => {
                self.http
                    .kept_mut()
                    .save_file(backend, &self.dialog, send_id, name);
            }
            Http::AskResponseNote(asked) => {
                self.http.kept_mut().ask_response_note(backend, *asked);
            }
            Http::AskNote(asked) => self.http.kept_mut().ask_note(*asked),
            Http::SaveNote(placement) => self.http.kept_mut().save_note(backend, placement),
            Http::CancelNote => self.http.kept_mut().cancel_note(),
            Http::OpenScript { key, stop } => self.http.open_script(backend, key, &stop),
            Http::OpenLevel { origin, phase } => {
                self.http
                    .open_settings_of(backend, &origin.id, (phase, None));
            }
            Http::ScriptFocusTaken => self.http.script_focus_taken(),
            Http::SendMessage { key, text } => self.http.send_message(backend, &key, text),
            Http::ClearSocketLog(key) => self.http.clear_socket_log(&key),
            Http::Runner(asked) => self.http.ask_runner(backend, &self.dialog, asked),
            Http::Leave(asked) => {
                self.quitting &= asked != http::leave::Asked::Cancel;
                self.http.answer_leave(asked);
            }
            Http::History(asked) => self.http.ask_history(backend, asked),
            Http::Code(asked) => self.http.ask_code(asked),
            Http::CopyCode(text) => self.say_copied(ctx, text, i18n::keys::http::code::COPIED),
            Http::SaveTo { key, target } => self.http.tabs_mut().save_to(backend, &key, target),
            Http::CancelSaveTo => self.http.tabs_mut().cancel_save_to(),
            Http::OpenSettings(container) => self.http.open_settings(backend, container),
            Http::EditSettings(settings) => self.http.edit_settings(*settings),
            Http::SaveSettings => self.http.save_settings(backend),
            Http::CloseSettings => self.http.close_settings(),
            Http::ShowPage(page) => self.http.show_page(backend, page),
            Http::TogglePage(page) => self.http.toggle_page(backend, page),
            Http::Send(key) => self.http.send(backend, key.as_deref(), now),
            Http::CancelSend(key) => self.http.cancel_send(backend, &key),
            Http::Explore { send_id, query } => self.http.explore(&send_id, query),
            Http::ResponseImage(send_id) => self.http.ask_image(backend, &send_id),
            Http::Copy(text) => self.say_copied(ctx, text, i18n::keys::settings::COPIED),
            Http::ClearResponse(key) => self.http.clear_response(backend, &key),
            Http::SetShare(share) => {
                self.settings.http_request_share = share;
                self.save_settings(backend);
            }
            Http::SaveActive => {
                let dirty = self.http.tabs().active().filter(|tab| tab.is_dirty());
                if let Some(key) = dirty.map(|tab| tab.key.clone()) {
                    self.http.tabs_mut().save(backend, &key);
                }
            }
        }
        self.remember_http(backend);
    }

    pub(super) fn update_http(&mut self, ctx: &egui::Context, backend: &Backend, time: f64) {
        self.http.update((ctx, backend), time, &mut self.errors);
        let tz_offset = notes_query::tz_offset_minutes(self.clock.now());
        self.http
            .update_history((ctx, backend), (time, tz_offset), &mut self.errors);
        self.report_kept();
        if let Some(count) = self.http.take_history_cleared() {
            let cleared = i18n::Message::new(i18n::keys::http::history::CLEARED);
            self.status.report(cleared.with("count", count));
        }
        if let Some(count) = self.http.take_cookies_removed() {
            let removed = i18n::Message::new(i18n::keys::http::cookies::REMOVED);
            self.status.report(removed.with("count", count));
        }
        self.remember_http(backend);
    }

    /// A note kept, from an answer or from a tool: the canvas learns of it, wherever it is.
    pub(super) fn report_kept(&mut self) {
        let kept = self.http.take_kept_answer();
        if kept.note_made {
            self.revision.bump();
        }
        if let Some(said) = kept.said {
            self.status.report(said);
        }
    }

    pub(super) fn remember_http(&mut self, backend: &Backend) {
        if self.http.remember(&mut self.library_preferences) {
            drop(api::preferences::write_library(
                backend,
                self.library_preferences.clone(),
            ));
        }
    }
}
