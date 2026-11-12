//! What the four JavaScript and TypeScript targets share: their strings, their keys, their
//! object literals, and where a file comes from.

use super::{CodeBody, CodeRequest};

/// `'…'` for a line, a template literal for a text that holds several: a JSON body stays laid
/// out as it was typed.
#[must_use]
pub fn string(text: &str) -> String {
    if text.contains('\n') {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("${", "\\${");
        return format!("`{escaped}`");
    }
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("'{escaped}'")
}

/// A key as written in an object literal: bare when it is an identifier, quoted when not.
#[must_use]
pub fn key(name: &str) -> String {
    let mut chars = name.chars();
    let identifier = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|next| next.is_ascii_alphanumeric() || next == '_' || next == '$');
    if identifier {
        name.to_string()
    } else {
        string(name)
    }
}

/// `{ a: 'b', 'c-d': e }` laid out a key a line at `indent`, the values already written.
#[must_use]
pub fn object(entries: &[(String, String)], indent: &str) -> String {
    if entries.is_empty() {
        return "{}".to_string();
    }
    let inner = format!("{indent}  ");
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, value)| format!("{inner}{}: {value},", key(name)))
        .collect();
    format!("{{\n{}\n{indent}}}", lines.join("\n"))
}

/// The headers, a Basic auth written in with `btoa`.
#[must_use]
pub fn headers(request: &CodeRequest) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .map(|(name, value)| (name.clone(), string(value)))
        .collect();
    if let Some((user, password)) = &request.basic {
        headers.push((
            "Authorization".to_string(),
            format!(
                "`Basic ${{btoa({})}}`",
                string(&format!("{user}:{password}"))
            ),
        ));
    }
    headers
}

/// The last part of a path: what a file part is sent as.
#[must_use]
pub fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Whether the body reads a file from the disk: Node's `openAsBlob` is then imported.
#[must_use]
pub fn reads_files(body: &CodeBody) -> bool {
    match body {
        CodeBody::File(_) => true,
        CodeBody::Multipart(parts) => parts.iter().any(|part| part.file),
        CodeBody::None | CodeBody::Text(_) | CodeBody::Form(_) => false,
    }
}

/// The lines that build a `FormData` named `form` before the call, from Node: a file part is
/// read with `openAsBlob`.
#[must_use]
pub fn form_data(body: &CodeBody) -> Vec<String> {
    let CodeBody::Multipart(parts) = body else {
        return Vec::new();
    };
    let mut lines = vec!["const form = new FormData();".to_string()];
    for part in parts {
        if part.file {
            lines.push(format!(
                "form.append({}, await openAsBlob({}), {});",
                string(&part.name),
                string(&part.value),
                string(&file_name(&part.value))
            ));
        } else {
            lines.push(format!(
                "form.append({}, {});",
                string(&part.name),
                string(&part.value)
            ));
        }
    }
    lines
}

/// The body as the call passes it, the form built before it named `form`.
#[must_use]
pub fn body(body: &CodeBody, indent: &str) -> Option<String> {
    match body {
        CodeBody::None => None,
        CodeBody::Text(text) => Some(string(text)),
        CodeBody::Form(fields) => {
            let entries: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), string(value)))
                .collect();
            Some(format!("new URLSearchParams({})", object(&entries, indent)))
        }
        CodeBody::Multipart(_) => Some("form".to_string()),
        CodeBody::File(path) => Some(format!("await openAsBlob({})", string(path))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_is_quoted_for_a_line_and_a_template_for_several() {
        assert_eq!(string("O'Brien"), r"'O\'Brien'");
        assert_eq!(string("a\\b"), r"'a\\b'");
        assert_eq!(
            string("{\n  \"a\": `${x}`\n}"),
            "`{\n  \"a\": \\`\\${x}\\`\n}`"
        );
    }

    #[test]
    fn a_key_is_bare_when_it_can_be() {
        assert_eq!(key("Authorization"), "Authorization");
        assert_eq!(key("Content-Type"), "'Content-Type'");
        assert_eq!(key("2fa"), "'2fa'");
    }

    #[test]
    fn a_file_is_sent_under_the_last_part_of_its_path() {
        assert_eq!(file_name("C:\\Users\\ada\\scan.pdf"), "scan.pdf");
        assert_eq!(file_name("/tmp/a.zip"), "a.zip");
    }
}
