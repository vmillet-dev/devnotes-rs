//! axios, from Node: `axios.request` with its own `auth` for a Basic one, `maxRedirects: 0`
//! for a redirect not followed.

use super::CodeRequest;
use super::javascript::{body, form_data, object, reads_files, string};

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut lines = vec!["import axios from 'axios';".to_string()];
    if reads_files(&request.body) {
        lines.push("import { openAsBlob } from 'node:fs';".to_string());
    }
    lines.push(String::new());
    lines.extend(form_data(&request.body));
    let mut options = vec![
        ("method".to_string(), string(request.method.as_str())),
        ("url".to_string(), string(&request.url)),
    ];
    let headers: Vec<(String, String)> = request
        .headers
        .iter()
        .map(|(name, value)| (name.clone(), string(value)))
        .collect();
    if !headers.is_empty() {
        options.push(("headers".to_string(), object(&headers, "  ")));
    }
    if let Some((user, password)) = &request.basic {
        options.push((
            "auth".to_string(),
            object(
                &[
                    ("username".to_string(), string(user)),
                    ("password".to_string(), string(password)),
                ],
                "  ",
            ),
        ));
    }
    if let Some(body) = body(&request.body, "  ") {
        options.push(("data".to_string(), body));
    }
    if !request.follow_redirects {
        options.push(("maxRedirects".to_string(), "0".to_string()));
    }
    // An answer of 4xx or 5xx is an answer to print, not an exception to catch.
    options.push(("validateStatus".to_string(), "() => true".to_string()));
    lines.push(format!(
        "const response = await axios.request({});",
        object(&options, "")
    ));
    lines.push("console.log(response.status, response.data);".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("axios", case, &write(&request));
        }
    }
}
