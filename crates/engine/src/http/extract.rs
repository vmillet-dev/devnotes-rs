//! A value of a JSON answer kept in a variable after each send, without a script: the path as
//! the response's explorer writes it — `$.data.items[0]['a key']` — read from `body`.

use serde_json::Value;

/// One step down a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
}

/// `body.access_token`, `body.items[0].id`, `body['a key']`; `None` for what does not read as
/// one.
fn steps(path: &str) -> Option<Vec<Step>> {
    let mut rest = path.trim().strip_prefix("body")?;
    let mut steps = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            if end == 0 {
                return None;
            }
            steps.push(Step::Key(after[..end].to_string()));
            rest = &after[end..];
        } else if let Some(after) = rest.strip_prefix("['") {
            let (key, consumed) = quoted(after)?;
            steps.push(Step::Key(key));
            rest = after[consumed..].strip_prefix("']")?;
        } else {
            let after = rest.strip_prefix('[')?;
            let end = after.find(']')?;
            steps.push(Step::Index(after[..end].parse().ok()?));
            rest = &after[end + 1..];
        }
    }
    Some(steps)
}

/// A key between quotes, `\'` and `\\` read back; and how many bytes it took.
fn quoted(text: &str) -> Option<(String, usize)> {
    let mut key = String::new();
    let mut chars = text.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => key.push(chars.next()?.1),
            '\'' => return Some((key, at)),
            _ => key.push(c),
        }
    }
    None
}

/// The value at `path` in a JSON `body`: a text as it is, anything else as JSON. `None` when the
/// body is not JSON or the path leads nowhere — the variable is then left as it was.
#[must_use]
pub fn extract(body: &str, path: &str) -> Option<String> {
    let steps = steps(path)?;
    let root: Value = serde_json::from_str(body).ok()?;
    let mut at = &root;
    for step in &steps {
        at = match step {
            Step::Key(key) => at.get(key)?,
            Step::Index(index) => at.get(*index)?,
        };
    }
    Some(match at {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    })
}

/// The explorer's `$.data.token` as a rule reads it: `body.data.token`.
#[must_use]
pub fn body_path(explorer_path: &str) -> String {
    match explorer_path.strip_prefix('$') {
        Some(rest) => format!("body{rest}"),
        None => explorer_path.to_owned(),
    }
}

/// The key a path ends on, or `None` when it ends on an index or a bare `body`.
fn last_key(path: &str) -> Option<String> {
    match steps(path)?.pop()? {
        Step::Key(key) => Some(key),
        Step::Index(_) => None,
    }
}

/// The name proposed for a value: its key in camelCase — `access_token` is `accessToken`.
#[must_use]
pub fn variable_name_of(path: &str) -> String {
    let key = last_key(path).unwrap_or_else(|| "value".to_owned());
    let mut name = String::new();
    let mut upper = false;
    for c in key.chars() {
        if !c.is_ascii_alphanumeric() {
            upper = !name.is_empty();
        } else if upper {
            name.push(c.to_ascii_uppercase());
            upper = false;
        } else if name.is_empty() {
            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }
    if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name
    } else {
        format!("value{name}")
    }
}

/// What a key reads as when it holds something to keep out of sight.
const SECRET_WORDS: [&str; 10] = [
    "token",
    "secret",
    "passw",
    "pwd",
    "private",
    "credential",
    "session",
    "signature",
    "cookie",
    "authorization",
];

/// A key that reads as a token, a password or a key is proposed as a secret.
#[must_use]
pub fn looks_secret(path: &str) -> bool {
    let Some(key) = last_key(path) else {
        return false;
    };
    let lower = key.to_lowercase();
    if SECRET_WORDS.iter().any(|word| lower.contains(word)) {
        return true;
    }
    let ends_a_word = lower == "key" || lower.ends_with("_key") || lower.ends_with("-key");
    let camel = key
        .strip_suffix("Key")
        .is_some_and(|before| before.ends_with(|c: char| c.is_ascii_lowercase()));
    ends_a_word || camel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_proposed_from_the_last_key_in_camel_case() {
        assert_eq!(variable_name_of("body.access_token"), "accessToken");
        assert_eq!(variable_name_of("body.data['user-id']"), "userId");
        assert_eq!(variable_name_of("body.items[0]"), "value");
        assert_eq!(variable_name_of("body.Id"), "id");
        assert_eq!(variable_name_of("body['2fa']"), "value2fa");
    }

    #[test]
    fn a_key_that_reads_as_a_secret_is_proposed_as_one() {
        for path in [
            "body.access_token",
            "body.password",
            "body.apiKey",
            "body.api_key",
            "body['x-key']",
            "body.key",
        ] {
            assert!(looks_secret(path), "{path}");
        }
        for path in ["body.keyboard", "body.monkey", "body.id", "body.items[0]"] {
            assert!(!looks_secret(path), "{path}");
        }
    }

    #[test]
    fn the_explorer_s_path_becomes_a_rule_s() {
        assert_eq!(body_path("$.data.token"), "body.data.token");
        assert_eq!(body_path("$['a key']"), "body['a key']");
        assert_eq!(body_path("body.id"), "body.id");
    }

    const LOGIN: &str = r#"{"access_token":"eyJ.a.b","expires_in":3600,"scope":null,
        "user":{"roles":["admin","billing"],"a key":{"it's":true}}}"#;

    #[test]
    fn reads_a_value_down_its_path_a_text_as_it_is_the_rest_as_json() {
        assert_eq!(
            extract(LOGIN, "body.access_token").as_deref(),
            Some("eyJ.a.b")
        );
        assert_eq!(extract(LOGIN, "body.expires_in").as_deref(), Some("3600"));
        assert_eq!(extract(LOGIN, "body.scope").as_deref(), Some("null"));
        assert_eq!(
            extract(LOGIN, "body.user.roles[1]").as_deref(),
            Some("billing")
        );
        assert_eq!(
            extract(LOGIN, "body.user.roles").as_deref(),
            Some(r#"["admin","billing"]"#)
        );
        assert_eq!(
            extract(LOGIN, r"body.user['a key']['it\'s']").as_deref(),
            Some("true")
        );
        assert_eq!(extract("[1,2]", "body[0]").as_deref(), Some("1"));
    }

    #[test]
    fn a_path_that_no_longer_reads_gives_nothing() {
        assert_eq!(extract(LOGIN, "body.refresh_token"), None);
        assert_eq!(extract(LOGIN, "body.user.roles[7]"), None);
        assert_eq!(extract(LOGIN, "body..x"), None);
        assert_eq!(extract(LOGIN, "headers.x"), None);
        assert_eq!(extract(LOGIN, "body['open"), None);
        assert_eq!(extract("<html>", "body.x"), None);
    }
}
