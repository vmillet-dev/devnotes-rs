//! JavaScript's `fetch`, as Node 20 and a browser run it: a file is read with Node's
//! `openAsBlob`, a redirect not followed is `redirect: 'manual'`.

use super::CodeRequest;
use super::javascript::{body, form_data, headers, object, reads_files, string};

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut lines = Vec::new();
    if reads_files(&request.body) {
        lines.push("import { openAsBlob } from 'node:fs';".to_string());
        lines.push(String::new());
    }
    lines.extend(form_data(&request.body));
    let mut options = vec![("method".to_string(), string(request.method.as_str()))];
    let headers = headers(request);
    if !headers.is_empty() {
        options.push(("headers".to_string(), object(&headers, "  ")));
    }
    if let Some(body) = body(&request.body, "  ") {
        options.push(("body".to_string(), body));
    }
    if !request.follow_redirects {
        options.push(("redirect".to_string(), string("manual")));
    }
    lines.push(format!(
        "const response = await fetch({}, {});",
        string(&request.url),
        object(&options, "")
    ));
    lines.push("console.log(response.status, await response.text());".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("fetch", case, &write(&request));
        }
    }
}
