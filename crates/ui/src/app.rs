//! The application frame: what every frame does, in its order (plan § 3.3.7).

use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use engine::{EngineEvent, Paths};
use platform::instance::Message;
use platform::window::Geometry;

use crate::action::AppAction;
use crate::action::{Action, Actions, GlobalAction};
use crate::backend::Backend;
use crate::inputs::{Inbox, Input, Waker};
use crate::services::preferences::Preferences;
use crate::state::AppState;
use crate::state::Area;
use crate::theme;
use crate::{banners, gate, http, keys, notes, titlebar, tools};

/// How long the exit waits for the database queue to empty (D19).
const EXIT_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// What `main` prepares before the window exists.
pub struct Startup {
    pub paths: Paths,
    pub window_file: PathBuf,
    /// The geometry the window was opened at, kept as the last known one.
    pub geometry: Option<Geometry>,
    /// Whether the system asks for less motion.
    pub reduced_motion: bool,
    /// The application's `preferences.json`, read before the window existed.
    pub preferences: Preferences,
    pub inbox: Inbox,
    pub waker: Waker,
    pub inputs: Receiver<Input>,
}

pub struct DevNotesApp {
    backend: Backend,
    state: AppState,
    inputs: Receiver<Input>,
    window_file: PathBuf,
    geometry: Option<Geometry>,
    shown: bool,
}

impl DevNotesApp {
    /// Where the core is born, once the waker can reach the window (D20). The library stays
    /// closed: the first frame is the gate.
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> std::io::Result<Self> {
        startup.waker.fill(&cc.egui_ctx);
        cc.egui_ctx.set_fonts(theme::fonts::definitions());
        // Like the reference's WebView: Ctrl+plus is not a zoom.
        cc.egui_ctx
            .options_mut(|options| options.zoom_with_keyboard = false);
        let mut state = AppState::new(startup.preferences, startup.reduced_motion);
        state.apply_look(&cc.egui_ctx);
        // The grammars take a few hundred milliseconds to load: never on the frame, which is
        // drawn again once they are in.
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            text::highlight::warm();
            ctx.request_repaint();
        });
        let backend = Backend::start(startup.paths, Arc::new(startup.inbox), startup.waker)?;
        state.gate.ask(&backend);
        state.updates.check(&backend, true);
        Ok(Self {
            backend,
            state,
            inputs: startup.inputs,
            window_file: startup.window_file,
            geometry: startup.geometry,
            shown: false,
        })
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// The size and place to reopen at: the last ones the window had unmaximised, so that a
    /// window closed maximised comes back maximised over the size it had before.
    fn remember(&mut self, viewport: &egui::ViewportInfo) {
        if viewport.minimized == Some(true) {
            return;
        }
        if viewport.maximized == Some(true) {
            if let Some(geometry) = &mut self.geometry {
                geometry.maximized = true;
            }
            return;
        }
        if let (Some(outer), Some(inner)) = (viewport.outer_rect, viewport.inner_rect) {
            self.geometry = Some(Geometry {
                x: outer.min.x,
                y: outer.min.y,
                width: inner.width(),
                height: inner.height(),
                maximized: false,
            });
        }
    }
}

