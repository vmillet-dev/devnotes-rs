//! Swift's `URLSession`, with async/await: a `URLRequest`, its headers and body. It builds no
//! multipart body — the front says so, and the body is left out — and it follows redirects
//! whatever is asked, as a session does without a delegate.

use super::source::double_quoted as quoted;
use super::{CodeBody, CodeRequest, form_encoded};

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut lines = vec![
        "import Foundation".to_string(),
        String::new(),
        format!(
            "var request = URLRequest(url: URL(string: {})!)",
            quoted(&request.url)
        ),
        format!("request.httpMethod = {}", quoted(request.method.as_str())),
    ];
    for (name, value) in &request.headers {
        lines.push(format!(
            "request.setValue({}, forHTTPHeaderField: {})",
            quoted(value),
            quoted(name)
        ));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "request.setValue(\"Basic \" + Data({}.utf8).base64EncodedString(), forHTTPHeaderField: \"Authorization\")",
            quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::None | CodeBody::Multipart(_) => {}
        CodeBody::Text(text) => {
            lines.push(format!("request.httpBody = Data({}.utf8)", quoted(text)));
        }
        CodeBody::Form(fields) => lines.push(format!(
            "request.httpBody = Data({}.utf8)",
            quoted(&form_encoded(fields))
        )),
        CodeBody::File(path) => lines.push(format!(
            "request.httpBody = try Data(contentsOf: URL(fileURLWithPath: {}))",
            quoted(path)
        )),
    }
    lines.push("let (data, response) = try await URLSession.shared.data(for: request)".to_string());
    lines.push(
        "print((response as! HTTPURLResponse).statusCode, String(decoding: data, as: UTF8.self))"
            .to_string(),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("swift", case, &write(&request));
        }
    }
}
