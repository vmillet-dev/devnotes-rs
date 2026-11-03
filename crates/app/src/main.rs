//! The `devnotes` binary: a request-script host when relaunched as one, the application
//! otherwise (plan § 3.3.8, « Lancement »).

// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod arguments;
mod logging;

use std::path::Path;
use std::process::ExitCode;

use engine::app_info::METADATA;
use platform::instance::{self, Claim, Instance, Message};
use platform::paths::{self, Locations};
use platform::window::Geometry;

/// The identity tool's lists in the source tree, for a binary run from `target/`.
const DEVELOPMENT_RESOURCES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/resources");

fn main() -> ExitCode {
    // Before anything else: a script host opens no window and writes no log.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == engine::SCRIPT_HOST_ARG)
    {
        std::process::exit(engine::serve_scripts());
    }

    let arguments = arguments::read(std::env::args_os().skip(1));
    let Some(locations) = arguments
        .profile
        .map(Locations::inside)
        .or_else(|| Locations::installed(paths::DEFAULT_PROFILE))
    else {
        eprintln!("devnotes: the system names no directory for the profile");
        return ExitCode::FAILURE;
    };
    // Before the log: a second launch writes nothing of its own.
    let instance = match instance::claim(&locations.profile) {
        Ok(Claim::First(instance)) => instance,
        Ok(Claim::Taken) => return hand_over(&locations.profile, arguments.action),
        Err(error) => {
            // Fails closed: a second writer would make the index silently wrong.
            eprintln!("devnotes: the profile could not be claimed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let _logger = logging::start(&locations.logs);
    logging::install_panic_hook();

    match run(locations, instance) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("The window did not open: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The profile is open in another process: it reveals its window, or runs `action`.
fn hand_over(profile: &Path, action: Option<String>) -> ExitCode {
    let message = action.map_or(Message::Reveal, Message::Action);
    match instance::send(profile, &message) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("devnotes: the running instance did not answer: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(locations: Locations, instance: Instance) -> eframe::Result {
    let restored = platform::window::read(&locations.window);
    let (inbox, waker, inputs) = ui::inputs::channel();
    let heard = inbox.clone();
    if let Err(error) =
        instance.serve(move |message| heard.post(ui::inputs::Input::Instance(message)))
    {
        log::warn!("Second launches will not be heard: {error}");
    }
    let startup = ui::Startup {
        paths: engine::Paths {
            profile: locations.profile,
            resources: paths::resources(Path::new(DEVELOPMENT_RESOURCES)),
        },
        window_file: locations.window,
        geometry: restored,
        inbox,
        waker,
        inputs,
    };

    eframe::run_native(
        METADATA.name,
        options(restored),
        Box::new(move |cc| Ok(Box::new(ui::DevNotesApp::new(cc, startup)?))),
    )
}

/// Created hidden at the restored geometry, shown by the first frame. eframe's persistence is
/// off: it would write egui's state, text fields included, in clear on disk.
fn options(restored: Option<Geometry>) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(METADATA.name)
        .with_app_id("devnotes")
        .with_inner_size([1_100.0, 720.0])
        .with_min_inner_size([640.0, 480.0])
        .with_visible(false);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../icons/128x128.png")) {
        viewport = viewport.with_icon(icon);
    }
    if let Some(geometry) = restored {
        viewport = viewport
            .with_inner_size([geometry.width, geometry.height])
            .with_position([geometry.x, geometry.y])
            .with_maximized(geometry.maximized);
    }

    eframe::NativeOptions {
        viewport,
        centered: restored.is_none(),
        persist_window: false,
        event_loop_builder: x11_when_possible(),
        ..Default::default()
    }
}

/// D15: under native Wayland, winit 0.30 delivers no dropped file and cannot hide the window,
/// so X11 (`XWayland`) is taken whenever a display offers it.
#[cfg(target_os = "linux")]
fn x11_when_possible() -> Option<eframe::EventLoopBuilderHook> {
    use winit::platform::x11::EventLoopBuilderExtX11;

    std::env::var_os("DISPLAY")?;
    Some(Box::new(|builder| {
        builder.with_x11();
    }))
}

#[cfg(not(target_os = "linux"))]
fn x11_when_possible() -> Option<eframe::EventLoopBuilderHook> {
    None
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_second_launch_hands_its_message_over_and_succeeds() {
        let profile = tempfile::tempdir().unwrap();
        let Ok(Claim::First(first)) = instance::claim(profile.path()) else {
            panic!("the profile was taken");
        };
        let (heard, hearing) = mpsc::channel();
        first
            .serve(move |message| heard.send(message).unwrap())
            .unwrap();

        assert_eq!(hand_over(profile.path(), None), ExitCode::SUCCESS);
        assert_eq!(
            hand_over(profile.path(), Some("palette".to_string())),
            ExitCode::SUCCESS
        );

        let wait = Duration::from_secs(10);
        assert_eq!(hearing.recv_timeout(wait).unwrap(), Message::Reveal);
        assert_eq!(
            hearing.recv_timeout(wait).unwrap(),
            Message::Action("palette".to_string())
        );
    }

    #[test]
    fn a_second_launch_with_nobody_to_answer_fails() {
        let profile = tempfile::tempdir().unwrap();

        assert_eq!(hand_over(profile.path(), None), ExitCode::FAILURE);
    }

    #[test]
    fn a_first_launch_opens_hidden_and_centred_at_the_default_size() {
        let options = options(None);

        assert_eq!(options.viewport.visible, Some(false));
        assert_eq!(
            options.viewport.inner_size,
            Some(egui::vec2(1_100.0, 720.0))
        );
        assert_eq!(
            options.viewport.min_inner_size,
            Some(egui::vec2(640.0, 480.0))
        );
        assert!(options.viewport.icon.is_some());
        assert!(options.centered);
        assert!(!options.persist_window);
    }

    #[test]
    fn a_later_launch_opens_where_the_window_was() {
        let options = options(Some(Geometry {
            x: 40.0,
            y: 30.0,
            width: 900.0,
            height: 600.0,
            maximized: true,
        }));

        assert_eq!(options.viewport.visible, Some(false));
        assert_eq!(options.viewport.inner_size, Some(egui::vec2(900.0, 600.0)));
        assert_eq!(options.viewport.position, Some(egui::pos2(40.0, 30.0)));
        assert_eq!(options.viewport.maximized, Some(true));
        assert!(!options.centered);
    }
}
