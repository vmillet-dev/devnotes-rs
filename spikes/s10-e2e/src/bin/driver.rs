//! The e2e driver of plan § 8.3, reduced: launch the app on a fresh profile with
//! `EGUI_INSPECTION=127.0.0.1:<free port>`, find nodes by `author_id`, click, type, press a key,
//! wait on conditions (`eventually` over `Settle`), screenshot, then really relaunch the app on
//! the same profile and check what it persisted. `driver <app binary> <out dir>`.

use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use egui::accesskit::{Node, NodeId, TreeUpdate};
use egui_inspection::protocol::{Request, Response, read_handshake, read_message, write_message};

struct App {
    child: Child,
    stream: TcpStream,
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port()
}

fn launch(binary: &str, profile: &std::path::Path) -> App {
    let port = free_port();
    let started = Instant::now();
    let child = Command::new(binary)
        .args(["--profile", profile.to_str().expect("utf8")])
        .env("EGUI_INSPECTION", format!("127.0.0.1:{port}"))
        .spawn()
        .expect("launched");
    let stream = loop {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            break stream;
        }
        assert!(started.elapsed() < Duration::from_secs(30), "the inspection port never opened");
        std::thread::sleep(Duration::from_millis(20));
    };
    let version = read_handshake(&stream).expect("handshake");
    println!("connected after {} ms, protocol v{version}", started.elapsed().as_millis());
    App { child, stream }
}

impl App {
    fn ask(&mut self, request: &Request) -> Response {
        write_message(&self.stream, request).expect("sent");
        read_message(BufReader::new(&self.stream)).expect("answered")
    }

    fn tree(&mut self) -> TreeUpdate {
        match self.ask(&Request::GetTree) {
            Response::Tree { accesskit: Some(tree), .. } => tree,
            other => panic!("no tree: {other:?}"),
        }
    }

    fn nodes(&mut self, test_id: &str) -> Vec<(NodeId, Node)> {
        self.tree().nodes.into_iter().filter(|(_, node)| node.author_id() == Some(test_id)).collect()
    }

    /// Re-reads the tree until the condition holds; `Settle` only saves pointless re-reads.
    fn eventually(&mut self, what: &str, mut condition: impl FnMut(&mut Self) -> bool) {
        let started = Instant::now();
        while !condition(self) {
            assert!(started.elapsed() < Duration::from_secs(10), "never: {what}");
            self.ask(&Request::Settle { max_steps: 10 });
        }
    }

    fn click(&mut self, test_id: &str) {
        let (_, node) = self.nodes(test_id).into_iter().next().unwrap_or_else(|| panic!("no {test_id}"));
        let bounds = node.bounds().expect("bounds");
        let at = egui::pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);
        let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        assert!(matches!(self.ask(&Request::ApplyEvents { events: vec![egui::Event::PointerMoved(at), button(true), button(false)] }), Response::Done));
    }

    fn type_text(&mut self, text: &str) {
        assert!(matches!(self.ask(&Request::ApplyEvents { events: vec![egui::Event::Text(text.into())] }), Response::Done));
    }

    fn press(&mut self, key: egui::Key) {
        let event = |pressed| egui::Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers: Default::default() };
        assert!(matches!(self.ask(&Request::ApplyEvents { events: vec![event(true), event(false)] }), Response::Done));
    }

    fn quit(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let binary = args.next().expect("app binary");
    let out = std::path::PathBuf::from(args.next().expect("out dir"));
    std::fs::create_dir_all(&out).expect("out");
    let profile = std::env::temp_dir().join(format!("s10-profile-{}", std::process::id()));
    std::fs::create_dir_all(&profile).expect("profile");
    let started = Instant::now();

    let mut app = launch(&binary, &profile);
    if let Response::Info { label, egui_version } = app.ask(&Request::GetInfo) {
        println!("peer {label:?}, egui {egui_version}");
    }
    app.eventually("the add button", |app| !app.nodes("add-note").is_empty());
    for _ in 0..3 {
        app.click("add-note");
    }
    app.eventually("three cards", |app| app.nodes("note-card").len() == 3);
    println!("step: three cards after three clicks");

    app.click("search");
    app.type_text("déploiement");
    app.press(egui::Key::Enter);
    app.eventually("saved in the profile", |_| std::fs::read_to_string(profile.join("saved.txt")).is_ok_and(|t| t == "déploiement"));
    println!("step: typed, Enter, written to the profile");

    match app.ask(&Request::GetScreenshot { pixels_per_point: Some(1.0) }) {
        Response::Screenshot(png) => {
            std::fs::write(out.join("s10-screenshot.png"), &png.bytes).expect("png");
            println!("step: screenshot {:?}", png.size);
        }
        other => panic!("no screenshot: {other:?}"),
    }
    assert!(matches!(app.ask(&Request::Resize { width: 640, height: 480 }), Response::Done));
    app.quit();

    let mut app = launch(&binary, &profile);
    app.eventually("the search restored after a real relaunch", |app| {
        app.nodes("search").iter().any(|(_, node)| node.value() == Some("déploiement"))
    });
    println!("step: relaunched on the same profile, value restored");
    app.quit();
    std::fs::remove_dir_all(&profile).ok();
    println!("scenario passed in {} ms", started.elapsed().as_millis());
}
