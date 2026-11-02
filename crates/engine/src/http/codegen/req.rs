//! Elixir's Req: one `Req.request!` whose options say the headers, the auth, the body and the
//! redirects, and that retries nothing.

use super::source::elixir as quoted;
use super::{CodeBody, CodeRequest};

/// `[{"a", "b"}]`: string keys, which an atom keyword list could not all be.
fn pairs(entries: &[(String, String)]) -> String {
    let pairs: Vec<String> = entries
        .iter()
        .map(|(name, value)| format!("{{{}, {value}}}", quoted(name)))
        .collect();
    format!("[{}]", pairs.join(", "))
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut options = vec![
        format!("method: :{}", request.method.as_str().to_lowercase()),
        format!("url: {}", quoted(&request.url)),
    ];
    if !request.headers.is_empty() {
        let headers: Vec<(String, String)> = request
            .headers
            .iter()
            .map(|(name, value)| (name.to_lowercase(), quoted(value)))
            .collect();
        options.push(format!("headers: {}", pairs(&headers)));
    }
    if let Some((user, password)) = &request.basic {
        options.push(format!(
            "auth: {{:basic, {}}}",
            quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => options.push(format!("body: {}", quoted(text))),
        CodeBody::Form(fields) => {
            let fields: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), quoted(value)))
                .collect();
            options.push(format!("form: {}", pairs(&fields)));
        }
        CodeBody::Multipart(parts) => {
            let parts: Vec<(String, String)> = parts
                .iter()
                .map(|part| {
                    let value = if part.file {
                        format!("File.stream!({})", quoted(&part.value))
                    } else {
                        quoted(&part.value)
                    };
                    (part.name.clone(), value)
                })
                .collect();
            options.push(format!("form_multipart: {}", pairs(&parts)));
        }
        CodeBody::File(path) => options.push(format!("body: File.stream!({})", quoted(path))),
    }
    options.push(format!("redirect: {}", request.follow_redirects));
    options.push("retry: false".to_string());
    let options: Vec<String> = options
        .into_iter()
        .map(|option| format!("    {option}"))
        .collect();
    format!(
        "response =\n  Req.request!(\n{}\n  )\n\nIO.puts(\"#{{response.status}} #{{inspect(response.body)}}\")",
        options.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("req", case, &write(&request));
        }
    }
}
