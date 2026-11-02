//! Kotlin with `OkHttp` 4: a client that follows redirects only when asked, the body a
//! `RequestBody` carrying its type, the request built and executed.

use std::collections::BTreeSet;

use super::source::{file_name, kotlin as quoted};
use super::{CodeBody, CodeRequest};
use crate::http::model::HttpMethod;

/// The body as `.method` takes it, what builds it written into `prelude`.
fn body_of(
    request: &CodeRequest,
    imports: &mut BTreeSet<&'static str>,
    prelude: &mut Vec<String>,
) -> String {
    let content_type = request.content_type().unwrap_or("application/octet-stream");
    match &request.body {
        CodeBody::None if matches!(request.method, HttpMethod::Get | HttpMethod::Head) => {
            "null".to_string()
        }
        CodeBody::None => {
            imports.insert("okhttp3.RequestBody.Companion.toRequestBody");
            "ByteArray(0).toRequestBody()".to_string()
        }
        CodeBody::Text(text) => {
            imports.insert("okhttp3.MediaType.Companion.toMediaType");
            imports.insert("okhttp3.RequestBody.Companion.toRequestBody");
            format!(
                "{}.toRequestBody({}.toMediaType())",
                quoted(text),
                quoted(content_type)
            )
        }
        CodeBody::Form(fields) => {
            imports.insert("okhttp3.FormBody");
            prelude.push("val body = FormBody.Builder()".to_string());
            for (name, value) in fields {
                prelude.push(format!("    .add({}, {})", quoted(name), quoted(value)));
            }
            prelude.push("    .build()".to_string());
            "body".to_string()
        }
        CodeBody::Multipart(parts) => {
            imports.insert("okhttp3.MultipartBody");
            prelude.push("val body = MultipartBody.Builder()".to_string());
            prelude.push("    .setType(MultipartBody.FORM)".to_string());
            for part in parts {
                if part.file {
                    imports.insert("java.io.File");
                    imports.insert("okhttp3.RequestBody.Companion.asRequestBody");
                    prelude.push(format!(
                        "    .addFormDataPart({}, {}, File({}).asRequestBody())",
                        quoted(&part.name),
                        quoted(&file_name(&part.value)),
                        quoted(&part.value)
                    ));
                } else {
                    prelude.push(format!(
                        "    .addFormDataPart({}, {})",
                        quoted(&part.name),
                        quoted(&part.value)
                    ));
                }
            }
            prelude.push("    .build()".to_string());
            "body".to_string()
        }
        CodeBody::File(path) => {
            imports.insert("java.io.File");
            imports.insert("okhttp3.MediaType.Companion.toMediaType");
            imports.insert("okhttp3.RequestBody.Companion.asRequestBody");
            format!(
                "File({}).asRequestBody({}.toMediaType())",
                quoted(path),
                quoted(content_type)
            )
        }
    }
}

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let mut imports: BTreeSet<&str> = ["okhttp3.OkHttpClient", "okhttp3.Request"]
        .into_iter()
        .collect();
    let mut prelude = Vec::new();
    let body = body_of(request, &mut imports, &mut prelude);
    if request.basic.is_some() {
        imports.insert("okhttp3.Credentials");
    }

    let mut lines: Vec<String> = imports
        .iter()
        .map(|import| format!("import {import}"))
        .collect();
    lines.push(String::new());
    lines.push("val client = OkHttpClient.Builder()".to_string());
    lines.push(format!(
        "    .followRedirects({})",
        request.follow_redirects
    ));
    lines.push("    .build()".to_string());
    lines.extend(prelude);
    lines.push("val request = Request.Builder()".to_string());
    lines.push(format!("    .url({})", quoted(&request.url)));
    // The body carries its type: a `Content-Type` row would be a second one.
    for (name, value) in request
        .headers
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("content-type") || body == "null")
    {
        lines.push(format!("    .header({}, {})", quoted(name), quoted(value)));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "    .header(\"Authorization\", Credentials.basic({}, {}))",
            quoted(user),
            quoted(password)
        ));
    }
    lines.push(format!(
        "    .method({}, {body})",
        quoted(request.method.as_str())
    ));
    lines.push("    .build()".to_string());
    lines.push("client.newCall(request).execute().use { response ->".to_string());
    lines.push("    println(\"${response.code} ${response.body?.string()}\")".to_string());
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
            golden::check("okhttp", case, &write(&request));
        }
    }
}
