//! wget: `--method`, a `--header` each, the body as `--body-data` or `--body-file`, what it
//! answers written out. It sends no multipart form: the front says so.

use super::{CodeBody, CodeRequest, form_encoded, shell_lines, shell_quoted};

#[must_use]
pub fn write(request: &CodeRequest, multiline: bool) -> String {
    let first = format!("wget --method {}", request.method.as_str());
    let mut arguments = vec!["--quiet".to_string()];
    if !request.follow_redirects {
        arguments.push("--max-redirect 0".to_string());
    }
    for (name, value) in &request.headers {
        arguments.push(format!(
            "--header {}",
            shell_quoted(&format!("{name}: {value}"))
        ));
    }
    if let Some((user, password)) = &request.basic {
        // Without it, wget sends its credentials only once a server has asked for them.
        arguments.push("--auth-no-challenge".to_string());
        arguments.push(format!("--user {}", shell_quoted(user)));
        arguments.push(format!("--password {}", shell_quoted(password)));
    }
    match &request.body {
        CodeBody::Text(text) => arguments.push(format!("--body-data {}", shell_quoted(text))),
        CodeBody::Form(fields) => {
            arguments.push(format!(
                "--body-data {}",
                shell_quoted(&form_encoded(fields))
            ));
        }
        CodeBody::File(path) => arguments.push(format!("--body-file {}", shell_quoted(path))),
        CodeBody::None | CodeBody::Multipart(_) => {}
    }
    arguments.push("--output-document -".to_string());
    arguments.push(shell_quoted(&request.url));
    shell_lines(&first, &arguments, multiline)
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("wget", case, &write(&request, true));
        }
    }
}
