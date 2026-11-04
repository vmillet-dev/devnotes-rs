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

use egui::accesskit::{Node, NodeId, TreeUpdate};
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
pub struct Tree {
    nodes: Vec<(NodeId, Node)>,
    focus: Option<NodeId>,
}

impl Tree {
    pub fn find(&self, author_id: &str) -> Option<&Node> {
        self.nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.author_id() == Some(author_id))
    }

    /// The control that has the focus, as a failure names it.
    pub fn focused(&self) -> String {
        let node = self
            .nodes
            .iter()
            .find(|(id, _)| Some(*id) == self.focus)
            .map(|(_, node)| node);
        match node {
            Some(node) => format!(
                "{} ({:?} {:?})",
                node.author_id().unwrap_or("no author_id"),
                node.role(),
                node.label().unwrap_or_default()
            ),
            None => "nothing".to_owned(),
        }
    }

    pub fn has(&self, author_id: &str) -> bool {
        self.find(author_id).is_some()
    }

    pub fn count(&self, author_id: &str) -> usize {
        self.all(author_id).len()
    }

    /// Every node wearing `author_id`, in reading order: top to bottom, then left to right.
    pub fn all(&self, author_id: &str) -> Vec<&Node> {
        let mut found: Vec<&Node> = self
            .nodes
            .iter()
            .map(|(_, node)| node)
            .filter(|node| node.author_id() == Some(author_id))
            .collect();
        // The tree is in no order of its own.
        found.sort_by(|a, b| top(a).total_cmp(&top(b)).then(left(a).total_cmp(&left(b))));
        found
    }

    /// What each of them says, as [`Self::text`] does.
    pub fn texts(&self, author_id: &str) -> Vec<String> {
        self.all(author_id)
            .into_iter()
            .map(|node| node.label().or(node.value()).unwrap_or_default().to_owned())
            .collect()
    }

    /// What a node says: its name, or its value for a node that has no name.
    pub fn text(&self, author_id: &str) -> Option<String> {
        let node = self.find(author_id)?;
        node.label().or(node.value()).map(str::to_owned)
    }
}

fn top(node: &Node) -> f64 {
    node.bounds().map_or(f64::INFINITY, |bounds| bounds.y0)
}

fn left(node: &Node) -> f64 {
    node.bounds().map_or(f64::INFINITY, |bounds| bounds.x0)
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
            Response::Tree { accesskit, .. } => match accesskit {
                Some(TreeUpdate { nodes, focus, .. }) => Tree {
                    nodes,
                    focus: Some(focus),
                },
                None => Tree {
                    nodes: Vec::new(),
                    focus: None,
                },
            },
            other => panic!("a tree was asked, {other:?} came back"),
        }
    }

    /// Reads the tree until `holds` is true of it: answers arrive from the core frames later.
    pub fn eventually(&mut self, what: &str, holds: impl Fn(&Tree) -> bool) {
        let until = Instant::now() + PATIENCE;
        loop {
            self.ask(&Request::Settle { max_steps: 10 });
            let tree = self.tree();
            if holds(&tree) {
                return;
            }
            assert!(
                Instant::now() < until,
                "{what}: still not true after {PATIENCE:?}; the focus is on {}",
                tree.focused()
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
        self.click_where(author_id, |_| true);
    }

    /// Clicks the first `author_id` that `which` picks, once it is shown: a row of a list, by
    /// what the user named it.
    pub fn click_where(&mut self, author_id: &str, which: impl Fn(&Node) -> bool) {
        self.eventually(&format!("{author_id} is shown"), |tree| {
            tree.all(author_id).into_iter().any(&which)
        });
        let bounds = self
            .tree()
            .all(author_id)
            .into_iter()
            .find(|node| which(node))
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

    /// The pointer over the first `author_id` in reading order, once it is shown: what a card
    /// shows on hover.
    pub fn hover(&mut self, author_id: &str) {
        self.hover_nth(author_id, 0);
    }

    /// The pointer over the `n`th `author_id` in reading order, counting from 0.
    pub fn hover_nth(&mut self, author_id: &str, n: usize) {
        self.eventually(&format!("{author_id} #{n} is shown"), |tree| {
            tree.count(author_id) > n
        });
        let tree = self.tree();
        let bounds = tree
            .all(author_id)
            .get(n)
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("{author_id} has no place on screen"));
        #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
        let pos = Pos2::new(
            f64::midpoint(bounds.x0, bounds.x1) as f32,
            f64::midpoint(bounds.y0, bounds.y1) as f32,
        );
        self.apply(vec![Event::PointerMoved(pos)]);
    }

    /// Clicks the `n`th `author_id` from the top, counting from 0: a choice among several whose
    /// words are the system's language.
    pub fn click_nth(&mut self, author_id: &str, n: usize) {
        self.eventually(&format!("{author_id} #{n} is shown"), |tree| {
            tree.count(author_id) > n
        });
        let tree = self.tree();
        let target = tree.all(author_id)[n];
        let label = target.label().map(str::to_owned);
        let at = top(target);
        self.click_where(author_id, |node| {
            node.label().map(str::to_owned) == label && (top(node) - at).abs() < 0.5
        });
    }

    /// Through the gate of a blank profile: the phrase twice, and the library opens.
    pub fn create_library(&mut self) {
        self.fill("vault-passphrase", PASSPHRASE);
        self.fill("vault-confirmation", PASSPHRASE);
        self.click("vault-submit");
        self.wait_for("area-notes");
    }

    /// Scrolls `within` down until `author_id` is drawn: a list draws only what is in view.
    pub fn scroll_to(&mut self, within: &str, author_id: &str) {
        self.wait_for(within);
        for _ in 0..40 {
            if self.has(author_id) {
                return;
            }
            let Some(bounds) = self.tree().find(within).and_then(Node::bounds) else {
                break;
            };
            #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
            let pos = Pos2::new(
                f64::midpoint(bounds.x0, bounds.x1) as f32,
                f64::midpoint(bounds.y0, bounds.y1) as f32,
            );
            self.apply(vec![
                Event::PointerMoved(pos),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -240.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        self.wait_for(author_id);
    }

    /// Back to the top of `within`: rows out of view are not drawn, so not counted.
    pub fn scroll_top(&mut self, within: &str) {
        self.wait_for(within);
        let Some(bounds) = self.tree().find(within).and_then(Node::bounds) else {
            return;
        };
        #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
        let pos = Pos2::new(
            f64::midpoint(bounds.x0, bounds.x1) as f32,
            f64::midpoint(bounds.y0, bounds.y1) as f32,
        );
        self.apply(vec![
            Event::PointerMoved(pos),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 100_000.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.ask(&Request::Settle { max_steps: 10 });
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

    /// The window's inside, in points.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.ask(&Request::Resize { width, height });
        self.ask(&Request::Settle { max_steps: 10 });
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
