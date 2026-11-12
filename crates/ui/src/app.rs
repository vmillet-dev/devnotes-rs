//! The application frame: what every frame does, in its order (plan § 3.3.7).

use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use engine::{EngineEvent, Paths};
use platform::autostart::{Autostart, NoAutostart};
use platform::dialog::{FileDialog, NoDialog};
use platform::instance::Message;
use platform::notify::{NoNotifier, Notifier};
use platform::shortcuts::{Global, NoShortcuts, Shortcuts};
use platform::tray::{self, Tray};
use platform::updater::{NoUpdater, Updater};
use platform::window::Geometry;

use crate::action::AppAction;
use crate::action::{Action, Actions, GlobalAction};
use crate::backend::Backend;
use crate::inputs::{Inbox, Input, Waker};
use crate::services::autostart::AutostartState;
use crate::services::global_keys::GlobalKeys;
use crate::services::preferences::Preferences;
use crate::services::updates::UpdateState;
use crate::state::Area;
use crate::state::{AppState, put_away};
use crate::theme;
use crate::{api, banners, gate, http, keys, notes, titlebar, tools};

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
    pub desktop: Desktop,
}

/// What the operating system answers. `Default` is a desktop that answers nothing.
pub struct Desktop {
    pub tray: Box<dyn Tray>,
    pub shortcuts: Box<dyn Shortcuts>,
    pub autostart: Arc<dyn Autostart + Send + Sync>,
    pub notifier: Arc<dyn Notifier + Send + Sync>,
    pub dialog: Arc<dyn FileDialog + Send + Sync>,
    pub updater: Arc<dyn Updater + Send + Sync>,
}

impl Default for Desktop {
    fn default() -> Self {
        Self {
            tray: Box::new(tray::NoTray),
            shortcuts: Box::new(NoShortcuts),
            autostart: Arc::new(NoAutostart),
            notifier: Arc::new(NoNotifier),
            dialog: Arc::new(NoDialog),
            updater: Arc::new(NoUpdater),
        }
    }
}

