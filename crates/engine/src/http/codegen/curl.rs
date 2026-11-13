//! cURL: `--request`, `--url`, a `--header` each, the body as `--data`, `--data-urlencode`,
//! `--form` or `--data-binary @file`.

use super::{CodeBody, CodeRequest, shell_lines, shell_quoted};
use crate::http::model::HttpMethod;

#[must_use]
pub fn write(request: &CodeRequest, multiline: bool) -> String {
    let mut arguments = Vec::new();
    // `--request HEAD` waits for a body that never comes; `--head` asks for none.
    let first = if request.method == HttpMethod::Head {
        "curl --head".to_string()
    } else {
        format!("curl --request {}", request.method.as_str())
    };
    arguments.push(format!("--url {}", shell_quoted(&request.url)));
    if request.follow_redirects {
        arguments.push("--location".to_string());
    }
    for (name, value) in &request.headers {
        arguments.push(format!(
            "--header {}",
            shell_quoted(&format!("{name}: {value}"))
        ));
    }
    if let Some((user, password)) = &request.basic {
        arguments.push(format!(
            "--user {}",
            shell_quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => arguments.push(format!("--data-raw {}", shell_quoted(text))),
        CodeBody::Form(fields) => {
            for (name, value) in fields {
                arguments.push(format!(
                    "--data-urlencode {}",
                    shell_quoted(&format!("{name}={value}"))
                ));
            }
        }
        CodeBody::Multipart(parts) => {
            for part in parts {
                // `--form` reads a file after `@` or `<`: only a file part is meant to.
                let (option, value) = if part.file {
                    ("--form", format!("{}=@{}", part.name, part.value))
                } else {
                    ("--form-string", format!("{}={}", part.name, part.value))
                };
                arguments.push(format!("{option} {}", shell_quoted(&value)));
            }
        }
        CodeBody::File(path) => {
            arguments.push(format!(
                "--data-binary {}",
                shell_quoted(&format!("@{path}"))
            ));
        }
    }
    shell_lines(&first, &arguments, multiline)
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    /// `--data @file` and `--form a=<file` read a file; the value is sent as it is.
    #[test]
    fn a_value_that_names_a_file_is_sent_as_it_is() {
        let request = |body| CodeRequest {
            method: crate::http::model::HttpMethod::Post,
            url: "https://api.exemple.fr/".to_string(),
            headers: Vec::new(),
            basic: None,
            body,
            follow_redirects: false,
        };

        let text = write(
            &request(CodeBody::Text("@/etc/hostname".to_string())),
            false,
        );
        let form = write(
            &request(CodeBody::Multipart(vec![super::super::CodePart {
                name: "a".to_string(),
                value: "<notes.txt".to_string(),
                file: false,
            }])),
            false,
        );

        assert!(text.contains("--data-raw '@/etc/hostname'"), "{text}");
        assert!(form.contains("--form-string 'a=<notes.txt'"), "{form}");
    }

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("curl", case, &write(&request, true));
        }
        let (_, get) = &golden::cases()[0];
        golden::check("curl", "get-one-line", &write(get, false));
    }
}
