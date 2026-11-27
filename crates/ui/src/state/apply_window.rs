//! What the title bar's menus, the preferences, the libraries, the palette and the update banner ask.

use super::{AppState, Area, EditorNext, http, libraries};
use crate::action::{Action, AppAction, GlobalAction};
use crate::api;
use crate::backend::Backend;
use crate::theme;
use crate::widgets::Layer;

impl AppState {
    pub(super) fn apply_menu(
        &mut self,
        action: crate::titlebar::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::Action as Menu;
        use engine::transfer::model::ExportScope;
        match action {
            Menu::Import => self.transfer.import(backend, &self.dialog),
            Menu::ExportAll => self.transfer.export(backend, ExportScope::Library),
            Menu::ExportSpace => {
                if let Some(space_id) = self.spaces.active_space().map(str::to_owned) {
                    self.transfer
                        .export(backend, ExportScope::Space { space_id });
                }
            }
            Menu::ExportSelection => {
                let ids = self.checked_on_screen();
                if !ids.is_empty() {
                    self.transfer.export(backend, ExportScope::Notes { ids });
                }
            }
            Menu::CopyMarkdown => {
                let ids = self.checked_on_screen();
                if !ids.is_empty() {
                    self.transfer.share(backend, ids);
                }
            }
            Menu::Prompted => {
                if let Some(answer) = self.titlebar_view.answer.take() {
                    self.transfer.answer(answer, backend);
                }
            }
            Menu::Quit => self.apply(Action::Global(GlobalAction::Quit), ctx, backend),
            Menu::OpenLibraries => {
                self.libraries.load(backend);
                let opener = self.titlebar_view.file_anchor;
                self.dialogs
                    .open(Layer::Settings, crate::titlebar::libraries::id(), opener);
            }
            Menu::Libraries(asked) => self.apply_libraries(asked, ctx, backend),
            Menu::OpenPreferences => {
                self.titlebar_view.open_preferences(&self.settings);
                self.security.load_variables(backend);
                self.backups.load(backend);
                let opener = self.titlebar_view.file_anchor;
                self.dialogs
                    .open(Layer::Settings, crate::titlebar::preferences::id(), opener);
            }
            Menu::Preferences(asked) => self.apply_preferences(asked, ctx, backend),
            Menu::OpenHelp(panel) => {
                if panel == crate::titlebar::about_menu::Panel::WhatsNew {
                    self.help.load(backend);
                }
                self.titlebar_view.show_help(panel);
                let opener = self.titlebar_view.about_anchor();
                self.dialogs
                    .open(Layer::App, crate::titlebar::about_menu::id(), opener);
            }
            Menu::CloseHelp => {
                self.titlebar_view.close_help();
                self.dialogs.close(ctx, crate::titlebar::about_menu::id());
            }
            Menu::OpenLink(url) => self.open_link(&url),
            Menu::CopyExample(text) => {
                let said = i18n::Message::new(i18n::keys::docs::COPIED);
                self.copy_out(ctx, backend, text, said);
            }
            Menu::Update(asked) => self.apply_update(asked, backend),
        }
    }

    pub(super) fn apply_update(
        &mut self,
        asked: crate::banners::update_prompt::Ask,
        backend: &Backend,
    ) {
        use crate::banners::update_prompt::Ask;
        match asked {
            Ask::Check => self.updates.check(backend, false),
            Ask::Reopen => self.updates.reopen(),
            Ask::ForgetCheck => self.updates.forget_check(),
            Ask::Install => {
                self.ask_switch(
                    libraries::Goal::Install,
                    http::leave::Reason::Update,
                    backend,
                );
            }
            Ask::Later { skip } => {
                if self.updates.dismiss(skip, &mut self.settings) {
                    self.save_settings(backend);
                }
            }
        }
    }

    /// Only the theme and the density show before a button: the rest of the application reads
    /// the applied settings, never the draft.
    pub(super) fn apply_preferences(
        &mut self,
        asked: crate::titlebar::preferences::Ask,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::preferences::Ask;
        match asked {
            Ask::Preview => {
                if let Some(look) = self.titlebar_view.previewed() {
                    theme::apply(ctx, look.theme, look.density);
                }
            }
            Ask::Apply => self.adopt_draft(ctx, backend),
            Ask::ApplyAndClose => {
                self.adopt_draft(ctx, backend);
                self.close_preferences(ctx);
            }
            Ask::Discard => {
                self.apply_look(ctx);
                self.close_preferences(ctx);
            }
            Ask::ChangePassphrase => {
                let change = crate::titlebar::preferences::change_id();
                self.dialogs.open(Layer::Passphrase, change, None);
            }
            Ask::TypedCurrent => self.security.clear_refusal(),
            Ask::Restore(id) => self.backups.ask(id),
            Ask::KeepLibrary => self.backups.dismiss(),
            Ask::ConfirmRestore => {
                if let Some(id) = self.backups.pending().map(str::to_owned) {
                    let goal = libraries::Goal::Restore(id);
                    self.ask_switch(goal, http::leave::Reason::Restore, backend);
                }
            }
            Ask::SubmitPassphrase => {
                let (current, next) = self.titlebar_view.take_phrases();
                self.security.change(backend, current, next);
            }
            Ask::KeepPassphrase => {
                self.titlebar_view.take_phrases();
                self.security.clear_refusal();
                self.dialogs
                    .close(ctx, crate::titlebar::preferences::change_id());
            }
        }
    }

