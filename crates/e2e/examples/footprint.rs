//! What the running application holds, read from outside: memory and CPU at each stage of a
//! session on a profile, on the machine's GPU and painting every frame, as a person's window does.
//! A debug build says nothing; the optimised flavour:
//!
//! ```
//! CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0 cargo build -p app --features e2e --target-dir target/perf
//! DEVNOTES_E2E_BIN=target/perf/debug/devnotes DEVNOTES_E2E_GPU=1 DEVNOTES_E2E_PAINT_ALL=1 \
//!   cargo run -p e2e --example footprint -- <profile> [new]
//! ```
//!
//! `new` creates the library on a blank profile; otherwise the profile is unlocked.

use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;
use std::time::{Duration, Instant};

use e2e::{App, PASSPHRASE};

/// Long enough for the window's last repaints and the backup of the day to end.
const REST: Duration = Duration::from_secs(10);
/// What the idle CPU is averaged over.
const IDLE: Duration = Duration::from_secs(20);
const NOTCHES: usize = 1_000;

#[derive(Clone, Copy)]
struct Reading {
    working_set: f64,
    private: f64,
    cpu_seconds: f64,
}

/// Mebibytes and seconds, from the system's own account of the process.
#[cfg(windows)]
fn read(id: u32) -> Reading {
    let script = format!(
        "$p = Get-Process -Id {id}; \"$($p.WorkingSet64) $($p.PrivateMemorySize64) $($p.TotalProcessorTime.TotalSeconds)\""
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .expect("powershell answers");
    let text = String::from_utf8_lossy(&output.stdout).replace(',', ".");
    let numbers: Vec<f64> = text
        .split_whitespace()
        .map(|word| word.parse().expect("a number"))
        .collect();
    Reading {
        working_set: numbers[0] / 1_048_576.0,
        private: numbers[1] / 1_048_576.0,
        cpu_seconds: numbers[2],
    }
}

#[cfg(not(windows))]
fn read(id: u32) -> Reading {
    let status = std::fs::read_to_string(format!("/proc/{id}/status")).expect("the process");
    let kilobytes = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|number| number.parse::<f64>().ok())
            .unwrap_or_default()
            / 1024.0
    };
    let stat = std::fs::read_to_string(format!("/proc/{id}/stat")).expect("the process");
    let ticks = |at: usize| {
        stat.rsplit(')')
            .next()
            .and_then(|rest| rest.split_whitespace().nth(at))
            .and_then(|number| number.parse::<f64>().ok())
            .unwrap_or_default()
    };
    Reading {
        working_set: kilobytes("VmRSS:"),
        private: kilobytes("RssAnon:"),
        cpu_seconds: (ticks(11) + ticks(12)) / 100.0,
    }
}

/// `Dedicated` video memory, in mebibytes, when the system counts it.
#[cfg(windows)]
fn video(id: u32) -> Option<f64> {
    let script = format!(
        "(Get-Counter '\\GPU Process Memory(pid_{id}*)\\Dedicated Usage').CounterSamples | Measure-Object CookedValue -Sum | ForEach-Object {{ $_.Sum }}"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).replace(',', ".");
    let bytes: f64 = text.trim().parse().ok()?;
    Some(bytes / 1_048_576.0)
}

#[cfg(not(windows))]
fn video(_id: u32) -> Option<f64> {
    None
}

/// A line of the table, after the rest and the idle average.
#[expect(
    clippy::disallowed_methods,
    reason = "an average over a duration, not a wait for a condition"
)]
fn report(app: &App, stage: &str) {
    let id = app.id();
    std::thread::sleep(REST);
    let before = read(id);
    std::thread::sleep(IDLE);
    let after = read(id);
    let cores = 100.0 * (after.cpu_seconds - before.cpu_seconds) / IDLE.as_secs_f64();
    let dedicated = video(id).map_or("n/a".to_owned(), |video| format!("{video:.0}"));
    println!(
        "| {stage} | {:.0} | {:.0} | {dedicated} | {cores:.2} % |",
        after.working_set, after.private
    );
}

fn browse(app: &mut App) {
    for _ in 0..5 {
        app.click_nth("note-card-open", 0);
        app.wait_for("editor-close");
        app.click("editor-close");
        app.eventually("the editor closes", |tree| !tree.has("editor-close"));
    }
    app.click("area-option-http");
    app.wait_for("http-rail-title");
    app.click("area-option-tools");
    app.click("area-option-notes");
    app.wait_for("note-card-open");
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let profile = PathBuf::from(arguments.next().expect("the profile"));
    let new = arguments.next().is_some();

    let started = Instant::now();
    let mut app = App::launch("footprint", &profile);
    app.wait_for("vault-passphrase");
    println!("launch → gate: {} ms", started.elapsed().as_millis());
    println!("| Stage | Working set MiB | Private MiB | Dedicated GPU MiB | Idle CPU |");
    println!("| --- | --- | --- | --- | --- |");
    report(&app, "gate");

    let started = Instant::now();
    if new {
        app.create_library();
    } else {
        app.fill("vault-passphrase", PASSPHRASE);
        app.click("vault-submit");
    }
    app.wait_for("note-card-open");
    println!("unlock → first cards: {} ms", started.elapsed().as_millis());
    report(&app, "unlocked, nothing touched");

    browse(&mut app);
    report(&app, "after opening notes and every area");

    for _ in 0..NOTCHES {
        app.wheel("canvas");
    }
    report(&app, "after scrolling the canvas");
    app.quit();
}
