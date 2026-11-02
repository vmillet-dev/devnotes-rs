//! `s6-rich roundtrip` runs the corpus; `s6-rich clipboard` reads `text/html` with arboard.

fn roundtrip() {
    let mut names: Vec<_> = std::fs::read_dir("corpus").expect("corpus").filter_map(|e| e.ok()).map(|e| e.path()).collect();
    names.sort();
    let (mut exact, mut fixed) = (0, 0);
    for path in &names {
        let source = std::fs::read_to_string(path).expect("md");
        let once = s6_rich::markdown::write(&s6_rich::markdown::parse(&source));
        let twice = s6_rich::markdown::write(&s6_rich::markdown::parse(&once));
        let name = path.file_name().expect("name").to_string_lossy();
        exact += usize::from(once == source);
        fixed += usize::from(twice == once);
        if once != source {
            let first = source.lines().zip(once.lines()).position(|(a, b)| a != b);
            let shown = first.map(|n| format!("line {}: {:?} → {:?}", n + 1, source.lines().nth(n).unwrap_or(""), once.lines().nth(n).unwrap_or(""))).unwrap_or_else(|| format!("lengths {} → {}", source.len(), once.len()));
            println!("  differs  {name:<26} {shown}");
        }
    }
    println!("{} files: {exact} written back byte for byte, {fixed} stable after one pass", names.len());
}

fn clipboard() {
    let mut clipboard = arboard::Clipboard::new().expect("a clipboard");
    println!("text: {:?}", clipboard.get_text());
    println!("html: {:?}", clipboard.get().html());
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("clipboard") => clipboard(),
        _ => roundtrip(),
    }
}

#[cfg(test)]
mod samples {
    /// The hand-written samples differ only by the blank line TipTap puts between blocks.
    #[test]
    fn samples_differ_only_by_blank_lines() {
        for name in ["sample-release-en", "sample-release-fr", "sample-welcome-en", "sample-welcome-fr"] {
            let source = std::fs::read_to_string(format!("corpus/{name}.md")).expect("md");
            let once = s6_rich::markdown::write(&s6_rich::markdown::parse(&source));
            let squeeze = |s: &str| s.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
            assert_eq!(squeeze(&source), squeeze(&once), "{name}");
        }
    }
}
