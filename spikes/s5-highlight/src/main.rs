//! Spike S5 / D22: how long syntect takes to tokenise 1 MB of JSON and 1 MB of TypeScript,
//! built once with `--features fancy` and once with `--features onig`.

use std::time::Instant;

use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

fn tokenise(set: &SyntaxSet, extension: &str, text: &str) -> usize {
    let syntax = set.find_syntax_by_extension(extension).expect("a syntax");
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut operations = 0;
    for line in text.split_inclusive('\n') {
        let ops = state.parse_line(line, set).expect("parsed");
        for (_, op) in &ops {
            stack.apply(op).expect("scope");
        }
        operations += ops.len();
    }
    operations
}

fn main() {
    let engine = if cfg!(feature = "fancy") { "fancy-regex" } else { "onig" };
    let started = Instant::now();
    let set = two_face::syntax::extra_newlines();
    println!("{engine}: syntax set loaded in {:.0} ms", started.elapsed().as_secs_f64() * 1e3);
    for (extension, path) in [("json", "out/inputs/1mb.json"), ("ts", "out/inputs/1mb.ts")] {
        let text = std::fs::read_to_string(path).expect("input");
        let mut samples = Vec::new();
        let mut operations = 0;
        for _ in 0..7 {
            let started = Instant::now();
            operations = tokenise(&set, extension, &text);
            samples.push(started.elapsed().as_secs_f64() * 1e3);
        }
        let first = samples[0];
        samples.sort_by(f64::total_cmp);
        println!(
            "  {extension:<4} {:>7} bytes {:>6} lines: first {first:6.0} ms  p50 {:6.0} ms  p95(max of 7) {:6.0} ms  ({operations} scope ops)",
            text.len(),
            text.lines().count(),
            samples[3],
            samples[6]
        );
        profile(&set, extension, &text);
    }
}

/// Where the time goes: the slowest lines, and the time to reach a viewport deep in the file.
#[allow(dead_code)]
pub fn profile(set: &SyntaxSet, extension: &str, text: &str) {
    let syntax = set.find_syntax_by_extension(extension).expect("a syntax");
    let mut state = ParseState::new(syntax);
    let mut per_line = Vec::new();
    let mut at_first_screen = 0.0;
    let started = Instant::now();
    for (number, line) in text.split_inclusive('\n').enumerate() {
        let t = Instant::now();
        state.parse_line(line, set).expect("parsed");
        per_line.push((t.elapsed().as_secs_f64() * 1e3, number, line.len()));
        if number == 60 {
            at_first_screen = started.elapsed().as_secs_f64() * 1e3;
        }
    }
    per_line.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total: f64 = per_line.iter().map(|p| p.0).sum();
    let top: f64 = per_line.iter().take(100).map(|p| p.0).sum();
    println!(
        "    first 60 lines {at_first_screen:.1} ms; slowest line {:.1} ms (line {}, {} bytes); 100 slowest lines = {:.0} % of {total:.0} ms",
        per_line[0].0, per_line[0].1, per_line[0].2, top / total * 100.0
    );
}
