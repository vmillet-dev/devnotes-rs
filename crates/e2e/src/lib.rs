//! The end-to-end driver (plan § 8.3): launches the `e2e` flavour of `devnotes` on a profile of
//! its own and drives it like a person, through egui's inspection protocol. It knows of the
//! application only what its AccessKit tree shows, and finds a control by its `author_id`.
//!
//! The flavour is built apart, so that no other build overwrites it:
//! `cargo build -p app --features e2e --target-dir target/e2e`.

use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use egui::accesskit::{Node, TreeUpdate};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_inspection::protocol::{self, Request, Response};

/// How long a condition may take to come true.
const PATIENCE: Duration = Duration::from_secs(30);

/// A test value, never anyone's phrase.
pub const PASSPHRASE: &str = "correct horse battery staple";

/// Where the flavour is, unless `DEVNOTES_E2E_BIN` says otherwise.
fn binary() -> PathBuf {
    if let Some(path) = std::env::var_os("DEVNOTES_E2E_BIN") {
        return PathBuf::from(path);
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = workspace.join(format!(
        "target/e2e/debug/devnotes{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        binary.exists(),
        "no e2e flavour at {}: cargo build -p app --features e2e --target-dir target/e2e",
        binary.display()
    );
    binary
}

/// Captures and what a failed scenario leaves behind, unless `DEVNOTES_E2E_OUTPUT` says
/// otherwise.
fn output() -> PathBuf {
    std::env::var_os("DEVNOTES_E2E_OUTPUT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/e2e/scenarios"),
        PathBuf::from,
    )
}

/// One step of a scenario, named in the output so that a failure says where it stopped.
pub fn step(name: &str, run: impl FnOnce()) {
    eprintln!("▶ {name}");
    run();
}

/// What the window shows at one frame.
pub struct Tree(Vec<Node>);

impl Tree {
    pub fn find(&self, author_id: &str) -> Option<&Node> {
        self.0
            .iter()
            .find(|node| node.author_id() == Some(author_id))
    }

    pub fn has(&self, author_id: &str) -> bool {
        self.find(author_id).is_some()
    }

    /// What a node says: its name, or its value for a node that has no name.
    pub fn text(&self, author_id: &str) -> Option<String> {
        let node = self.find(author_id)?;
        node.label().or(node.value()).map(str::to_owned)
    }
}

/// The application, running on `profile`: a real process, a real window.
pub struct App {
    scenario: &'static str,
    profile: PathBuf,
    child: Child,
    stream: TcpStream,
}

impl App {
    /// Launches the flavour on `profile` and waits until it answers.
    pub fn launch(scenario: &'static str, profile: &Path) -> Self {
        let port = free_port();
        let mut child = Command::new(binary())
            .arg("--profile")
            .arg(profile)
            .env(
                egui_inspection::INSPECTION_ENV_VAR,
                format!("127.0.0.1:{port}"),
            )
            .stdout(Stdio::null())
            .spawn()
            .expect("the e2e flavour starts");
        let stream = connect(port, &mut child);
        App {
            scenario,
            profile: profile.to_path_buf(),
            child,
            stream,
        }
    }

    /// Closes the window as `Quit` does, waits for the process to end, and launches it again on
    /// the same profile: what a person closing and reopening the application sees.
    #[must_use]
    pub fn relaunch(self) -> Self {
        let scenario = self.scenario;
        let profile = self.profile.clone();
        self.quit();
        App::launch(scenario, &profile)
    }

    /// Asks for `Quit` from a second launch, the way a global shortcut would, and waits for the
    /// process to end, its queue drained.
    pub fn quit(mut self) {
        let asked = Command::new(binary())
            .arg("--profile")
            .arg(&self.profile)
            .args(["--action", "quit"])
            .status()
            .expect("a second launch starts");
        assert!(asked.success(), "the running instance heard no quit");
        let ended = self.child.wait().expect("the application ends");
        assert!(ended.success(), "the application ended with {ended}");
    }

    pub fn tree(&mut self) -> Tree {
        match self.ask(&Request::GetTree) {
            Response::Tree { accesskit, .. } => Tree(
                accesskit
                    .map(|TreeUpdate { nodes, .. }| nodes.into_iter().map(|(_, node)| node))
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            other => panic!("a tree was asked, {other:?} came back"),
        }
    }

    /// Reads the tree until `holds` is true of it: answers arrive from the core frames later.
    pub fn eventually(&mut self, what: &str, holds: impl Fn(&Tree) -> bool) {
        let until = Instant::now() + PATIENCE;
        loop {
            self.ask(&Request::Settle { max_steps: 10 });
            if holds(&self.tree()) {
                return;
            }
            assert!(
                Instant::now() < until,
                "{what}: still not true after {PATIENCE:?}"
            );
        }
    }

    pub fn wait_for(&mut self, author_id: &str) {
        self.eventually(&format!("{author_id} is shown"), |tree| tree.has(author_id));
    }

    pub fn has(&mut self, author_id: &str) -> bool {
        self.ask(&Request::Settle { max_steps: 10 });
        self.tree().has(author_id)
    }

    /// Clicks the middle of a control, once it is shown.
    pub fn click(&mut self, author_id: &str) {
        self.wait_for(author_id);
        let bounds = self
            .tree()
            .find(author_id)
            .and_then(Node::bounds)
            .unwrap_or_else(|| panic!("{author_id} has no place on screen"));
        #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
        let pos = Pos2::new(
            f64::midpoint(bounds.x0, bounds.x1) as f32,
            f64::midpoint(bounds.y0, bounds.y1) as f32,
        );
        let button = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.apply(vec![Event::PointerMoved(pos)]);
        self.apply(vec![button(true)]);
        self.apply(vec![button(false)]);
    }

    /// Types into the control that has the focus.
    pub fn type_text(&mut self, text: &str) {
        self.apply(vec![Event::Text(text.to_owned())]);
    }

    /// Clicks a field, then types into it.
    pub fn fill(&mut self, author_id: &str, text: &str) {
        self.click(author_id);
        self.type_text(text);
    }

    /// A key pressed and released at its place: the area keys read the position, not the letter.
    pub fn press(&mut self, key: Key, modifiers: Modifiers) {
        let event = |pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        self.apply(vec![event(true)]);
        self.apply(vec![event(false)]);
    }

    /// The window as it stands, as `<output>/<scenario>/<name>.png`.
    pub fn capture(&mut self, name: &str) -> PathBuf {
        let png = match self.ask(&Request::GetScreenshot {
            pixels_per_point: Some(1.0),
        }) {
            Response::Screenshot(png) => png,
            other => panic!("a capture was asked, {other:?} came back"),
        };
        let directory = output().join(self.scenario);
        std::fs::create_dir_all(&directory).expect("the capture directory is made");
        let path = directory.join(format!("{name}.png"));
        std::fs::write(&path, png.bytes).expect("the capture is written");
        path
    }

    fn apply(&mut self, events: Vec<Event>) {
        self.ask(&Request::ApplyEvents { events });
    }

    fn ask(&mut self, request: &Request) -> Response {
        match exchange(&mut self.stream, request) {
            Ok(Response::Error { message }) => {
                panic!("the application refused {request:?}: {message}")
            }
            Ok(response) => response,
            Err(error) => panic!("the application stopped answering {request:?}: {error}"),
        }
    }
}

/// A failed scenario leaves its last screen and the application's log beside its captures.
impl Drop for App {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let directory = output().join(self.scenario);
            let _ = std::fs::create_dir_all(&directory);
            let screen = Request::GetScreenshot {
                pixels_per_point: Some(1.0),
            };
            if let Ok(Response::Screenshot(png)) = exchange(&mut self.stream, &screen) {
                let _ = std::fs::write(directory.join("failure.png"), png.bytes);
            }
            keep_logs(&self.profile.join("logs"), &directory);
        }
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn exchange(stream: &mut TcpStream, request: &Request) -> std::io::Result<Response> {
    protocol::write_message(&mut *stream, request)?;
    protocol::read_message(&mut *stream)
}

fn keep_logs(logs: &Path, into: &Path) {
    let Ok(entries) = std::fs::read_dir(logs) else {
        return;
    };
    for entry in entries.flatten() {
        let _ = std::fs::copy(entry.path(), into.join(entry.file_name()));
    }
}

/// A port nobody listens on now: the system's choice, given back for the application to take.
fn free_port() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a local port is free");
    listener.local_addr().expect("the port is known").port()
}

/// The port opens once the window and its adapter exist, half a second on Windows.
fn connect(port: u16, child: &mut Child) -> TcpStream {
    let until = Instant::now() + PATIENCE;
    loop {
        if let Some(status) = child.try_wait().expect("the application is there") {
            panic!("the application ended before it answered: {status}");
        }
        if let Ok(mut stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
            let version = protocol::read_handshake(&mut stream).expect("an inspection handshake");
            assert_eq!(version, protocol::PROTOCOL_VERSION, "another protocol");
            return stream;
        }
        assert!(
            Instant::now() < until,
            "the application never opened its port"
        );
        std::thread::yield_now();
    }
}