    pub(super) fn adopt_draft(&mut self, ctx: &egui::Context, backend: &Backend) {
        if let Some(settings) = self.titlebar_view.adopt() {
            self.settings = settings;
            self.language = self.settings.locale.language();
            self.apply_look(ctx);
            self.save_settings(backend);
            crate::api::backups::keep_launch_copies(backend, self.settings.automatic_backups);
        }
        if let Some(lines) = self.titlebar_view.adopt_preview_lines(self.preview_lines) {
            self.set_preview_lines(lines, backend);
        }
        if let Some(rows) = self
            .titlebar_view
            .adopt_variables(self.security.variables())
        {
            self.security.save_variables(backend, rows);
            self.revision.bump();
        }
    }

    pub(super) fn close_preferences(&mut self, ctx: &egui::Context) {
        self.titlebar_view.close_preferences();
        self.dialogs.close(ctx, crate::titlebar::preferences::id());
    }

    pub(super) fn apply_libraries(
        &mut self,
        asked: crate::titlebar::libraries::Ask,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::libraries::Ask;
        use libraries::Goal;
        match asked {
            Ask::Close => {
                self.libraries.dismiss_deletion();
                self.dialogs.close(ctx, crate::titlebar::libraries::id());
            }
            Ask::Create(name) => {
                self.ask_switch(Goal::Create(name), http::leave::Reason::Switch, backend);
            }
            Ask::Open(id) => {
                self.ask_switch(Goal::Open(id), http::leave::Reason::Switch, backend);
            }
            Ask::Rename { id, name } => self.libraries.rename(backend, id, name),
            Ask::Delete(id) => self.libraries.ask_to_delete(id),
            Ask::KeepIt => self.libraries.dismiss_deletion(),
            Ask::ConfirmDelete => self.libraries.confirm_deletion(backend),
        }
    }

    /// A switch takes what the HTTP tabs hold: asked before the library changes, not after.
    pub(super) fn ask_switch(
        &mut self,
        goal: libraries::Goal,
        reason: http::leave::Reason,
        backend: &Backend,
    ) {
        if self.http.ask_leave(reason) {
            self.switch_library(goal, backend);
        } else {
            self.libraries.hold(goal);
        }
    }

    /// Closes the open library: the gate comes back once it is answered, and what belonged to
    /// that library goes with `reset_library_scope`.
    pub(super) fn switch_library(&mut self, goal: libraries::Goal, backend: &Backend) {
        self.http.leaving(backend);
        match goal {
            libraries::Goal::Open(id) => self.gate.leave_for(api::libraries::open(backend, id)),
            libraries::Goal::Create(name) => {
                self.gate
                    .leave_for(api::libraries::create_and_open(backend, name));
            }
            libraries::Goal::Restore(id) => {
                self.gate.leave_for_copy(api::backups::restore(backend, id));
            }
            libraries::Goal::Install => self.updates.install(backend),
        }
    }

    pub(super) fn apply_palette(
        &mut self,
        asked: crate::palette::Asked,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::palette::Asked;
        use crate::palette::commands::Command;
        use crate::palette::options::Chosen;
        match asked {
            Asked::Type(text) => self.palette.type_text(text, ctx.input(|input| input.time)),
            Asked::Chip(chip) => self.palette.add_chip(chip),
            Asked::Unchip(at) => self.palette.remove_chip(at),
            Asked::Close => self.palette.close(),
            Asked::Copy(id) => {
                self.palette.copying();
                self.notes.copy(backend, id);
            }
            Asked::Choose(chosen) => {
                self.palette.close();
                let area = match chosen {
                    Chosen::Command(Command::Area(area)) => Some(area),
                    Chosen::Command(Command::Request) | Chosen::Request(_) => Some(Area::Http),
                    Chosen::Tool(_) => Some(Area::Tools),
                    // Over whichever area is on screen.
                    Chosen::Command(Command::Preferences | Command::Shortcuts) => None,
                    _ => Some(Area::Notes),
                };
                if let Some(area) = area {
                    self.apply(Action::App(AppAction::ShowArea(area)), ctx, backend);
                }
                self.run_chosen(chosen, ctx, backend);
            }
        }
    }

    pub(super) fn run_chosen(
        &mut self,
        chosen: crate::palette::options::Chosen,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::palette::commands::Command;
        use crate::palette::options::Chosen;
        match chosen {
            Chosen::Note(id) => self.open_in_editor(EditorNext::Note(id)),
            Chosen::Request(id) => {
                let open = crate::http::Action::OpenRequest(id);
                self.apply(Action::Http(open), ctx, backend);
            }
            Chosen::Tool(id) => {
                let open = crate::tools::Action::Open(id);
                self.apply(Action::Tools(open), ctx, backend);
            }
            Chosen::Command(Command::Request) => {
                let draft = crate::http::Action::NewRequest(engine::http::model::RequestKind::Http);
                self.apply(Action::Http(draft), ctx, backend);
            }
            Chosen::Command(Command::Preferences) => {
                let open = crate::titlebar::Action::OpenPreferences;
                self.apply(Action::Menu(open), ctx, backend);
            }
            Chosen::Command(Command::Shortcuts) => {
                let panel = crate::titlebar::about_menu::Panel::Sheet;
                let open = crate::titlebar::Action::OpenHelp(panel);
                self.apply(Action::Menu(open), ctx, backend);
            }
            Chosen::Space(id) => {
                self.choose_space(Some(id));
                self.remember_place(backend);
            }
            Chosen::Command(Command::Create(kind)) => self.new_note(Some(kind)),
            Chosen::Command(Command::Folder) => self.folders.ask_name(self.spaces.active_space()),
            Chosen::Command(Command::Area(_)) => {}
            Chosen::Create(title) => {
                if let Some(place) = self.new_place() {
                    let kind = engine::notes::kind::NoteKind::Snippet;
                    self.editor
                        .draft_titled(backend, (kind, place.0, place.1), title);
                }
            }
        }
    }
}
