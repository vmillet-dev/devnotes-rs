//! Angular's `HttpClient`, in TypeScript, from a service or a component that injected it as
//! `this.http`. A browser reads no path: a file is a `File` the code declares, to be given one
//! — from an `<input type="file">` —; and it follows redirects whatever is asked.

use super::javascript::{headers, object, string};
use super::{CodeBody, CodeRequest};

/// `scan` for a part named « scan », `file` for a body or a name that is no identifier.
fn identifier(name: &str) -> String {
    let mut written = String::new();
    let mut upper = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if upper && !written.is_empty() {
                written.push(character.to_ascii_uppercase());
            } else {
                written.push(character);
            }
            upper = false;
        } else {
            upper = true;
        }
    }
    match written.chars().next() {
        Some(first) if first.is_ascii_alphabetic() => written,
        _ => "file".to_string(),
    }
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let headers = headers(request);
    let mut lines = Vec::new();
    if !headers.is_empty() {
        lines.push("import { HttpHeaders } from '@angular/common/http';".to_string());
        lines.push(String::new());
    }
    let body = match &request.body {
        CodeBody::None => None,
        CodeBody::Text(text) => Some(string(text)),
        CodeBody::Form(fields) => {
            let entries: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), string(value)))
                .collect();
            Some(format!(
                "new URLSearchParams({}).toString()",
                object(&entries, "    ")
            ))
        }
        CodeBody::Multipart(parts) => {
            for part in parts.iter().filter(|part| part.file) {
                lines.push(format!("declare const {}: File;", identifier(&part.name)));
            }
            lines.push("const form = new FormData();".to_string());
            for part in parts {
                let value = if part.file {
                    identifier(&part.name)
                } else {
                    string(&part.value)
                };
                lines.push(format!("form.append({}, {value});", string(&part.name)));
            }
            Some("form".to_string())
        }
        CodeBody::File(_) => {
            lines.push("declare const file: File;".to_string());
            Some("file".to_string())
        }
    };
    if matches!(request.body, CodeBody::Multipart(_) | CodeBody::File(_)) {
        lines.push(String::new());
    }
    let mut options = Vec::new();
    if !headers.is_empty() {
        options.push((
            "headers".to_string(),
            format!("new HttpHeaders({})", object(&headers, "    ")),
        ));
    }
    if let Some(body) = body {
        options.push(("body".to_string(), body));
    }
    options.push(("responseType".to_string(), string("text")));
    lines.push("this.http".to_string());
    lines.push(format!(
        "  .request({}, {}, {})",
        string(request.method.as_str()),
        string(&request.url),
        object(&options, "  ")
    ));
    lines.push("  .subscribe((response) => console.log(response));".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("angular", case, &write(&request));
        }
    }

    #[test]
    fn a_part_is_declared_under_a_name_typescript_reads() {
        assert_eq!(identifier("scan"), "scan");
        assert_eq!(identifier("id-card"), "idCard");
        assert_eq!(identifier("2fa"), "file");
    }
}
