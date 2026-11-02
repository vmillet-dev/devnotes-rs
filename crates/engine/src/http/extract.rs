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

#[cfg(test)]
mod tests {
    use super::*;

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
