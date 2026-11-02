//! Spike S1: the window shell. Window created invisible, shown on the first frame; close to
//! tray (hidden, not closed); revealed from another thread — the single-instance socket, a
//! global shortcut or the tray — the R3 question; every input event logged for the driver.
//!
//! `s1-shell --profile <dir> [--tray] [--dialog]`. A second process on the same profile hands
//! its arguments (`reveal`, `hide`, `fullscreen`, `quit`) to the first over the socket and exits.

use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::sync::OnceLock;
use std::time::Instant;

use eframe::egui;
use interprocess::local_socket::traits::{ListenerExt, Stream as _};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, ToNsName};

static STARTED: OnceLock<Instant> = OnceLock::new();
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

fn log(line: &str) {
    let ms = STARTED.get_or_init(Instant::now).elapsed().as_millis();
    println!("[{ms:>6}] {line}");
    let _ = std::io::stdout().flush();
}

/// The socket is named after the canonical profile path: one application per profile (D16).
fn socket_name(profile: &str) -> String {
    let canonical = std::fs::canonicalize(profile).unwrap_or_else(|_| profile.into());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    format!("devnotes-{:016x}.sock", hasher.finish())
}

fn handle(command: &str) {
    let Some(ctx) = CONTEXT.get() else { return };
    log(&format!("socket: {command}"));
    match command {
        "reveal" => {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        "hide" => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false)),
        "fullscreen" => ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true)),
        "windowed" => ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false)),
        "quit" => {
            QUITTING.set(()).ok();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        _ => {}
    }
    // The wake-up itself: a hidden window gets no redraw, only this proxy event.
    ctx.request_repaint();
}

static QUITTING: OnceLock<()> = OnceLock::new();

struct Shell {
    frames: u64,
    shown: bool,
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames += 1;
        if !self.shown {
            self.shown = true;
            log(&format!("first frame; pixels_per_point {}", ctx.pixels_per_point()));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        if ctx.input(|i| i.viewport().close_requested()) && QUITTING.get().is_none() {
            log("close requested: hidden to tray");
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => {
                        log(&format!("key {key:?} physical {physical_key:?} ctrl {} shift {} alt {}", modifiers.ctrl, modifiers.shift, modifiers.alt));
                    }
                    egui::Event::Text(text) => log(&format!("text {text:?}")),
                    egui::Event::Ime(ime) => log(&format!("ime {ime:?}")),
                    _ => {}
                }
            }
            for file in &i.raw.dropped_files {
                log(&format!("dropped {:?}", file.path()));
            }
            if let Some(full) = i.viewport().fullscreen {
                if full {
                    log("fullscreen");
                }
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label(format!("S1 shell — frame {}", self.frames));
        });
    }
}

fn tray_thread() {
    std::thread::spawn(|| {
        // The only GTK thread of the process (D29).
        if let Err(error) = gtk::init() {
            log(&format!("tray: gtk init failed: {error}"));
            return;
        }
        let icon = tray_icon::Icon::from_rgba(vec![232; 16 * 16 * 4], 16, 16).expect("icon");
        let menu = tray_icon::menu::Menu::new();
        let reveal = tray_icon::menu::MenuItem::new("Afficher", true, None);
        menu.append(&reveal).expect("menu");
        match tray_icon::TrayIconBuilder::new().with_icon(icon).with_menu(Box::new(menu)).with_tooltip("DevNotes").build() {
            Ok(_tray) => {
                log("tray: built");
                std::thread::spawn(|| {
                    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().recv() {
                        log(&format!("tray: menu {:?}", event.id));
                        handle("reveal");
                    }
                });
                gtk::main();
            }
            Err(error) => log(&format!("tray: build failed: {error}")),
        }
    });
}

fn main() -> eframe::Result {
    STARTED.get_or_init(Instant::now);
    let args: Vec<String> = std::env::args().collect();
    let profile = args.iter().position(|a| a == "--profile").and_then(|at| args.get(at + 1)).cloned().unwrap_or_else(|| "/tmp".into());
    let name = socket_name(&profile);
    let ns_name = name.clone().to_ns_name::<GenericNamespaced>().expect("name");

    let listener = match ListenerOptions::new().name(ns_name.clone()).create_sync() {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let mut stream = Stream::connect(ns_name).expect("the first instance answers");
            let command = args.iter().skip(1).find(|a| !a.starts_with("--") && **a != profile).cloned().unwrap_or_else(|| "reveal".into());
            writeln!(stream, "{command}").expect("sent");
            log(&format!("second instance: handed {command:?} over and exits"));
            return Ok(());
        }
        Err(error) => panic!("socket: {error}"),
    };
    log(&format!("single instance: bound {name}"));
    std::thread::spawn(move || {
        for stream in listener.incoming().filter_map(Result::ok) {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                handle(line.trim());
            }
        }
    });

    // Global shortcut: Ctrl+Shift+D reveals (X11 grabs; no Wayland support in this crate).
    let hotkeys = global_hotkey::GlobalHotKeyManager::new();
    match &hotkeys {
        Ok(manager) => {
            use global_hotkey::hotkey::{Code, HotKey, Modifiers};
            let result = manager.register(HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD));
            log(&format!("global shortcut Ctrl+Shift+D: {result:?}"));
            std::thread::spawn(|| {
                while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().recv() {
                    if event.state == global_hotkey::HotKeyState::Pressed {
                        log("global shortcut pressed");
                        handle("reveal");
                    }
                }
            });
        }
        Err(error) => log(&format!("global shortcut manager: {error}")),
    }

    if args.iter().any(|a| a == "--tray") {
        tray_thread();
    }
    if args.iter().any(|a| a == "--dialog") {
        std::thread::spawn(|| {
            let picked = rfd::FileDialog::new().pick_file();
            log(&format!("dialog answered: {picked:?}"));
        });
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]).with_visible(false).with_title("S1 shell"),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "s1-shell",
        options,
        Box::new(|cc| {
            CONTEXT.set(cc.egui_ctx.clone()).ok();
            log(&format!("eframe created; backend {:?}", cc.wgpu_render_state.as_ref().map(|s| s.adapter.get_info().backend)));
            Ok(Box::new(Shell { frames: 0, shown: false }))
        }),
    )
}
