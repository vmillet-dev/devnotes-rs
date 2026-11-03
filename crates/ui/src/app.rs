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
use crate::{banners, gate, keys, notes, titlebar};

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
        let backend = Backend::start(startup.paths, Arc::new(startup.inbox), startup.waker)?;
        state.gate.ask(&backend);
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

/// An area before its screens: the whole room, named, so that the switch has somewhere to go.
/// Not drawn, it costs nothing, and its state stays in `AppState`.
fn area_shell(ui: &mut egui::Ui, area: Area) {
    let id = match area {
        Area::Notes => "area-notes",
        Area::Tools => "area-tools",
        Area::Http => "area-http",
    };
    let (_, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Main, true, id));
    crate::widgets::address(ui, &response, id);
}

/// Brought back from the tray, from under other windows, or from the taskbar.
fn reveal(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
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
                // Read by the screens they concern, as they are ported.
                Input::Engine(_) => {}
            }
        }
        self.state.update(ctx, &self.backend);
        let viewport = ctx.input(|input| input.viewport().clone());
        self.remember(&viewport);
    }

    /// Draw, then the keys the widgets left, then the actions: always in this order (§ 3.3.7).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut actions = Actions::default();
        let language = self.state.language;
        egui::Panel::top("titlebar")
            .exact_size(titlebar::HEIGHT)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                titlebar::show(
                    ui,
                    self.state.settings.area,
                    self.state.gate.is_open(),
                    language,
                    &mut actions,
                );
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
        egui::CentralPanel::default().show(ui, |ui| {
            if self.state.gate.is_open() {
                match self.state.settings.area {
                    Area::Notes => {
                        let mut view = std::mem::take(&mut self.state.notes_view);
                        notes::show(ui, &self.state, &mut view, &mut actions);
                        self.state.notes_view = view;
                    }
                    area => area_shell(ui, area),
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
        let notes_shown = self.state.gate.is_open() && self.state.settings.area == Area::Notes;
        // A modal over the canvas takes its keys: the canvas's stand aside.
        let levels: &[&[keys::Binding]] = if notes_shown && !self.state.editor.is_open() {
            &[
                &keys::NOTES,
                &notes::canvas::shortcuts::CANVAS,
                &keys::APPLICATION,
            ]
        } else if notes_shown {
            &[&keys::NOTES, &keys::APPLICATION]
        } else {
            &[&keys::APPLICATION]
        };
        if let Some(action) = keys::dispatch(ui.ctx(), levels) {
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

    fn present(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> bool {
        use egui_kittest::kittest::Queryable;
        harness
            .query_by(|node| node.author_id() == Some(author_id))
            .is_some()
    }

    fn click(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
        use egui_kittest::kittest::Queryable;
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .click();
        settle(harness);
    }

    fn type_into(harness: &mut Harness<'_, DevNotesApp>, author_id: &str, words: &str) {
        use egui_kittest::kittest::Queryable;
        click(harness, author_id);
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .type_text(words);
        settle(harness);
    }

    /// Through the gate, as a first launch does it.
    fn open_library(directory: &std::path::Path) -> Harness<'static, DevNotesApp> {
        let startup = startup(directory);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1100.0, 720.0))
            .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
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

    fn count(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> usize {
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
