//! `HTTPie`: the method and the URL, its options, and a `Name:value` item a header; the body
//! raw, as a form, as multipart fields and files, or a file on its standard input.

use super::{CodeBody, CodeRequest, shell_lines, shell_quoted};

#[must_use]
pub fn write(request: &CodeRequest, multiline: bool) -> String {
    let mut arguments = Vec::new();
    if request.follow_redirects {
        arguments.push("--follow".to_string());
    }
    if let Some((user, password)) = &request.basic {
        arguments.push(format!(
            "--auth {}",
            shell_quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::Form(_) => arguments.push("--form".to_string()),
        CodeBody::Multipart(_) => arguments.push("--multipart".to_string()),
        CodeBody::Text(text) => arguments.push(format!("--raw {}", shell_quoted(text))),
        CodeBody::None | CodeBody::File(_) => {}
    }
    for (name, value) in &request.headers {
        arguments.push(shell_quoted(&format!("{name}:{value}")));
    }
    match &request.body {
        CodeBody::Form(fields) => {
            for (name, value) in fields {
                arguments.push(shell_quoted(&format!("{name}={value}")));
            }
        }
        CodeBody::Multipart(parts) => {
            for part in parts {
                let separator = if part.file { "@" } else { "=" };
                arguments.push(shell_quoted(&format!(
                    "{}{separator}{}",
                    part.name, part.value
                )));
            }
        }
        CodeBody::File(path) => arguments.push(format!("< {}", shell_quoted(path))),
        CodeBody::None | CodeBody::Text(_) => {}
    }
    let first = format!(
        "http {} {}",
        request.method.as_str(),
        shell_quoted(&request.url)
    );
    shell_lines(&first, &arguments, multiline)
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("httpie", case, &write(&request, true));
        }
    }
}
