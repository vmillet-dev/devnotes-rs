//! The journeys of the plan (§ 6.2, § 6.3) through the real application, without a window: a
//! frame here is the pass and its tessellation, CPU only, the AccessKit tree a person without a
//! screen reader never pays included. Nothing runs unless `DEVNOTES_PERF` names a profile
//! written by the `reference_profile` bench; it writes to that profile.
//!
//! ```
//! DEVNOTES_PERF=<profile> cargo test --release -p ui --lib app::perf -- --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, TouchPhase, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::db::run_here;

use super::DevNotesApp;
use super::tests::startup;
use crate::action::Action;
use crate::gate::IndexFill;
use crate::state::Area;
use crate::testing::PASSPHRASE;

type Window = Harness<'static, DevNotesApp>;

/// The reference screen, 1 920 × 1 080, less the window's frame.
const SIZE: (f32, f32) = (1904.0, 1041.0);
/// Sixty frames a second, for what waits on egui's clock.
const FRAME: f32 = 1.0 / 60.0;
const PATIENCE: Duration = Duration::from_mins(1);
/// Past it a card is under the header or the hint, where no click reaches it.
const HEADER: f32 = 200.0;
const WHEEL: f32 = 600.0;
/// Past the end of the canvas at [`WHEEL`] a frame.
const SCROLLED_FRAMES: usize = 1_500;
const TERMS: [&str; 20] = [
    "éta",
    "étap",
    "étape",
    "kub",
    "kubectl",
    "SELECT",
    "calls",
    "Vérifier",
    "réplication",
    "zzzz",
    "Note 1",
    "Note 12",
    "Note 123",
    "tag-5",
    "deployment",
    "Authorization",
    "astreinte",
    "bascule",
    "mean_exec",
    "déploiement",
];

#[derive(Default)]
struct Samples(Vec<Duration>);

impl Samples {
    fn at(sorted: &[Duration], share: f64) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "a few thousand samples")]
        let last = (sorted.len() - 1) as f64;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a position within the samples"
        )]
        let at = (last * share).round() as usize;
        sorted[at].as_secs_f64() * 1000.0
    }

    fn report(&self, name: &str) {
        let mut sorted = self.0.clone();
        sorted.sort();
        println!(
            "| {name} | {} | {:.2} | {:.2} | {:.2} | {:.2} |",
            sorted.len(),
            Self::at(&sorted, 0.5),
            Self::at(&sorted, 0.95),
            Self::at(&sorted, 0.99),
            Self::at(&sorted, 1.0),
        );
    }
}

fn once(name: &str, spent: Duration) {
    println!("| {name} | 1 | {:.0} | | | |", spent.as_secs_f64() * 1000.0);
}

/// The process's resident memory, in MiB: without a window, no GPU driver in it.
#[cfg(windows)]
fn resident() -> f64 {
    let script = format!("(Get-Process -Id {}).WorkingSet64", std::process::id());
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .unwrap();
    let bytes: f64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap();
    bytes / 1_048_576.0
}

#[cfg(not(windows))]
fn resident() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let kilobytes: f64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse().ok())
        .unwrap();
    kilobytes / 1024.0
}

fn memory(stage: &str) {
    println!("| resident, {stage} | 1 | {:.0} MiB | | | |", resident());
}

fn launch(profile: &Path) -> Window {
    text::highlight::warm();
    let startup = startup(profile);
    let mut window = Harness::builder()
        .with_size(vec2(SIZE.0, SIZE.1))
        .with_step_dt(FRAME)
        .with_max_steps(100_000)
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    window
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    settle(&mut window);
    window
}

fn settle(window: &mut Window) {
    super::tests::settle(window);
}

/// One frame as the window pays it, but for the GPU.
fn frame(window: &mut Window) -> Duration {
    let started = Instant::now();
    window.step();
    let spent = started.elapsed();
    let shapes = window.output().shapes.clone();
    let pixels = window.output().pixels_per_point;
    let started = Instant::now();
    let _ = window.ctx.tessellate(shapes, pixels);
    spent + started.elapsed()
}