/// Brought back from the tray, from under other windows, or from the taskbar.
fn reveal(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

impl DevNotesApp {
    fn over_the_area(&mut self, ui: &egui::Ui, actions: &mut Actions) {
        let mut palette = std::mem::take(&mut self.state.palette_view);
        crate::palette::show(ui.ctx(), &self.state, &mut palette, actions);
        self.state.palette_view = palette;
        // Over the palette as over the canvas: drawn after both.
        let mut view = std::mem::take(&mut self.state.notes_view);
        notes::fill_form(ui, &self.state, &mut view, actions);
        self.state.notes_view = view;
        let mut view = std::mem::take(&mut self.state.http_view);
        http::leave::show(ui, &self.state, &mut view, actions);
        self.state.http_view = view;
    }

    /// The window goes hidden, its queue drained behind it (`on_exit`), or stays for an answer.
    fn close_asked(&mut self, ctx: &egui::Context) {
        if self.state.may_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        if self.state.http.leave().is_asking() {
            reveal(ctx);
        }
    }
}

impl DevNotesApp {
    /// Who reads what the widgets left of the keys, on what is on screen.
    fn dispatch_keys(&self, ctx: &egui::Context) -> Option<Action> {
        let notes_shown = self.state.gate.is_open() && self.state.settings.area == Area::Notes;
        let bindings = &self.state.settings.bindings;
        let canvas = notes::canvas::shortcuts::bound_to(bindings);
        let rail = [keys::rail(bindings)];
        // A modal over the canvas takes its keys: the canvas's stand aside.
        let levels: &[&[keys::Binding]] = if self.state.dialogs.top().is_some() {
            &[]
        } else if self.state.palette.is_open() {
            &[&keys::APPLICATION]
        } else if notes_shown && !self.state.modal_open() {
            &[&keys::NOTES, &rail, &canvas, &keys::APPLICATION]
        } else if notes_shown {
            &[&keys::NOTES, &rail, &keys::APPLICATION]
        } else if self.state.gate.is_open()
            && self.state.settings.area == Area::Http
            && self.state.http.tabs().closing().is_none()
            && !self.state.http.leave().is_asking()
        {
            &[&keys::HTTP, &rail, &keys::APPLICATION]
        } else if self.state.gate.is_open()
            && self.state.settings.area == Area::Tools
            && !self.state.tools_view.is_picking()
            && self.state.http.kept().asked().is_none()
        {
            &[&keys::TOOLS, &rail, &keys::APPLICATION]
        } else {
            &[&keys::APPLICATION]
        };
        keys::dispatch(ctx, levels)
    }
}

impl eframe::App for DevNotesApp {
    /// Runs while the window is hidden too: eframe draws nothing then, so the window is shown
    /// from here, once it exists at its restored geometry.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.shown {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            self.shown = true;
        }
        theme::focus::track(ctx);
        for input in self.inputs.try_iter() {
            match input {
                Input::Instance(Message::Reveal) => reveal(ctx),
                Input::Instance(Message::Action(name)) => match GlobalAction::named(&name) {
                    Some(action) => self.state.apply(Action::Global(action), ctx, &self.backend),
                    None => log::warn!("A second launch asked for an unknown action"),
                },
                Input::Engine(EngineEvent::IndexProgress { done, total }) => {
                    self.state.gate.index_progress(done, total);
                }
                Input::Engine(EngineEvent::IndexReady) => self.state.gate.index_ready(),
                Input::Engine(EngineEvent::Socket(event)) => self.state.http.receive_socket(event),
                Input::Engine(EngineEvent::Console(event)) => {
                    self.state.http.receive_console(event);
                }
                Input::Engine(EngineEvent::Run(event)) => self.state.http.receive_run(event),
            }
        }
        self.state.update(ctx, &self.backend);
        let viewport = ctx.input(|input| input.viewport().clone());
        self.remember(&viewport);
        if viewport.close_requested() {
            self.close_asked(ctx);
        }
    }

    /// Draw, then the keys the widgets left, then the actions: always in this order (§ 3.3.7).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut actions = Actions::default();
        let language = self.state.language;
        egui::Panel::top("titlebar")
            .exact_size(titlebar::HEIGHT)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let mut view = std::mem::take(&mut self.state.titlebar_view);
                titlebar::show(ui, &self.state, &mut view, &mut actions);
                self.state.titlebar_view = view;
            });
        if let Some(notice) = self.state.errors.notice() {
            let notice = notice.clone();
            egui::Panel::top("error-banner")
                .frame(egui::Frame::NONE.fill(ui.visuals().panel_fill))
                .show(ui, |ui| {
                    if banners::error_banner(ui, &notice, language) {
                        actions.push(Action::App(AppAction::DismissError));
                    }
                });
        }
        if let Some((_, status)) = self.state.status.latest() {
            let status = status.clone();
            egui::Panel::top("status-toast")
                .frame(egui::Frame::NONE.fill(ui.visuals().panel_fill))
                .show(ui, |ui| {
                    if banners::status_toast(ui, &status, language) {
                        actions.push(Action::App(AppAction::DismissStatus));
                    }
                });
        }
        // Flush with the window, as the reference: egui's 8 px margin cost the header its one row.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(ui.visuals().panel_fill))
            .show(ui, |ui| {
                if self.state.gate.is_open() {
                    match self.state.settings.area {
                        Area::Notes => {
                            let mut view = std::mem::take(&mut self.state.notes_view);
                            notes::show(ui, &self.state, &mut view, &mut actions);
                            self.state.notes_view = view;
                        }
                        Area::Http => {
                            let mut view = std::mem::take(&mut self.state.http_view);
                            http::show(ui, &self.state, &mut view, &mut actions);
                            self.state.http_view = view;
                        }
                        Area::Tools => {
                            let mut view = std::mem::take(&mut self.state.tools_view);
                            tools::show(ui, &self.state, &mut view, &mut actions);
                            self.state.tools_view = view;
                        }
                    }
                } else {
                    let mut asked = Vec::new();
                    gate::show(
                        ui,
                        &self.state.gate,
                        &mut self.state.gate_view,
                        language,
                        &mut asked,
                    );
                    for action in asked {
                        actions.push(Action::Gate(action));
                    }
                }
            });
        if self.state.gate.is_open() {
            self.over_the_area(ui, &mut actions);
        }
        // The About menu stays at the gate, and what it opens with it.
        let mut view = std::mem::take(&mut self.state.titlebar_view);
        titlebar::dialogs(ui.ctx(), &self.state, &mut view, &mut actions);
        self.state.titlebar_view = view;
        if let Some(action) = self.dispatch_keys(ui.ctx()) {
            actions.push(action);
        }
        for action in actions.drain() {
            self.state.apply(action, ui.ctx(), &self.backend);
        }
    }

    /// The window's colour before anything is drawn over it, from the first frame.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self) {
        self.state.http.leaving(&self.backend);
        if !self.backend.drain(EXIT_LIMIT) {
            log::warn!("The exit did not wait any longer for the database to finish");
        }
        let Some(geometry) = self.geometry else {
            return;
        };
        if let Err(error) = platform::window::write(&self.window_file, &geometry) {
            log::warn!("The window geometry was not written: {error}");
        }
    }
}

