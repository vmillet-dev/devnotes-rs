//! C# with `HttpClient` (.NET 6, top-level statements): a handler that follows redirects only
//! when asked, the request's headers, its content carrying the type.

use super::source::{double_quoted as quoted, file_name};
use super::{CodeBody, CodeRequest};
use crate::http::model::HttpMethod;

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

#[must_use]
pub fn write(request: &CodeRequest) -> String {
    let content_type = request.content_type();
    let mut lines = Vec::new();
    lines.push("using System.Net.Http.Headers;".to_string());
    lines.push("using System.Text;".to_string());
    lines.push(String::new());
    lines.push(format!(
        "using var handler = new HttpClientHandler {{ AllowAutoRedirect = {} }};",
        request.follow_redirects
    ));
    lines.push("using var client = new HttpClient(handler);".to_string());
    lines.push(format!(
        "using var request = new HttpRequestMessage(HttpMethod.{}, {});",
        method(request.method),
        quoted(&request.url)
    ));
    // A `Content-Type` belongs to the content in .NET, not to the request.
    for (name, value) in request
        .headers
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("content-type"))
    {
        lines.push(format!(
            "request.Headers.TryAddWithoutValidation({}, {});",
            quoted(name),
            quoted(value)
        ));
    }
    if let Some((user, password)) = &request.basic {
        lines.push(format!(
            "request.Headers.Authorization = new AuthenticationHeaderValue(\"Basic\", Convert.ToBase64String(Encoding.UTF8.GetBytes({})));",
            quoted(&format!("{user}:{password}"))
        ));
    }
    match &request.body {
        CodeBody::None => {}
        CodeBody::Text(text) => lines.push(format!(
            "request.Content = new StringContent({}, Encoding.UTF8, {});",
            quoted(text),
            quoted(content_type.unwrap_or("text/plain"))
        )),
        CodeBody::Form(fields) => {
            lines.push(
                "request.Content = new FormUrlEncodedContent(new Dictionary<string, string>"
                    .to_string(),
            );
            lines.push("{".to_string());
            for (name, value) in fields {
                lines.push(format!("    [{}] = {},", quoted(name), quoted(value)));
            }
            lines.push("});".to_string());
        }
        CodeBody::Multipart(parts) => {
            lines.push("var form = new MultipartFormDataContent();".to_string());
            for part in parts {
                if part.file {
                    lines.push(format!(
                        "form.Add(new StreamContent(File.OpenRead({})), {}, {});",
                        quoted(&part.value),
                        quoted(&part.name),
                        quoted(&file_name(&part.value))
                    ));
                } else {
                    lines.push(format!(
                        "form.Add(new StringContent({}), {});",
                        quoted(&part.value),
                        quoted(&part.name)
                    ));
                }
            }
            lines.push("request.Content = form;".to_string());
        }
        CodeBody::File(path) => {
            lines.push(format!(
                "request.Content = new StreamContent(File.OpenRead({}));",
                quoted(path)
            ));
            if let Some(content_type) = content_type {
                lines.push(format!(
                    "request.Content.Headers.ContentType = new MediaTypeHeaderValue({});",
                    quoted(content_type)
                ));
            }
        }
    }
    lines.push("using var response = await client.SendAsync(request);".to_string());
    lines.push(
        "Console.WriteLine($\"{(int)response.StatusCode} {await response.Content.ReadAsStringAsync()}\");"
            .to_string(),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::golden;
    use super::*;

    #[test]
    fn each_case_is_written_as_its_golden_file_says() {
        for (case, request) in golden::cases() {
            golden::check("csharp", case, &write(&request));
        }
    }
}