fn has(window: &Window, author_id: &str) -> bool {
    window
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

/// Frames until `holds`, however long the core takes: what it took since `started`.
fn until(window: &mut Window, started: Instant, holds: impl Fn(&Window) -> bool) -> Duration {
    while !holds(window) {
        assert!(started.elapsed() < PATIENCE, "still not there");
        window.step();
    }
    started.elapsed()
}

fn click(window: &mut Window, author_id: &str) {
    window
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
}

fn act(window: &mut Window, action: Action) {
    let ctx = window.ctx.clone();
    let app = window.state_mut();
    app.state.apply(action, &ctx, &app.backend);
    settle(window);
}

fn unlock(window: &mut Window) {
    click(window, "vault-passphrase");
    settle(window);
    window
        .get_by(|node| node.author_id() == Some("vault-passphrase"))
        .type_text(PASSPHRASE);
    settle(window);
    let started = Instant::now();
    click(window, "vault-submit");
    once(
        "unlock → past the gate",
        until(window, started, |w| has(w, "area-notes")),
    );
    once(
        "unlock → first cards",
        until(window, started, |w| has(w, "note-card-open")),
    );
    once(
        "unlock → index whole",
        until(window, started, |w| {
            w.state().state.gate.index == IndexFill::Ready
        }),
    );
    settle(window);
}

/// The wheel over the point `at`, a notch a frame.
fn wheel(window: &mut Window, at: egui::Pos2, frames: usize) -> Samples {
    window.hover_at(at);
    window.step();
    let mut samples = Samples::default();
    for _ in 0..frames {
        window.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -WHEEL),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        });
        samples.0.push(frame(window));
    }
    samples
}

fn card_titles(window: &Window) -> Vec<String> {
    window
        .query_all_by(|node| node.author_id() == Some("note-card-open"))
        .filter(|node| (HEADER..SIZE.1 - HEADER).contains(&node.rect().top()))
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

fn scroll_canvas(window: &mut Window) {
    wheel(window, pos2(1_000.0, 500.0), SCROLLED_FRAMES).report("canvas scrolled");
    let titles = card_titles(window);
    println!("<!-- the canvas stopped on {:?} -->", titles.first());
}

/// From the frame that asks the core to the frame that draws its answer.
fn search(window: &mut Window) {
    let mut answers = Samples::default();
    for term in TERMS {
        let ctx = window.ctx.clone();
        let app = window.state_mut();
        app.state.notes_query.type_search(&ctx, term.to_owned());
        let mut asked = None;
        loop {
            let began = Instant::now();
            window.step();
            let loading = window.state().state.notes_query.is_loading();
            match (loading, asked) {
                (true, None) => asked = Some(began),
                (false, Some(asked)) => {
                    answers.0.push(asked.elapsed());
                    break;
                }
                _ => {}
            }
        }
    }
    answers.report("search, asked → drawn");
    window.state_mut().state.notes_query.clear_search();
    settle(window);
}

fn typing_in_search(window: &mut Window) {
    click(window, "search-input");
    settle(window);
    let mut samples = Samples::default();
    for key in "kubectl rollout restart".chars() {
        window.event(Event::Text(key.to_string()));
        samples.0.push(frame(window));
    }
    samples.report("a key in the search field");
    window.state_mut().state.notes_query.clear_search();
    window.key_press(Key::Escape);
    settle(window);
}

/// The click that opens the card titled `title`, and when it ended.
fn press_card(window: &mut Window, title: &str) -> Instant {
    let rect = window
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
        })
        .rect();
    let pos = rect.min + vec2(60.0, 14.0);
    window.event(Event::PointerMoved(pos));
    window.step();
    for pressed in [true, false] {
        window.event(Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        if pressed {
            window.step();
        }
    }
    let started = Instant::now();
    window.step();
    started
}

fn close_editor(window: &mut Window) {
    click(window, "editor-close");
    settle(window);
}

fn open_ten_notes(window: &mut Window) {
    let mut opened = Samples::default();
    for title in card_titles(window).into_iter().take(10) {
        let started = press_card(window, &title);
        until(window, started, |w| has(w, "editor-close"));
        settle(window);
        opened.0.push(started.elapsed());
        close_editor(window);
    }
    opened.report("a note opened");
}

