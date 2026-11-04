//! Typed text written back as Markdown, escaped only where a character would read as syntax (the
//! reference's `escapeMarkdownText`). ⚠️ Not the escaping of a Markdown library, which escapes
//! every `_ * ~ [ ] \` and turns `& < >` into entities: `{{db_host}}` stopped being a field, and
//! « a -> b » was stored as `a -&gt; b`. A `{{field}}` is left exactly as typed.

/// A tab starting a line is written `&#9;`: Markdown reads it as an indented code block, or
/// drops it on a line that continues a paragraph.
pub fn escape(text: &str, starts_line: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, field, after)) = next_field(rest) {
        escape_run(before, &mut out);
        out.push_str(field);
        rest = after;
    }
    escape_run(rest, &mut out);
    if !starts_line {
        return out;
    }
    let tabs = out.chars().take_while(|glyph| *glyph == '\t').count();
    format!("{}{}", "&#9;".repeat(tabs), &out[tabs..])
}

/// `{{` then anything but braces and line ends, then `}}`.
fn next_field(text: &str) -> Option<(&str, &str, &str)> {
    let mut from = 0;
    while let Some(open) = text[from..].find("{{").map(|at| at + from) {
        let inner = &text[open + 2..];
        let end = inner.find(['{', '}', '\n']).unwrap_or(inner.len());
        if inner[end..].starts_with("}}") {
            let close = open + 2 + end + 2;
            return Some((&text[..open], &text[open..close], &text[close..]));
        }
        from = open + 1;
    }
    None
}

fn escape_run(run: &str, out: &mut String) {
    let glyphs: Vec<char> = run.chars().collect();
    for (at, glyph) in glyphs.iter().enumerate() {
        let before = at.checked_sub(1).map(|at| glyphs[at]);
        let after = glyphs.get(at + 1).copied();
        if needs_escape(*glyph, before, after, &glyphs[at..]) {
            out.push('\\');
        }
        out.push(*glyph);
    }
}

fn needs_escape(glyph: char, before: Option<char>, after: Option<char>, rest: &[char]) -> bool {
    let space_before = before.is_none_or(char::is_whitespace);
    let space_after = after.is_none_or(char::is_whitespace);
    match glyph {
        '\\' => after.is_some_and(|after| after.is_ascii_punctuation()),
        '`' | '[' => true,
        // Emphasis needs a flank: `2 * 3` and `snake_case` are left as they are.
        '*' => !(space_before && space_after),
        '_' => !(space_before && space_after || is_word(before) && is_word(after)),
        // `~/.ssh` is a path; `~~` and `~word` could strike through.
        '~' => !space_after && after != Some('/'),
        '<' => after.is_some_and(|after| after.is_ascii_alphabetic() || "/!?".contains(after)),
        '&' => is_entity(rest),
        _ => false,
    }
}

fn is_word(glyph: Option<char>) -> bool {
    glyph.is_some_and(char::is_alphanumeric)
}

/// `&#9;`, `&#x9;`, `&amp;`: what Markdown would read as one character.
fn is_entity(rest: &[char]) -> bool {
    let Some(end) = rest.iter().position(|glyph| *glyph == ';') else {
        return false;
    };
    let name: String = rest[1..end].iter().collect();
    let lower = name.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("#x") {
        return !hex.is_empty() && hex.chars().all(|glyph| glyph.is_ascii_hexdigit());
    }
    if let Some(decimal) = lower.strip_prefix('#') {
        return !decimal.is_empty() && decimal.chars().all(|glyph| glyph.is_ascii_digit());
    }
    lower.starts_with(|glyph: char| glyph.is_ascii_alphabetic())
        && lower.chars().all(|glyph| glyph.is_ascii_alphanumeric())
}
