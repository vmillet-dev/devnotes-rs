//! Node's undici: `request`, which follows no redirect unless `maxRedirections` says how many.

use super::CodeRequest;
use super::javascript::{body, form_data, headers, object, reads_files, string};

/// What « Suivre les redirections » allows, as a client would by default.
const FOLLOWED: u32 = 10;

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut lines = vec!["import { request } from 'undici';".to_string()];
    if reads_files(&request.body) {
        lines.push("import { openAsBlob } from 'node:fs';".to_string());
    }
    lines.push(String::new());
    lines.extend(form_data(&request.body));
    let mut options = vec![("method".to_string(), string(request.method.as_str()))];
    let headers = headers(request);
    if !headers.is_empty() {
        options.push(("headers".to_string(), object(&headers, "  ")));
    }
    if let Some(body) = body(&request.body, "  ") {
        options.push(("body".to_string(), body));
    }
    if request.follow_redirects {
        options.push(("maxRedirections".to_string(), FOLLOWED.to_string()));
    }
    lines.push(format!(
        "const {{ statusCode, body }} = await request({}, {});",
        string(&request.url),
        object(&options, "")
    ));
    lines.push("console.log(statusCode, await body.text());".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("undici", case, &write(&request));
        }
    }
}
