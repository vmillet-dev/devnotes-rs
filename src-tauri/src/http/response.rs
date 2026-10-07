//! What an answer is read as: its language, its cookies, its JSON laid out. Pure, so the send
//! module only gathers bytes.

use serde::Serialize;
use specta::Type;

use super::model::KeyValue;
use crate::notes::language::Language;

/// A `Set-Cookie`, its attributes named.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    /// As the server wrote it: `Max-Age` or `Expires`, whichever came.
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

fn essence(content_type: Option<&str>) -> String {
    content_type
        .and_then(|kind| kind.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// The language a text body is coloured and saved in, read off its type.
pub fn language(content_type: Option<&str>, body: &str) -> Language {
    let kind = essence(content_type);
    let has = |word: &str| kind.contains(word);
    if has("json") {
        Language::Json
    } else if has("html") {
        Language::Html
    } else if has("xml") {
        Language::Xml
    } else if has("javascript") || has("ecmascript") {
        Language::Js
    } else if has("css") {
        Language::Css
    } else if has("yaml") {
        Language::Yml
    } else if has("graphql") {
        Language::Graphql
    } else if kind.is_empty() && looks_like_json(body) {
        Language::Json
    } else {
        Language::Txt
    }
}

fn looks_like_json(body: &str) -> bool {
    let trimmed = body.trim_start();
    (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde::de::IgnoredAny>(body).is_ok()
}

/// Every `Set-Cookie` of the answer, in its order.
pub fn cookies(headers: &[KeyValue]) -> Vec<Cookie> {
    headers
        .iter()
        .filter(|header| header.key.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|header| cookie(&header.value))
        .collect()
}

fn cookie(line: &str) -> Option<Cookie> {
    let mut parts = line.split(';');
    let (name, value) = parts.next()?.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let mut cookie = Cookie {
        name: name.to_string(),
        value: value.trim().trim_matches('"').to_string(),
        ..Cookie::default()
    };
    for attribute in parts {
        let (key, value) = attribute
            .split_once('=')
            .map_or((attribute.trim(), ""), |(key, value)| {
                (key.trim(), value.trim())
            });
        match key.to_ascii_lowercase().as_str() {
            "domain" => cookie.domain = Some(value.to_string()),
            "path" => cookie.path = Some(value.to_string()),
            // `Max-Age` wins over `Expires`, as a browser reads them.
            "max-age" => cookie.expires = Some(format!("Max-Age={value}")),
            "expires" if cookie.expires.is_none() => cookie.expires = Some(value.to_string()),
            "httponly" => cookie.http_only = true,
            "secure" => cookie.secure = true,
            "samesite" => cookie.same_site = Some(value.to_string()),
            _ => {}
        }
    }
    Some(cookie)
}

/// Laid out two spaces a level, its tokens copied as they came: re-serialising would round a
/// number past `f64` (`12345678901234567890`) and write `1.10` as `1.1`. `None` when it does
/// not read as JSON.
pub fn pretty_json(text: &str) -> Option<String> {
    serde_json::from_str::<serde::de::IgnoredAny>(text).ok()?;
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    let mut depth = 0usize;
    let mut chars = text.chars().peekable();
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        out.extend(std::iter::repeat_n("  ", depth));
    };
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('"');
                let mut escaped = false;
                for inner in chars.by_ref() {
                    out.push(inner);
                    if escaped {
                        escaped = false;
                    } else if inner == '\\' {
                        escaped = true;
                    } else if inner == '"' {
                        break;
                    }
                }
            }
            '{' | '[' => {
                let close = if c == '{' { '}' } else { ']' };
                while chars.peek().is_some_and(|next| next.is_whitespace()) {
                    chars.next();
                }
                if chars.peek() == Some(&close) {
                    chars.next();
                    out.push(c);
                    out.push(close);
                } else {
                    out.push(c);
                    depth += 1;
                    newline(&mut out, depth);
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                newline(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(',');
                newline(&mut out, depth);
            }
            ':' => out.push_str(": "),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        }
    }

    #[test]
    fn json_is_laid_out_with_its_numbers_and_strings_as_they_came() {
        assert_eq!(
            pretty_json(r#"{"id":12345678901234567890,"n":1.10,"s":"a, {b}: \"c\"","e":[],"o":{ },"l":[1,{"k":null}]}"#)
                .unwrap(),
            "{\n  \"id\": 12345678901234567890,\n  \"n\": 1.10,\n  \"s\": \"a, {b}: \\\"c\\\"\",\n  \"e\": [],\n  \"o\": {},\n  \"l\": [\n    1,\n    {\n      \"k\": null\n    }\n  ]\n}"
        );
        assert_eq!(pretty_json("  42 ").as_deref(), Some("42"));
        assert_eq!(pretty_json("{\"a\":"), None);
    }

    #[test]
    fn the_language_follows_the_type_or_failing_one_the_text() {
        assert_eq!(
            language(Some("application/problem+json; charset=utf-8"), ""),
            Language::Json
        );
        assert_eq!(language(Some("text/html"), ""), Language::Html);
        assert_eq!(language(Some("application/xhtml+xml"), ""), Language::Html);
        assert_eq!(language(Some("application/rss+xml"), ""), Language::Xml);
        assert_eq!(language(Some("text/javascript"), ""), Language::Js);
        assert_eq!(language(Some("text/plain"), "{\"a\":1}"), Language::Txt);
        assert_eq!(language(None, " [1, 2]"), Language::Json);
        assert_eq!(language(None, "hello"), Language::Txt);
    }

    #[test]
    fn a_set_cookie_is_read_with_its_attributes_and_the_other_headers_are_not() {
        let found = cookies(&[
            header("content-type", "text/plain"),
            header(
                "set-cookie",
                "session=\"abc=1\"; Path=/; Domain=example.com; Expires=Wed, 21 Oct 2026 07:28:00 GMT; Max-Age=3600; HttpOnly; Secure; SameSite=Lax",
            ),
            header("Set-Cookie", "theme=dark"),
            header("set-cookie", "=nameless"),
        ]);

        assert_eq!(
            found,
            vec![
                Cookie {
                    name: "session".to_string(),
                    value: "abc=1".to_string(),
                    domain: Some("example.com".to_string()),
                    path: Some("/".to_string()),
                    expires: Some("Max-Age=3600".to_string()),
                    http_only: true,
                    secure: true,
                    same_site: Some("Lax".to_string()),
                },
                Cookie {
                    name: "theme".to_string(),
                    value: "dark".to_string(),
                    ..Cookie::default()
                },
            ]
        );
    }
}
