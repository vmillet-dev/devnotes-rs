//! PHP with Guzzle 7: one `request` whose options say the headers, the auth, the body and
//! the redirects; an error status is printed, not thrown.

use super::source::single_quoted as quoted;
use super::{CodeBody, CodeRequest};

fn array(entries: &[(String, String)], indent: &str) -> String {
    let inner = format!("{indent}    ");
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, value)| format!("{inner}{} => {value},", quoted(name)))
        .collect();
    format!("[\n{}\n{indent}]", lines.join("\n"))
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut options: Vec<(String, String)> = Vec::new();
    if !request.headers.is_empty() {
        let headers: Vec<(String, String)> = request
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), quoted(value)))
            .collect();
        options.push(("headers".to_string(), array(&headers, "    ")));
    }
    if let Some((user, password)) = &request.basic {
        options.push((
            "auth".to_string(),
            format!("[{}, {}]", quoted(user), quoted(password)),
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => options.push(("body".to_string(), quoted(text))),
        CodeBody::Form(fields) => {
            let fields: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), quoted(value)))
                .collect();
            options.push(("form_params".to_string(), array(&fields, "    ")));
        }
        CodeBody::Multipart(parts) => {
            let lines: Vec<String> = parts
                .iter()
                .map(|part| {
                    let contents = if part.file {
                        format!("fopen({}, 'r')", quoted(&part.value))
                    } else {
                        quoted(&part.value)
                    };
                    format!(
                        "        ['name' => {}, 'contents' => {contents}],",
                        quoted(&part.name)
                    )
                })
                .collect();
            options.push((
                "multipart".to_string(),
                format!("[\n{}\n    ]", lines.join("\n")),
            ));
        }
        CodeBody::File(path) => {
            options.push(("body".to_string(), format!("fopen({}, 'r')", quoted(path))));
        }
    }
    options.push((
        "allow_redirects".to_string(),
        request.follow_redirects.to_string(),
    ));
    options.push(("http_errors".to_string(), "false".to_string()));
    format!(
        "<?php\n\nrequire 'vendor/autoload.php';\n\n$client = new GuzzleHttp\\Client();\n$response = $client->request({}, {}, {});\necho $response->getStatusCode(), ' ', $response->getBody();",
        quoted(request.method.as_str()),
        quoted(&request.url),
        array(&options, "")
    )
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("guzzle", case, &write(&request));
        }
    }
}
