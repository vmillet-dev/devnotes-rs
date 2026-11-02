//! Rust's reqwest, blocking: a client that follows no redirect unless asked, one builder chain
//! for the request, its answer printed.

use super::source::double_quoted as quoted;
use super::{CodeBody, CodeRequest};
use crate::http::model::HttpMethod;

fn method(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let multipart = matches!(request.body, CodeBody::Multipart(_));
    let mut lines = vec![if multipart {
        "use reqwest::blocking::{Client, multipart};".to_string()
    } else {
        "use reqwest::blocking::Client;".to_string()
    }];
    lines.push(String::new());
    lines.push("fn main() -> Result<(), Box<dyn std::error::Error>> {".to_string());
    if request.follow_redirects {
        lines.push("    let client = Client::new();".to_string());
    } else {
        lines.push("    let client = Client::builder()".to_string());
        lines.push("        .redirect(reqwest::redirect::Policy::none())".to_string());
        lines.push("        .build()?;".to_string());
    }
    if let CodeBody::Multipart(parts) = &request.body {
        lines.push("    let form = multipart::Form::new()".to_string());
        let last = parts.len().saturating_sub(1);
        for (at, part) in parts.iter().enumerate() {
            let end = if at == last { ";" } else { "" };
            if part.file {
                lines.push(format!(
                    "        .file({}, {})?{end}",
                    quoted(&part.name),
                    quoted(&part.value)
                ));
            } else {
                lines.push(format!(
                    "        .text({}, {}){end}",
                    quoted(&part.name),
                    quoted(&part.value)
                ));
            }
        }
    }
    lines.push("    let response = client".to_string());
    lines.push(format!(
        "        .request(reqwest::Method::{}, {})",
        method(request.method),
        quoted(&request.url)
    ));
    for (name, value) in &request.headers {
        lines.push(format!(
            "        .header({}, {})",
            quoted(name),
            quoted(value)
        ));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "        .basic_auth({}, Some({}))",
            quoted(user),
            quoted(password)
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => lines.push(format!("        .body({})", quoted(text))),
        CodeBody::Form(fields) => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(name, value)| format!("({}, {})", quoted(name), quoted(value)))
                .collect();
            lines.push(format!("        .form(&[{}])", pairs.join(", ")));
        }
        CodeBody::Multipart(_) => lines.push("        .multipart(form)".to_string()),
        CodeBody::File(path) => {
            lines.push(format!(
                "        .body(std::fs::File::open({})?)",
                quoted(path)
            ));
        }
    }
    lines.push("        .send()?;".to_string());
    lines.push("    println!(\"{} {}\", response.status(), response.text()?);".to_string());
    lines.push("    Ok(())".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("reqwest", case, &write(&request));
        }
    }
}
