//! The application frame: what every frame does, in its order (plan § 3.3.7).

use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use engine::Paths;
use platform::instance::Message;
use platform::window::Geometry;

use crate::backend::Backend;
use crate::inputs::{Inbox, Input, Waker};
use crate::theme::{self, Appearance, Density};

/// What `main` prepares before the window exists.
pub struct Startup {
    pub paths: Paths,
    pub window_file: PathBuf,
    /// The geometry the window was opened at, kept as the last known one.
    pub geometry: Option<Geometry>,
    /// Whether the system asks for less motion; the preference adds to it.
    pub reduced_motion: bool,
    pub inbox: Inbox,
    pub waker: Waker,
    pub inputs: Receiver<Input>,
}

pub struct DevNotesApp {
    backend: Backend,
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
        theme::motion::set_reduced(&cc.egui_ctx, startup.reduced_motion);
        theme::apply(&cc.egui_ctx, Appearance::default(), Density::default());
        let backend = Backend::start(startup.paths, Arc::new(startup.inbox), startup.waker)?;
        Ok(Self {
            backend,
            inputs: startup.inputs,
            window_file: startup.window_file,
            geometry: startup.geometry,
            shown: false,
        })
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
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
                // The events and the global actions are read by the screens they concern.
                Input::Instance(Message::Action(_)) | Input::Engine(_) => {}
            }
        }
        let viewport = ctx.input(|input| input.viewport().clone());
        self.remember(&viewport);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_| {});
    }

    /// The window's colour before anything is drawn over it, from the first frame.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self) {
        let Some(geometry) = self.geometry else {
            return;
        };
        if let Err(error) = platform::window::write(&self.window_file, &geometry) {
            log::warn!("The window geometry was not written: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    use super::*;
    use crate::inputs;

    fn startup(directory: &std::path::Path) -> Startup {
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
}