/// `lines` of Rust, a snippet of its own, opened and typed into, then thrown away.
fn large_snippet(window: &mut Window, title: &str, lines: usize) {
    let engine = window.state().backend().engine();
    let space_id = run_here(engine, engine::spaces::list_spaces).unwrap()[0]
        .id
        .clone();
    let line = "    let étape = migration::run(&mut connection)?; // déploiement du service\n";
    let mut draft = crate::testing::snippet(space_id, title.to_owned());
    draft.language = engine::notes::language::Language::Rs;
    draft.content = line.repeat(lines);
    let made = run_here(engine, move |engine| {
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    let ctx = window.ctx.clone();
    let app = window.state_mut();
    app.state.notes_query.type_search(&ctx, title.to_owned());
    for _ in 0..12 {
        window.step();
    }
    settle(window);
    let started = press_card(window, title);
    let drawn = until(window, started, |w| has(w, "editor-body"));
    once(&format!("{title}: body drawn"), drawn);
    settle(window);
    once(
        &format!("{title}: coloured, window at rest"),
        started.elapsed(),
    );
    click(window, "editor-body");
    settle(window);
    let mut typed = Samples::default();
    for _ in 0..60 {
        window.event(Event::Text("a".to_owned()));
        typed.0.push(frame(window));
    }
    typed.report(&format!("{title}: a key typed"));
    close_editor(window);
    let id = made.id.clone();
    let kept = run_here(window.state().backend().engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap();
    assert_eq!(
        kept.content.len(),
        made.content.len() + 60,
        "every key landed"
    );
    window.state_mut().state.notes_query.clear_search();
    run_here(window.state().backend().engine(), |engine| {
        engine::notes::delete_note(engine, made.id.clone())
    })
    .unwrap();
    settle(window);
}

fn board(window: &mut Window) {
    let engine = window.state().backend().engine();
    let space = run_here(engine, engine::spaces::list_spaces)
        .unwrap()
        .into_iter()
        .find(|space| space.name == "Espace 0")
        .expect("the reference's first space");
    act(
        window,
        Action::Notes(crate::notes::Action::ChooseSpace(Some(space.id))),
    );
    let started = Instant::now();
    click(window, "view-board");
    window.step();
    once(
        "board: first frame of its cards",
        until(window, started, |w| w.state().state.board.view().is_some()),
    );
    let drawn = frame(window);
    once("board: that frame", drawn);
    settle(window);
    once("board: window at rest", started.elapsed());
    eprintln!("TRACE board {:?}", card_titles(window).len());
    wheel(window, pos2(1_000.0, 500.0), 600).report("board scrolled");
    eprintln!("TRACE board after {:?}", card_titles(window).first());
    click(window, "view-list");
    settle(window);
    act(
        window,
        Action::Notes(crate::notes::Action::ChooseSpace(None)),
    );
}

fn engine_time<T>(window: &Window, job: impl FnOnce(&engine::Engine) -> T) -> Duration {
    let started = Instant::now();
    run_here(window.state().backend().engine(), job);
    started.elapsed()
}

fn http(window: &mut Window) {
    let started = Instant::now();
    click(window, "area-option-http");
    once(
        "HTTP rail loaded",
        until(window, started, |w| has(w, "http-node-name")),
    );
    settle(window);
    assert_eq!(window.state().state.settings.area, Area::Http);
    let mut unfolded = Samples::default();
    for _ in 0..10 {
        window
            .get_all_by(|node| node.author_id() == Some("http-node-twisty"))
            .next()
            .expect("a row to unfold")
            .click();
        unfolded.0.push(frame(window));
        settle(window);
    }
    unfolded.report("a rail folder unfolded");
    let rows = |w: &Window| {
        w.query_all_by(|node| node.author_id() == Some("http-node-name"))
            .filter_map(|n| n.accesskit_node().label())
            .collect::<Vec<_>>()
    };
    eprintln!(
        "TRACE rows before {:?}",
        rows(window).iter().take(3).collect::<Vec<_>>()
    );
    wheel(window, pos2(120.0, 500.0), 300).report("HTTP rail scrolled");
    eprintln!(
        "TRACE rows after {} {:?}",
        rows(window).len(),
        rows(window).iter().take(3).collect::<Vec<_>>()
    );
    let mut found = Samples::default();
    for term in ["états", "Lister", "ressources 19", "zzzz", "GET"] {
        found.0.push(engine_time(window, |engine| {
            engine::http::search_http_requests(engine, term.to_owned())
        }));
    }
    found.report("requests searched");
    let mut history = Samples::default();
    for _ in 0..5 {
        history.0.push(engine_time(window, |engine| {
            engine::http::http_history(engine, -120)
        }));
    }
    history.report("HTTP history, first page");
    click(window, "area-option-notes");
    settle(window);
}

fn profile() -> Option<PathBuf> {
    std::env::var_os("DEVNOTES_PERF").map(PathBuf::from)
}

#[test]
fn journeys() {
    let Some(profile) = profile() else { return };
    let mut window = launch(&profile);
    println!("| Journey | Samples | p50 ms | p95 ms | p99 ms | max ms |");
    println!("| --- | --- | --- | --- | --- | --- |");
    memory("at the gate");
    unlock(&mut window);
    memory("unlocked");
    let mut idle = Samples::default();
    for _ in 0..120 {
        idle.0.push(frame(&mut window));
    }
    idle.report("nothing moves");
    scroll_canvas(&mut window);
    memory("canvas scrolled");
    typing_in_search(&mut window);
    search(&mut window);
    open_ten_notes(&mut window);
    large_snippet(&mut window, "Bench 100 kB", 1_350);
    large_snippet(&mut window, "Bench 1 MB", 13_500);
    board(&mut window);
    http(&mut window);
    crate::testing::close_library(window.state().backend());
}
