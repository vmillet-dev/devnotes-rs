//! Go's net/http, laid out as gofmt would: a `main` that builds the request, sends it with a
//! client that stops at the first redirect unless asked, and prints the answer.

use std::collections::BTreeSet;

use super::source::{double_quoted as quoted, file_name};
use super::{CodeBody, CodeRequest, form_encoded};

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut imports: BTreeSet<&str> = ["fmt", "io", "net/http"].into_iter().collect();
    let mut body_lines: Vec<String> = Vec::new();
    let mut after_request: Vec<String> = Vec::new();
    let body = match &request.body {
        CodeBody::None => "nil".to_string(),
        CodeBody::Text(text) => {
            imports.insert("strings");
            format!("strings.NewReader({})", quoted(text))
        }
        CodeBody::Form(fields) => {
            imports.insert("strings");
            format!("strings.NewReader({})", quoted(&form_encoded(fields)))
        }
        CodeBody::Multipart(parts) => {
            imports.extend(["bytes", "mime/multipart"]);
            body_lines.push("var form bytes.Buffer".to_string());
            body_lines.push("writer := multipart.NewWriter(&form)".to_string());
            for (at, part) in parts.iter().enumerate() {
                if part.file {
                    imports.insert("os");
                    body_lines.push(format!("file{at}, err := os.Open({})", quoted(&part.value)));
                    body_lines.push("if err != nil {\n\t\tpanic(err)\n\t}".to_string());
                    body_lines.push(format!(
                        "part{at}, _ := writer.CreateFormFile({}, {})",
                        quoted(&part.name),
                        quoted(&file_name(&part.value))
                    ));
                    body_lines.push(format!("io.Copy(part{at}, file{at})"));
                    body_lines.push(format!("file{at}.Close()"));
                } else {
                    body_lines.push(format!(
                        "writer.WriteField({}, {})",
                        quoted(&part.name),
                        quoted(&part.value)
                    ));
                }
            }
            body_lines.push("writer.Close()".to_string());
            after_request.push(
                "request.Header.Set(\"Content-Type\", writer.FormDataContentType())".to_string(),
            );
            "&form".to_string()
        }
        CodeBody::File(path) => {
            imports.insert("os");
            body_lines.push(format!("file, err := os.Open({})", quoted(path)));
            body_lines.push("if err != nil {\n\t\tpanic(err)\n\t}".to_string());
            body_lines.push("defer file.Close()".to_string());
            "file".to_string()
        }
    };

    let mut lines = body_lines;
    lines.push(format!(
        "request, err := http.NewRequest({}, {}, {body})",
        quoted(request.method.as_str()),
        quoted(&request.url)
    ));
    lines.push("if err != nil {\n\t\tpanic(err)\n\t}".to_string());
    for (name, value) in &request.headers {
        lines.push(format!(
            "request.Header.Set({}, {})",
            quoted(name),
            quoted(value)
        ));
    }
    lines.extend(after_request);
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "request.SetBasicAuth({}, {})",
            quoted(user),
            quoted(password)
        ));
    }
    if request.follow_redirects {
        lines.push("client := &http.Client{}".to_string());
    } else {
        lines.push(
            "client := &http.Client{\n\t\tCheckRedirect: func(*http.Request, []*http.Request) error {\n\t\t\treturn http.ErrUseLastResponse\n\t\t},\n\t}"
                .to_string(),
        );
    }
    lines.push("response, err := client.Do(request)".to_string());
    lines.push("if err != nil {\n\t\tpanic(err)\n\t}".to_string());
    lines.push("defer response.Body.Close()".to_string());
    lines.push("answer, _ := io.ReadAll(response.Body)".to_string());
    lines.push("fmt.Println(response.StatusCode, string(answer))".to_string());

    let imports: Vec<String> = imports
        .into_iter()
        .map(|import| format!("\t{}", quoted(import)))
        .collect();
    let body: Vec<String> = lines.into_iter().map(|line| format!("\t{line}")).collect();
    format!(
        "package main\n\nimport (\n{}\n)\n\nfunc main() {{\n{}\n}}",
        imports.join("\n"),
        body.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("go", case, &write(&request));
        }
    }
}
