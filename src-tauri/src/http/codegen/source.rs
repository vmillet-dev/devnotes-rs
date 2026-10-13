//! The string literals of the other twelve targets, each language's own escapes.

/// `"…"` with `\\`, `\"` and the control characters escaped: C#, Go, Java, Python, Rust, Swift.
#[must_use]
pub fn double_quoted(text: &str) -> String {
    let mut written = String::with_capacity(text.len() + 2);
    written.push('"');
    for character in text.chars() {
        match character {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written.push('"');
    written
}

/// Kotlin's `"…"`, where `$` begins a template.
#[must_use]
pub fn kotlin(text: &str) -> String {
    double_quoted(text).replace('$', "\\$")
}

/// Elixir's `"…"`, where `#{` begins an interpolation.
#[must_use]
pub fn elixir(text: &str) -> String {
    double_quoted(text).replace("#{", "\\#{")
}

/// `'…'` where only `\\` and `'` are escaped and a line break is kept as typed: PHP and Ruby,
/// whose double quotes interpolate.
#[must_use]
pub fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Dart's `'…'`: escapes as `"…"` elsewhere, and `$` begins an interpolation.
#[must_use]
pub fn dart(text: &str) -> String {
    let inner = double_quoted(text);
    let inner = &inner[1..inner.len() - 1];
    format!(
        "'{}'",
        inner
            .replace("\\\"", "\"")
            .replace('\'', "\\'")
            .replace('$', "\\$")
    )
}

/// The last part of a path: what a file part is sent as.
#[must_use]
pub fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_literal_escapes_what_its_language_reads() {
        assert_eq!(double_quoted("a \"b\"\n\\c"), r#""a \"b\"\n\\c""#);
        assert_eq!(kotlin("$tenant"), r#""\$tenant""#);
        assert_eq!(elixir("#{x}"), r#""\#{x}""#);
        assert_eq!(single_quoted("O'Brien\\"), r"'O\'Brien\\'");
        assert_eq!(dart("O'Brien \"$x\"\n"), r#"'O\'Brien "\$x"\n'"#);
    }
}
