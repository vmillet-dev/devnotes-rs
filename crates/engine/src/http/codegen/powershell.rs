//! PowerShell 7: `Invoke-RestMethod`, its headers a hashtable, the body as `-Body`, a form as a
//! hashtable, multipart as `-Form`, a file as `-InFile`; a Basic auth encoded where it runs.

use super::{CodeBody, CodeRequest};
use crate::http::model::HttpMethod;

/// `'…'` for PowerShell: a quote is doubled.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn method(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "Get",
        HttpMethod::Post => "Post",
        HttpMethod::Put => "Put",
        HttpMethod::Patch => "Patch",
        HttpMethod::Delete => "Delete",
        HttpMethod::Head => "Head",
        HttpMethod::Options => "Options",
    }
}

/// `@{ 'A' = 'b'; 'C' = 'd' }` on one line, or one entry a line.
fn hashtable(entries: &[(String, String)], multiline: bool) -> String {
    if entries.is_empty() {
        return "@{}".to_string();
    }
    if multiline {
        let lines: Vec<String> = entries
            .iter()
            .map(|(name, value)| format!("  {} = {value}", quoted(name)))
            .collect();
        format!("@{{\n{}\n}}", lines.join("\n"))
    } else {
        let pairs: Vec<String> = entries
            .iter()
            .map(|(name, value)| format!("{} = {value}", quoted(name)))
            .collect();
        format!("@{{ {} }}", pairs.join("; "))
    }
}

#[must_use]
pub fn write(request: &CodeRequest, multiline: bool) -> String {
    let content_type = request.content_type();
    // The type goes as `-ContentType`: in `-Headers`, PowerShell 5 refuses it.
    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("content-type"))
        .map(|(name, value)| (name.clone(), quoted(value)))
        .collect();
    if let Some((user, password)) = &request.basic {
        headers.push((
            "Authorization".to_string(),
            format!(
                "'Basic ' + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes({}))",
                quoted(&format!("{user}:{password}"))
            ),
        ));
    }

    let mut lines = Vec::new();
    let mut parameters = vec![
        format!("-Uri {}", quoted(&request.url)),
        format!("-Method {}", method(request.method)),
    ];
    if !headers.is_empty() {
        lines.push(format!("$headers = {}", hashtable(&headers, multiline)));
        parameters.push("-Headers $headers".to_string());
    }
    if let Some(content_type) = content_type
        && !matches!(request.body, CodeBody::Multipart(_))
    {
        parameters.push(format!("-ContentType {}", quoted(content_type)));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => {
            lines.push(format!("$body = {}", quoted(text)));
            parameters.push("-Body $body".to_string());
        }
        CodeBody::Form(fields) => {
            let entries: Vec<(String, String)> = fields
                .iter()
                .map(|(name, value)| (name.clone(), quoted(value)))
                .collect();
            lines.push(format!("$body = {}", hashtable(&entries, multiline)));
            parameters.push("-Body $body".to_string());
        }
        CodeBody::Multipart(parts) => {
            let entries: Vec<(String, String)> = parts
                .iter()
                .map(|part| {
                    let value = if part.file {
                        format!("(Get-Item -Path {})", quoted(&part.value))
                    } else {
                        quoted(&part.value)
                    };
                    (part.name.clone(), value)
                })
                .collect();
            lines.push(format!("$form = {}", hashtable(&entries, multiline)));
            parameters.push("-Form $form".to_string());
        }
        CodeBody::File(path) => parameters.push(format!("-InFile {}", quoted(path))),
    }
    if !request.follow_redirects {
        parameters.push("-MaximumRedirection 0".to_string());
    }

    let call = if multiline {
        let mut call = vec!["Invoke-RestMethod".to_string()];
        call.extend(parameters.iter().map(|parameter| format!("  {parameter}")));
        let last = call.len() - 1;
        call.iter()
            .enumerate()
            .map(|(at, line)| {
                if at == last {
                    line.clone()
                } else {
                    format!("{line} `")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        format!("Invoke-RestMethod {}", parameters.join(" "))
    };
    lines.push(call);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("powershell", case, &write(&request, true));
        }
        let (_, get) = &golden::cases()[0];
        golden::check("powershell", "get-one-line", &write(get, false));
    }

    #[test]
    fn a_quote_is_doubled() {
        assert_eq!(quoted("O'Brien"), "'O''Brien'");
    }
}
