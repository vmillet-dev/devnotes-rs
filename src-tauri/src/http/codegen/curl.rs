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
        CodeBody::Text(text) => arguments.push(format!("--data {}", shell_quoted(text))),
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
                let value = if part.file {
                    format!("{}=@{}", part.name, part.value)
                } else {
                    format!("{}={}", part.name, part.value)
                };
                arguments.push(format!("--form {}", shell_quoted(&value)));
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

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("curl", case, &write(&request, true));
        }
        let (_, get) = &golden::cases()[0];
        golden::check("curl", "get-one-line", &write(get, false));
    }
}
