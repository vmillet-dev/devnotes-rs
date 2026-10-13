//! Python's requests and httpx: one call whose keywords say the headers, the auth, the body and
//! the redirects; a file is opened where it is read.

use super::source::double_quoted as quoted;
use super::{CodeBody, CodeRequest};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Library {
    Requests,
    Httpx,
}

fn dict(entries: &[(String, String)]) -> String {
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, value)| format!("        {}: {value},", quoted(name)))
        .collect();
    format!("{{\n{}\n    }}", lines.join("\n"))
}

fn write(request: &CodeRequest, library: Library) -> String {
    let name = match library {
        Library::Requests => "requests",
        Library::Httpx => "httpx",
    };
    let mut arguments = vec![quoted(request.method.as_str()), quoted(&request.url)];
    if !request.headers.is_empty() {
        let headers: Vec<(String, String)> = request
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), quoted(value)))
            .collect();
        arguments.push(format!("headers={}", dict(&headers)));
    }
    if let Some((user, password)) = &request.basic {
        arguments.push(format!("auth=({}, {})", quoted(user), quoted(password)));
    }
    // httpx takes raw bytes as `content`, requests as `data`.
    let raw = match library {
        Library::Requests => "data",
        Library::Httpx => "content",
    };
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => arguments.push(format!("{raw}={}", quoted(text))),
        CodeBody::Form(fields) => {
            let fields: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), quoted(value)))
                .collect();
            arguments.push(format!("data={}", dict(&fields)));
        }
        CodeBody::Multipart(parts) => {
            let texts: Vec<(String, String)> = parts
                .iter()
                .filter(|part| !part.file)
                .map(|part| (part.name.clone(), quoted(&part.value)))
                .collect();
            let files: Vec<(String, String)> = parts
                .iter()
                .filter(|part| part.file)
                .map(|part| {
                    (
                        part.name.clone(),
                        format!("open({}, \"rb\")", quoted(&part.value)),
                    )
                })
                .collect();
            if !texts.is_empty() {
                arguments.push(format!("data={}", dict(&texts)));
            }
            if !files.is_empty() {
                arguments.push(format!("files={}", dict(&files)));
            }
        }
        CodeBody::File(path) => {
            arguments.push(format!("{raw}=open({}, \"rb\")", quoted(path)));
        }
    }
    let follow = if request.follow_redirects {
        "True"
    } else {
        "False"
    };
    arguments.push(match library {
        Library::Requests => format!("allow_redirects={follow}"),
        Library::Httpx => format!("follow_redirects={follow}"),
    });
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|argument| format!("    {argument},"))
        .collect();
    format!(
        "import {name}\n\nresponse = {name}.request(\n{}\n)\nprint(response.status_code, response.text)",
        arguments.join("\n")
    )
}

#[must_use]
pub fn requests(request: &CodeRequest) -> String {
    write(request, Library::Requests)
}

#[must_use]
pub fn httpx(request: &CodeRequest) -> String {
    write(request, Library::Httpx)
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("requests", case, &requests(&request));
            golden::check("httpx", case, &httpx(&request));
        }
    }
}
