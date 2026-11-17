//! A gap, a radius or a text size typed outside the theme would escape its scale: the sweep
//! reads every source file for the calls that take one.

use std::path::{Path, PathBuf};

/// Calls whose argument at that rank is a gap, a radius or a size.
const CALLS: [(&str, usize); 11] = [
    ("CornerRadius::same(", 0),
    ("add_space(", 0),
    ("Margin::symmetric(", 0),
    ("Margin::same(", 0),
    ("FontId::new(", 0),
    ("FontId::proportional(", 0),
    ("FontId::monospace(", 0),
    ("text(", 2),
    ("one_line(", 2),
    ("arrowed(", 3),
    (".size(", 0),
];

#[test]
fn no_gap_radius_or_text_size_is_written_outside_the_theme() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    for file in rust_files(&source) {
        if file.components().any(|part| part.as_os_str() == "theme") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        for (number, line) in text.lines().enumerate() {
            if writes_a_spacing(line) || writes_a_size_and_weight(line) {
                offenders.push(format!("{}:{}", file.display(), number + 1));
            }
        }
        for call in literal_calls(&text) {
            offenders.push(format!("{}: {call}", file.display()));
        }
    }

    assert!(
        offenders.is_empty(),
        "gaps, radii or sizes outside theme::tokens: {offenders:#?}"
    );
}

/// An item spacing, or a gap constant of the interface's (`f32`, a point or more), set to a number
/// other than zero: a factor (`0.7` em) or a count of QR modules is not a gap.
fn writes_a_spacing(line: &str) -> bool {
    let line = line.trim();
    let named_gap =
        (line.starts_with("const ") || line.contains(" const ")) && line.contains(": f32 =");
    let named_gap = named_gap
        && ["GAP", "PAD", "MARGIN", "INSET", "SPACING"]
            .iter()
            .any(|name| line.split(':').next().unwrap_or_default().contains(name));
    let spacing = line.contains("item_spacing") && line.contains('=');
    if !spacing && !named_gap {
        return false;
    }
    let value = line.split('=').nth(1).unwrap_or_default();
    numbers(value).any(|number| {
        if named_gap {
            number >= 1.0
        } else {
            number != 0.0
        }
    })
}

/// A text's `(size, weight)` with the size typed: `(12.5, 600.0)`, handed to a helper that
/// draws text. A call's parenthesis (`vec2(12.0, 400.0)`) is a size on screen, not this.
fn writes_a_size_and_weight(line: &str) -> bool {
    const WEIGHTS: [&str; 7] = [
        "300.0", "400.0", "500.0", "600.0", "650.0", "700.0", "800.0",
    ];
    line.match_indices('(').any(|(at, _)| {
        let called = line[..at]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_alphanumeric() || "_)]".contains(before));
        let inside = line[at + 1..].split(')').next().unwrap_or_default();
        let parts: Vec<&str> = inside.split(',').map(str::trim).collect();
        !called && parts.len() == 2 && parse(parts[0]).is_some() && WEIGHTS.contains(&parts[1])
    })
}

/// The calls of `CALLS` whose argument is a number other than zero, written whole.
fn literal_calls(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (call, rank) in CALLS {
        for (at, _) in text.match_indices(call) {
            let open = at + call.len();
            let arguments = arguments(&text[open..]);
            let Some(argument) = arguments.get(rank) else {
                continue;
            };
            // One line's tuple: `(12.5, 400.0)` is a size and a weight.
            let argument = argument.trim().trim_start_matches('(');
            let first = argument.split(',').next().unwrap_or_default().trim();
            if parse(first).is_some_and(|number| number != 0.0) {
                found.push(format!("{call}{}", arguments.join(",")));
            }
        }
    }
    found
}

/// A call's top-level arguments, from just after its parenthesis.
fn arguments(rest: &str) -> Vec<String> {
    let mut depth = 0_u32;
    let mut current = String::new();
    let mut all = Vec::new();
    for character in rest.chars() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => break,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                all.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    all.push(current);
    all
}

fn numbers(text: &str) -> impl Iterator<Item = f64> + '_ {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || "._".contains(character)))
        .filter_map(parse)
}

fn parse(token: &str) -> Option<f64> {
    let token = token.trim().trim_end_matches("_f32");
    if !token.starts_with(|character: char| character.is_ascii_digit()) {
        return None;
    }
    token.parse().ok()
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn the_sweep_sees_a_typed_gap_radius_or_size() {
    assert!(writes_a_spacing("ui.spacing_mut().item_spacing.x = 8.0;"));
    assert!(writes_a_spacing("const GAP: f32 = 10.0;"));
    assert!(!writes_a_spacing("ui.spacing_mut().item_spacing.y = 0.0;"));
    assert!(!writes_a_spacing("const GAP: f32 = gap::INSET;"));
    assert_eq!(literal_calls("CornerRadius::same(6)").len(), 1);
    assert_eq!(
        literal_calls("widgets::text(ui, &words, 12.5, 400.0, family, ink)").len(),
        1
    );
    assert_eq!(
        literal_calls("widgets::one_line(ui, &words, (12.5, 400.0), ink, 80.0)").len(),
        1
    );
    assert!(literal_calls("CornerRadius::same(radius::CONTROL)").is_empty());
    assert!(writes_a_size_and_weight(
        "paragraph(ui, &words, (13.0, 600.0), ink);"
    ));
    assert!(!writes_a_size_and_weight(
        "paragraph(ui, &words, (font::READING, 600.0), ink);"
    ));
    assert!(!writes_a_size_and_weight("let size = vec2(12.0, 400.0);"));
    assert!(literal_calls("widgets::text(ui, &words, font::BODY, 400.0, f, ink)").is_empty());
}
