//! Java's `java.net.http.HttpClient` (Java 11): a client that follows redirects only when
//! asked, one builder for the request, a body publisher. It builds no multipart body: the
//! front says so, and the body is left out.

use super::source::double_quoted as quoted;
use super::{CodeBody, CodeRequest, form_encoded};

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut imports = vec![
        "java.net.URI",
        "java.net.http.HttpClient",
        "java.net.http.HttpRequest",
        "java.net.http.HttpResponse",
    ];
    let publisher = match &request.body {
        CodeBody::None | CodeBody::Multipart(_) => {
            "HttpRequest.BodyPublishers.noBody()".to_string()
        }
        CodeBody::Text(text) => format!("HttpRequest.BodyPublishers.ofString({})", quoted(text)),
        CodeBody::Form(fields) => format!(
            "HttpRequest.BodyPublishers.ofString({})",
            quoted(&form_encoded(fields))
        ),
        CodeBody::File(path) => {
            imports.push("java.nio.file.Path");
            format!(
                "HttpRequest.BodyPublishers.ofFile(Path.of({}))",
                quoted(path)
            )
        }
    };
    if request.basic.is_some() {
        imports.push("java.util.Base64");
    }
    imports.sort_unstable();

    let mut lines: Vec<String> = imports
        .iter()
        .map(|import| format!("import {import};"))
        .collect();
    lines.push(String::new());
    lines.push("public class Main {".to_string());
    lines.push("    public static void main(String[] args) throws Exception {".to_string());
    lines.push("        HttpClient client = HttpClient.newBuilder()".to_string());
    lines.push(format!(
        "            .followRedirects(HttpClient.Redirect.{})",
        if request.follow_redirects {
            "NORMAL"
        } else {
            "NEVER"
        }
    ));
    lines.push("            .build();".to_string());
    lines.push("        HttpRequest request = HttpRequest.newBuilder()".to_string());
    lines.push(format!(
        "            .uri(URI.create({}))",
        quoted(&request.url)
    ));
    for (name, value) in &request.headers {
        lines.push(format!(
            "            .header({}, {})",
            quoted(name),
            quoted(value)
        ));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "            .header(\"Authorization\", \"Basic \" + Base64.getEncoder().encodeToString({}.getBytes()))",
            quoted(&format!("{user}:{password}"))
        ));
    }
    lines.push(format!(
        "            .method({}, {publisher})",
        quoted(request.method.as_str())
    ));
    lines.push("            .build();".to_string());
    lines.push(
        "        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());"
            .to_string(),
    );
    lines.push(
        "        System.out.println(response.statusCode() + \" \" + response.body());".to_string(),
    );
    lines.push("    }".to_string());
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
            golden::check("java", case, &write(&request));
        }
    }
}