#[cfg(test)]
mod captures;

#[cfg(test)]
mod library_panel_tests;

#[cfg(test)]
mod update_prompt_tests;

#[cfg(test)]
mod about_link_tests;

#[cfg(test)]
mod prompt_tests;

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    use super::*;
    use crate::inputs;

    pub(super) fn settle(harness: &mut Harness<'_, DevNotesApp>) {
        crate::testing::settle(harness, DevNotesApp::backend);
    }

    pub(super) fn startup(directory: &std::path::Path) -> Startup {
        startup_with(directory, inputs::channel())
    }

    fn startup_with(
        directory: &std::path::Path,
        (inbox, waker, receiver): (Inbox, Waker, Receiver<Input>),
    ) -> Startup {
        Startup {
            paths: Paths {
                profile: directory.to_path_buf(),
                resources: directory.to_path_buf(),
            },
            window_file: directory.join("window.json"),
            geometry: None,
            reduced_motion: false,
            preferences: Preferences::default(),
            inbox,
            waker,
            inputs: receiver,
        }
    }

    fn visible_requests(harness: &Harness<'_, DevNotesApp>) -> usize {
        harness
            .output()
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .filter(|command| matches!(command, egui::ViewportCommand::Visible(true)))
            .count()
    }

    #[test]
    fn the_window_is_shown_once_its_first_frame_is_ready() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());

        // The harness runs its first frames while it is built: the request is behind us.
        assert!(harness.state().shown);
        for _ in 0..3 {
            harness.step();
            assert_eq!(visible_requests(&harness), 0);
        }
    }

    #[test]
    fn a_second_launch_brings_the_window_to_the_front() {
        let directory = tempfile::tempdir().unwrap();
        let (inbox, waker, receiver) = inputs::channel();
        let posted = inbox.clone();
        let mut harness = Harness::new_eframe(|cc| {
            DevNotesApp::new(cc, startup_with(directory.path(), (inbox, waker, receiver))).unwrap()
        });

        posted.post(Input::Instance(Message::Reveal));
        harness.step();

        let commands: Vec<&egui::ViewportCommand> = harness
            .output()
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .collect();
        assert!(commands.contains(&&egui::ViewportCommand::Visible(true)));
        assert!(commands.contains(&&egui::ViewportCommand::Focus));
    }

    #[test]
    fn the_engine_starts_on_a_profile_with_no_library_yet() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        harness.step();

        let state = engine::db::run_here(
            harness.state().backend().engine(),
            engine::vault::vault_state,
        );

        assert_eq!(state.unwrap(), engine::vault::VaultState::Absent);
    }

    #[test]
    fn a_maximised_window_keeps_the_size_it_had_before() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        let placed = egui::ViewportInfo {
            outer_rect: Some(egui::Rect::from_min_size(
                egui::pos2(10.0, 20.0),
                egui::vec2(820.0, 640.0),
            )),
            inner_rect: Some(egui::Rect::from_min_size(
                egui::pos2(10.0, 50.0),
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let app = harness.state_mut();

        app.remember(&placed);
        app.remember(&egui::ViewportInfo {
            maximized: Some(true),
            ..placed.clone()
        });

        let geometry = app.geometry.unwrap();
        assert!(geometry.maximized);
        assert!((geometry.width - 800.0).abs() < f32::EPSILON);
        assert!((geometry.x - 10.0).abs() < f32::EPSILON);
    }

    pub(super) fn present(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> bool {
        use egui_kittest::kittest::Queryable;
        harness
            .query_by(|node| node.author_id() == Some(author_id))
            .is_some()
    }

    pub(super) fn click(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
        use egui_kittest::kittest::Queryable;
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .click();
        settle(harness);
    }

    pub(super) fn type_into(harness: &mut Harness<'_, DevNotesApp>, author_id: &str, words: &str) {
        use egui_kittest::kittest::Queryable;
        click(harness, author_id);
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .type_text(words);
        settle(harness);
    }

    /// Through the gate, as a first launch does it.
    pub(super) fn open_library(directory: &std::path::Path) -> Harness<'static, DevNotesApp> {
        let startup = startup(directory);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1100.0, 720.0))
            .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
        // The window finds the arrows in the system's fonts, which a bench has not.
        harness
            .ctx
            .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
        settle(&mut harness);
        type_into(&mut harness, "vault-passphrase", crate::testing::PASSPHRASE);
        type_into(
            &mut harness,
            "vault-confirmation",
            crate::testing::PASSPHRASE,
        );
        click(&mut harness, "vault-submit");
        settle(&mut harness);
        assert!(harness.state().state().gate.is_open());
        harness
    }

    #[test]
    fn the_areas_are_offered_only_once_the_library_is_open() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        settle(&mut harness);

        assert!(present(&harness, "theme-toggle"));
        assert!(!present(&harness, "area-switch"));
    }

    pub(super) fn count(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> usize {
        use egui_kittest::kittest::Queryable;
        harness
            .query_all_by(|node| node.author_id() == Some(author_id))
            .count()
    }

    fn seeded_mark(harness: &Harness<'_, DevNotesApp>) -> Option<String> {
        let directory = engine::db::run_here(
            harness.state().backend().engine(),
            engine::libraries::open_library_directory,
        )
        .unwrap();
        let file = directory.join(engine::layout::PREFERENCES);
        Preferences::read(&file)
            .get(crate::services::samples::SEEDED_KEY)
            .map(str::to_owned)
    }

    #[test]
    fn a_new_library_opens_on_its_samples_seeded_once() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());

        assert_eq!(count(&harness, "note-card"), 6);
        // The list is left out of the folders on purpose.
        assert_eq!(count(&harness, "note-card-folder"), 5);
        assert_eq!(seeded_mark(&harness).as_deref(), Some("true"));

        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        app.state.reset_library_scope(&ctx, &app.backend);
        settle(&mut harness);

        assert_eq!(count(&harness, "note-card"), 6);
    }

    #[test]
    fn the_mark_lost_does_not_seed_a_library_that_has_a_space() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        let file = engine::db::run_here(
            harness.state().backend().engine(),
            engine::libraries::open_library_directory,
        )
        .unwrap()
        .join(engine::layout::PREFERENCES);
        std::fs::remove_file(file).unwrap();

        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        app.state.reset_library_scope(&ctx, &app.backend);
        settle(&mut harness);

        assert_eq!(count(&harness, "note-card"), 6);
        assert_eq!(seeded_mark(&harness).as_deref(), Some("skipped"));
    }

    #[test]
    fn the_switch_and_its_keys_change_the_area() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        assert!(present(&harness, "area-notes"));

        click(&mut harness, "area-option-tools");
        assert!(present(&harness, "area-tools"));

        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num3);
        settle(&mut harness);
        assert!(present(&harness, "area-http"));
        assert_eq!(harness.state().state().settings.area, Area::Http);
    }

    #[test]
    fn the_file_menu_is_offered_with_the_library_and_quits_on_the_second_click() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        settle(&mut harness);
        assert!(!present(&harness, "file-menu"));

        let mut harness = open_library(tempfile::tempdir().unwrap().path());
        click(&mut harness, "file-menu");
        click(&mut harness, "file-quit");
        assert!(present(&harness, "file-option-exportAll"), "still open");
        assert!(!present(&harness, "passphrase-prompt"));
    }

    #[test]
    fn an_entry_with_nothing_to_act_on_does_nothing() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        click(&mut harness, "file-menu");

        let idle = harness.get_by(|node| node.author_id() == Some("file-option-exportSelection"));
        assert!(idle.accesskit_node().is_disabled());
        click(&mut harness, "file-option-exportSelection");

        assert!(present(&harness, "file-option-exportAll"), "the menu stays");
        assert!(!harness.state().state().transfer.is_busy());
    }

    #[test]
    fn a_copy_is_said_only_while_the_preference_asks_for_it() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let said = i18n::Message::new(i18n::keys::settings::COPIED);

        app.state.settings.copy_confirmation = false;
        app.state
            .copy_out(&ctx, "echo quiet".to_owned(), said.clone());
        assert!(app.state.status.latest().is_none());

        app.state.settings.copy_confirmation = true;
        app.state.copy_out(&ctx, "echo said".to_owned(), said);
        assert!(app.state.status.latest().is_some());
    }

    #[test]
    fn the_about_menu_stays_at_the_gate() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        settle(&mut harness);
        assert!(!present(&harness, "file-menu"));

        click(&mut harness, "about-open");
        click(&mut harness, "about-about");

        assert!(present(&harness, "about-dialog"));
        click(&mut harness, "about-close");
        assert!(!present(&harness, "about-dialog"));
    }

    #[test]
    fn whats_new_shows_the_changelog_and_marks_what_is_installed() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());

        click(&mut harness, "about-open");
        click(&mut harness, "about-whats-new");
        settle(&mut harness);

        assert!(count(&harness, "news-version") > 0);
        click(&mut harness, "news-close");
        assert!(!present(&harness, "news-close"));
    }

    #[test]
    fn the_guide_walks_jumps_and_closes_from_any_chapter() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        let chapter = |harness: &Harness<'_, DevNotesApp>| {
            let node = harness.get_by(|node| node.author_id() == Some("guide-chapter"));
            node.accesskit_node().value().unwrap_or_default()
        };

        click(&mut harness, "about-open");
        click(&mut harness, "about-getting-started");
        let first = chapter(&harness);
        click(&mut harness, "guide-next");
        let second = chapter(&harness);
        assert_ne!(first, second);

        click(&mut harness, "guide-dot-http");
        assert!(!present(&harness, "guide-next"), "the last has no next");
        assert_ne!(chapter(&harness), second);

        click(&mut harness, "guide-close");
        assert!(!present(&harness, "guide-chapter"));
    }

    #[test]
    fn the_sheet_lists_its_groups_and_escape_closes_it() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());

        click(&mut harness, "about-open");
        click(&mut harness, "about-shortcuts");
        assert!(count(&harness, "shortcut-group") > 2);

        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
        assert!(!present(&harness, "sheet-close"));
    }

    #[test]
    fn the_toggle_switches_to_the_other_theme() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        settle(&mut harness);
        let before = harness.ctx.theme();

        click(&mut harness, "theme-toggle");

        assert_ne!(harness.ctx.theme(), before);
    }

    #[test]
    fn a_banner_stays_until_it_is_dismissed() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        let state = &mut harness.state_mut().state;
        state.errors.report(
            i18n::keys::errors::ATTACHMENTS_LOAD_FAILED,
            &engine::error::AppError::from(engine::error::StorageError::Locked),
        );
        state
            .status
            .report(i18n::Message::new(i18n::keys::attachments::SAVED));
        settle(&mut harness);
        assert!(present(&harness, "error-banner"));
        assert!(present(&harness, "status-toast"));

        click(&mut harness, "error-dismiss");
        click(&mut harness, "status-dismiss");

        assert!(!present(&harness, "error-banner"));
        assert!(!present(&harness, "status-toast"));
    }
}
