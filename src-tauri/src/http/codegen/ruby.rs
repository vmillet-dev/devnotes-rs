//! Ruby's `Net::HTTP`, from the standard library: a request object, its headers and body, sent
//! in a session. It follows no redirect, whatever is asked.

use super::source::single_quoted as quoted;
use super::{CodeBody, CodeRequest};
use crate::http::model::HttpMethod;

fn class(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "Get",
        HttpMethod::Post => "Post",
        HttpMethod::Put => "Put",
        HttpMethod::Patch => "Patch",
        HttpMethod::Delete => "Delete",
        HttpMethod::Head => "Head",
        HttpMethod::Options => "Options",
    }
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut lines = vec![
        "require 'net/http'".to_string(),
        "require 'uri'".to_string(),
        String::new(),
        format!("uri = URI({})", quoted(&request.url)),
        format!("request = Net::HTTP::{}.new(uri)", class(request.method)),
    ];
    for (name, value) in &request.headers {
        lines.push(format!("request[{}] = {}", quoted(name), quoted(value)));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "request.basic_auth({}, {})",
            quoted(user),
            quoted(password)
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => lines.push(format!("request.body = {}", quoted(text))),
        CodeBody::Form(fields) => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(name, value)| format!("{} => {}", quoted(name), quoted(value)))
                .collect();
            lines.push(format!("request.set_form_data({})", pairs.join(", ")));
        }
        CodeBody::Multipart(parts) => {
            let pairs: Vec<String> = parts
                .iter()
                .map(|part| {
                    let value = if part.file {
                        format!("File.open({})", quoted(&part.value))
                    } else {
                        quoted(&part.value)
                    };
                    format!("[{}, {value}]", quoted(&part.name))
                })
                .collect();
            lines.push(format!(
                "request.set_form([{}], 'multipart/form-data')",
                pairs.join(", ")
            ));
        }
        CodeBody::File(path) => {
            lines.push(format!("request.body = File.binread({})", quoted(path)));
        }
    }
    lines.push(
        "response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == 'https') do |http|"
            .to_string(),
    );
    lines.push("  http.request(request)".to_string());
    lines.push("end".to_string());
    lines.push("puts \"#{response.code} #{response.body}\"".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("ruby", case, &write(&request));
        }
    }
}
