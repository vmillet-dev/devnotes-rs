//! One constant per translation key, generated from `fr.json`: a misspelled key does not
//! compile. Modules follow the key's segments (`notes.header.search` is
//! `keys::notes::header::SEARCH`), a segment that is a Rust keyword takes `r#`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;

const SOURCE: &str = "translations/fr.json";
/// Where the application's name is written, once.
const MANIFEST: &str = "../engine/Cargo.toml";

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");
    println!("cargo::rerun-if-changed={MANIFEST}");
    println!("cargo::rustc-env=DEVNOTES_DISPLAY_NAME={}", display_name());
    let text = std::fs::read_to_string(SOURCE).expect("fr.json is readable");
    let root: Value = serde_json::from_str(&text).expect("fr.json is JSON");
    let Value::Object(entries) = root else {
        panic!("fr.json is an object of keys");
    };

    let mut out = String::new();
    module(&mut out, &entries, "", 0);
    let target = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("keys.rs");
    std::fs::write(target, out).expect("keys.rs is written");
}

fn module(out: &mut String, entries: &serde_json::Map<String, Value>, prefix: &str, depth: usize) {
    let indent = "    ".repeat(depth);
    let mut constants = BTreeMap::new();
    let mut modules = BTreeMap::new();
    for (segment, value) in entries {
        let path = if prefix.is_empty() {
            segment.clone()
        } else {
            format!("{prefix}.{segment}")
        };
        let clash = match value {
            Value::Object(children) => modules
                .insert(module_name(segment), (path.clone(), children))
                .is_some(),
            Value::String(_) => constants
                .insert(constant_name(segment), path.clone())
                .is_some(),
            other => panic!("{path} is neither a text nor a group: {other}"),
        };
        assert!(
            !clash,
            "{path} makes the same name as one of its neighbours"
        );
    }
    for (name, path) in constants {
        let _ = writeln!(
            out,
            "{indent}pub const {name}: crate::Key = crate::Key(\"{path}\");"
        );
    }
    for (name, (path, children)) in modules {
        let _ = writeln!(out, "{indent}pub mod {name} {{");
        module(out, children, &path, depth + 1);
        let _ = writeln!(out, "{indent}}}");
    }
}

fn display_name() -> String {
    let text = std::fs::read_to_string(MANIFEST).expect("the engine's manifest is readable");
    let manifest: toml::Table = toml::from_str(&text).expect("the engine's manifest is TOML");
    manifest["package"]["metadata"]["devnotes"]["display-name"]
        .as_str()
        .expect("[package.metadata.devnotes] display-name is a string")
        .to_string()
}

/// `unlockTitle` is `UNLOCK_TITLE`, `fr-FR` is `FR_FR`, `1` is `_1`.
fn constant_name(segment: &str) -> String {
    let mut name = String::new();
    let mut previous_lower = false;
    for character in segment.chars() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase() && previous_lower {
                name.push('_');
            }
            name.push(character.to_ascii_uppercase());
            previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
        } else {
            name.push('_');
            previous_lower = false;
        }
    }
    if name.starts_with(|first: char| first.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

fn module_name(segment: &str) -> String {
    let name = constant_name(segment).to_ascii_lowercase();
    match name.as_str() {
        "self" | "super" | "crate" => format!("{name}_"),
        keyword if KEYWORDS.contains(&keyword) => format!("r#{name}"),
        _ => name,
    }
}

const KEYWORDS: [&str; 36] = [
    "as", "async", "await", "box", "break", "const", "continue", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod",
    "move", "mut", "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe",
    "use", "where",
];
