//! Dart's `package:http`: a `Request` — a `MultipartRequest` for a form with files —, its
//! headers and body, sent and read back.

use super::source::dart as quoted;
use super::{CodeBody, CodeRequest};

fn map(entries: &[(String, String)]) -> String {
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, value)| format!("    {}: {},", quoted(name), quoted(value)))
        .collect();
    format!("{{\n{}\n  }}", lines.join("\n"))
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut imports = vec!["import 'package:http/http.dart' as http;".to_string()];
    if request.basic.is_some() {
        imports.insert(0, "import 'dart:convert';".to_string());
    }
    if matches!(request.body, CodeBody::File(_)) {
        imports.insert(0, "import 'dart:io';".to_string());
    }
    let multipart = matches!(request.body, CodeBody::Multipart(_));
    let mut lines = imports;
    lines.push(String::new());
    lines.push("Future<void> main() async {".to_string());
    lines.push(format!(
        "  final request = http.{}({}, Uri.parse({}));",
        if multipart {
            "MultipartRequest"
        } else {
            "Request"
        },
        quoted(request.method.as_str()),
        quoted(&request.url)
    ));
    if !request.headers.is_empty() {
        lines.push(format!(
            "  request.headers.addAll({});",
            map(&request.headers)
        ));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "  request.headers['Authorization'] = 'Basic ${{base64Encode(utf8.encode({}))}}';",
            quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => lines.push(format!("  request.body = {};", quoted(text))),
        CodeBody::Form(fields) => lines.push(format!("  request.bodyFields = {};", map(fields))),
        CodeBody::Multipart(parts) => {
            for part in parts {
                if part.file {
                    lines.push(format!(
                        "  request.files.add(await http.MultipartFile.fromPath({}, {}));",
                        quoted(&part.name),
                        quoted(&part.value)
                    ));
                } else {
                    lines.push(format!(
                        "  request.fields[{}] = {};",
                        quoted(&part.name),
                        quoted(&part.value)
                    ));
                }
            }
        }
        CodeBody::File(path) => lines.push(format!(
            "  request.bodyBytes = await File({}).readAsBytes();",
            quoted(path)
        )),
    }
    lines.push(format!(
        "  request.followRedirects = {};",
        request.follow_redirects
    ));
    lines.push(
        "  final response = await http.Response.fromStream(await request.send());".to_string(),
    );
    lines.push("  print('${response.statusCode} ${response.body}');".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("dart", case, &write(&request));
        }
    }
}