pub struct DevNotesApp {
    backend: Backend,
    state: AppState,
    inputs: Receiver<Input>,
    window_file: PathBuf,
    geometry: Option<Geometry>,
    shown: bool,
    tray: Box<dyn Tray>,
    /// The language of the tray's menu.
    tray_language: i18n::Language,
    was_minimized: bool,
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
        state.global_keys = GlobalKeys::with(startup.desktop.shortcuts);
        state.autostart = AutostartState::with(startup.desktop.autostart);
        state.notifier = startup.desktop.notifier;
        state.dialog = startup.desktop.dialog;
        state.updates = UpdateState::with(startup.desktop.updater);
        state.apply_look(&cc.egui_ctx);
        // The grammars take a few hundred milliseconds to load: never on the frame, which is
        // drawn again once they are in.
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            text::highlight::warm();
            ctx.request_repaint();
        });
        let backend = Backend::start(startup.paths, Arc::new(startup.inbox), startup.waker)?;
        // A crash reaches no exit: what it left goes before anyone unlocks anything.
        drop(api::attachments::sweep_copies(&backend));
        state.gate.ask(&backend);
        state.updates.check(&backend, true);
        state.autostart.read(&backend);
        let tray_language = state.language;
        Ok(Self {
            backend,
            state,
            inputs: startup.inputs,
            window_file: startup.window_file,
            geometry: startup.geometry,
            shown: false,
            tray: startup.desktop.tray,
            tray_language,
            was_minimized: false,
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
        if viewport.minimized == Some(true) || viewport.fullscreen == Some(true) {
            return;
        }
        if viewport.maximized == Some(true) {
            if let Some(geometry) = &mut self.geometry {
                geometry.maximized = true;
            }
            return;
        }
        if let Some(inner) = viewport.inner_rect {
            // Native Wayland names no place: nothing reads it back there either.
            let place = viewport
                .outer_rect
                .map_or(egui::Pos2::ZERO, |outer| outer.min);
            self.geometry = Some(Geometry {
                x: place.x,
                y: place.y,
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

/// Whether a person is already looking at the window: the palette then only opens.
fn in_front(ctx: &egui::Context) -> bool {
    ctx.input(|input| {
        let viewport = input.viewport();
        viewport.focused == Some(true) && viewport.minimized != Some(true)
    })
}

impl DevNotesApp {
    /// Read before the window is brought back: afterwards it is always in front.
    fn asked_from_outside(&mut self, action: GlobalAction, ctx: &egui::Context) {
        let action = if action == GlobalAction::Palette && in_front(ctx) {
            GlobalAction::PaletteHere
        } else {
            action
        };
        reveal(ctx);
        self.state.apply(Action::Global(action), ctx, &self.backend);
    }

    fn asked_by_tray(&mut self, asked: tray::Asked, ctx: &egui::Context) {
        match asked {
            tray::Asked::Reveal => reveal(ctx),
            tray::Asked::NewNote => self.asked_from_outside(GlobalAction::NewNote, ctx),
            tray::Asked::Capture => self.asked_from_outside(GlobalAction::Capture, ctx),
            tray::Asked::Palette => self.asked_from_outside(GlobalAction::Palette, ctx),
            tray::Asked::Quit => {
                self.state
                    .apply(Action::Global(GlobalAction::Quit), ctx, &self.backend);
            }
        }
    }

    fn asked_by_a_key(&mut self, key: Global, ctx: &egui::Context) {
        let action = match key {
            Global::Palette => GlobalAction::Palette,
            Global::Capture => GlobalAction::Capture,
            Global::NewNote => GlobalAction::NewNote,
        };
        self.asked_from_outside(action, ctx);
    }

    /// Replaced only when the language moved: a menu rebuilt every frame would close under the
    /// mouse.
    fn keep_the_tray_in_the_language(&mut self) {
        if self.tray_language != self.state.language {
            self.tray_language = self.state.language;
            self.tray
                .set_labels(&crate::services::tray::labels(self.tray_language));
        }
    }

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

    /// Both need a tray: hidden without one, nothing could call the window back.
    fn hides_on_close(&self) -> bool {
        !self.state.quitting && self.state.settings.close_to_tray && self.tray.exists()
    }

    /// To the tray; or hidden, its queue drained behind it (`on_exit`); or kept for an answer.
    /// ⚠️ Kept, it comes up: a hidden window draws nothing, neither the question nor the editor
    /// that has to close.
    fn close_asked(&mut self, ctx: &egui::Context) {
        if self.hides_on_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            put_away(ctx);
            return;
        }
        if self.state.may_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        reveal(ctx);
    }

    /// On the way down only: a window still minimised while it comes back stays.
    fn follow_the_minimize(&mut self, viewport: &egui::ViewportInfo, ctx: &egui::Context) {
        let minimized = viewport.minimized == Some(true);
        if minimized
            && !self.was_minimized
            && self.state.settings.minimize_to_tray
            && self.tray.exists()
        {
            put_away(ctx);
        }
        self.was_minimized = minimized;
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
        } else if notes_shown && !self.state.overlay_open() {
            &[&keys::NOTES, &rail, &keys::EDITOR, &keys::APPLICATION]
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
        let inputs = self.inputs.clone();
        for input in inputs.try_iter() {
            match input {
                Input::Instance(Message::Reveal) => reveal(ctx),
                Input::Instance(Message::Action(name)) => match GlobalAction::named(&name) {
                    Some(action) => self.state.apply(Action::Global(action), ctx, &self.backend),
                    None => log::warn!("A second launch asked for an unknown action"),
                },
                Input::Tray(asked) => self.asked_by_tray(asked, ctx),
                Input::Shortcut(key) => self.asked_by_a_key(key, ctx),
                Input::NoFileDialog => {
                    let said = i18n::Message::new(i18n::keys::errors::NO_FILE_DIALOG);
                    self.state.errors.notify(said);
                }
                Input::Engine(EngineEvent::IndexProgress { done, total }) => {
                    self.state.gate.index_progress(done, total);
                }
                Input::Engine(EngineEvent::IndexReady) => {
                    self.state.gate.index_ready();
                    // What was listed from the first slice is asked again, whole.
                    self.state.revision.bump();
                }
                Input::Engine(EngineEvent::Socket(event)) => self.state.http.receive_socket(event),
                Input::Engine(EngineEvent::Console(event)) => {
                    self.state.http.receive_console(event);
                }
                Input::Engine(EngineEvent::Run(event)) => self.state.http.receive_run(event),
            }
        }
        self.state.update(ctx, &self.backend);
        self.keep_the_tray_in_the_language();
        let listening = self.state.titlebar_view.listens_for_a_global_key();
        self.state.global_keys.update(
            &self.state.settings.bindings,
            listening,
            &mut self.state.errors,
        );
        let viewport = ctx.input(|input| input.viewport().clone());
        self.remember(&viewport);
        self.follow_the_minimize(&viewport, ctx);
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
        drop(api::attachments::sweep_copies(&self.backend));
        if !self.backend.drain(EXIT_LIMIT) {
            log::warn!("The exit did not wait any longer for the database to finish");
        }
        self.state.global_keys.release();
        self.tray.remove();
        if let Some(geometry) = self.geometry
            && let Err(error) = platform::window::write(&self.window_file, &geometry)
        {
            log::warn!("The window geometry was not written: {error}");
        }
        self.state.updates.install_on_the_way_out();
    }
}

#[cfg(test)]
mod captures;

#[cfg(test)]
mod perf;

#[cfg(test)]
mod screens;

#[cfg(test)]
mod area_switch_tests;

#[cfg(test)]
mod library_panel_tests;

#[cfg(test)]
mod update_prompt_tests;

#[cfg(test)]
mod about_link_tests;

#[cfg(test)]
mod prompt_tests;

#[cfg(test)]
mod window_tests;

#[cfg(test)]
mod global_key_tests;

#[cfg(test)]
mod palette_copy_tests;

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
            desktop: Desktop::default(),
        }
    }

    /// Remembers what the application asked of it.
    #[derive(Clone, Default)]
    pub(super) struct FakeTray {
        labels: std::sync::Arc<std::sync::Mutex<Vec<tray::Labels>>>,
        removed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl Tray for FakeTray {
        fn exists(&self) -> bool {
            !self.removed.load(std::sync::atomic::Ordering::Relaxed)
        }

        fn set_labels(&self, labels: &tray::Labels) {
            self.labels.lock().unwrap().push(labels.clone());
        }

        fn remove(&self) {
            self.removed
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub(super) fn with_a_tray(
        directory: &std::path::Path,
    ) -> (Harness<'static, DevNotesApp>, FakeTray, Inbox) {
        let fake = FakeTray::default();
        let (inbox, waker, receiver) = inputs::channel();
        let posted = inbox.clone();
        let mut startup = startup_with(directory, (inbox, waker, receiver));
        startup.desktop.tray = Box::new(fake.clone());
        let harness = Harness::new_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
        (harness, fake, posted)
    }

    pub(super) fn commands(harness: &Harness<'_, DevNotesApp>) -> Vec<egui::ViewportCommand> {
        harness
            .output()
            .viewport_output
            .values()
            .flat_map(|viewport| viewport.commands.clone())
            .collect()
    }

    #[test]
    fn the_tray_brings_the_window_back() {
        let directory = tempfile::tempdir().unwrap();
        let (mut harness, _, posted) = with_a_tray(directory.path());

        posted.post(Input::Tray(tray::Asked::Reveal));
        harness.step();

        let commands = commands(&harness);
        assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
        assert!(commands.contains(&egui::ViewportCommand::Focus));
    }

    #[test]
    fn the_trays_quit_closes_the_window() {
        let directory = tempfile::tempdir().unwrap();
        let (mut harness, _, posted) = with_a_tray(directory.path());

        posted.post(Input::Tray(tray::Asked::Quit));
        harness.step();

        assert!(commands(&harness).contains(&egui::ViewportCommand::Close));
    }

    #[test]
    fn the_trays_palette_opens_over_the_library() {
        let directory = tempfile::tempdir().unwrap();
        let mut harness = open_library(directory.path());
        let ctx = harness.ctx.clone();

        harness
            .state_mut()
            .asked_by_tray(tray::Asked::Palette, &ctx);
        settle(&mut harness);

        assert!(harness.state().state().palette.is_open());
    }

    #[test]
    fn the_trays_menu_follows_the_language_on_screen_and_goes_at_the_exit() {
        let directory = tempfile::tempdir().unwrap();
        let (mut harness, fake, _) = with_a_tray(directory.path());
        let other = match harness.state().state().language {
            i18n::Language::Fr => i18n::Language::En,
            i18n::Language::En => i18n::Language::Fr,
        };

        harness.step();
        assert!(
            fake.labels.lock().unwrap().is_empty(),
            "built with its menu"
        );

        harness.state_mut().state.language = other;
        harness.step();
        harness.step();
        assert_eq!(
            *fake.labels.lock().unwrap(),
            [crate::services::tray::labels(other)]
        );

        eframe::App::on_exit(harness.state_mut());
        assert!(!fake.exists());
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

    fn left_in_the_plain_copies(directory: &std::path::Path) -> std::path::PathBuf {
        let copies = directory.join("open");
        std::fs::create_dir_all(&copies).unwrap();
        std::fs::write(copies.join("left.txt"), "step one").unwrap();
        copies
    }

    #[test]
    fn plain_copies_are_gone_after_the_launch_and_after_the_exit() {
        let directory = tempfile::tempdir().unwrap();
        let copies = left_in_the_plain_copies(directory.path());

        let mut harness =
            Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());
        settle(&mut harness);
        assert!(!copies.exists(), "before anything is unlocked");

        left_in_the_plain_copies(directory.path());
        eframe::App::on_exit(harness.state_mut());
        assert!(!copies.exists());
    }

    #[test]
    fn a_desktop_with_no_file_dialog_says_so() {
        let directory = tempfile::tempdir().unwrap();
        let (mut harness, _, posted) = with_a_tray(directory.path());

        posted.post(Input::NoFileDialog);
        settle(&mut harness);

        assert!(present(&harness, "error-banner"));
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
            .copy_out(&ctx, &app.backend, "echo quiet".to_owned(), said.clone());
        assert!(app.state.status.latest().is_none());

        app.state.settings.copy_confirmation = true;
        app.state
            .copy_out(&ctx, &app.backend, "echo said".to_owned(), said);
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
