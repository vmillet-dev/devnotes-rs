//! A request's body by its kind. The body left when another kind is chosen is set aside, by tab,
//! and comes back as it was when its kind is chosen again: « Switch to None » on a `GET` would
//! otherwise cost the text it warns about. Never written: a request saves the body it shows.

use std::collections::HashMap;

use engine::http::model::{FormPart, RequestBody};

/// The kinds a body is chosen among, in the order they are offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyKind {
    None,
    Json,
    Text,
    Form,
    Multipart,
    Binary,
}

impl BodyKind {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Json,
        Self::Text,
        Self::Form,
        Self::Multipart,
        Self::Binary,
    ];

    pub fn of(body: &RequestBody) -> Self {
        match body {
            RequestBody::None => Self::None,
            RequestBody::Json { .. } => Self::Json,
            RequestBody::Text { .. } => Self::Text,
            RequestBody::Form { .. } => Self::Form,
            RequestBody::Multipart { .. } => Self::Multipart,
            RequestBody::Binary { .. } => Self::Binary,
        }
    }
}

/// Where a file chosen goes: the body itself, or the multipart row at that rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFor {
    Body,
    Part(usize),
}

/// The body with the file chosen in it; `None` when its kind no longer takes one.
pub fn with_file(body: &RequestBody, target: FileFor, path: String) -> Option<RequestBody> {
    match (target, body) {
        (FileFor::Body, RequestBody::Binary { .. }) => Some(RequestBody::Binary { path }),
        (FileFor::Part(at), RequestBody::Multipart { parts }) => {
            let mut parts = parts.clone();
            if at == parts.len() {
                parts.push(FormPart::default());
            }
            let part = parts.get_mut(at)?;
            part.value = path;
            part.file = true;
            Some(RequestBody::Multipart { parts })
        }
        _ => None,
    }
}

/// What each kind of body held when another was chosen.
pub type BodiesLeft = HashMap<BodyKind, RequestBody>;

/// Sets a body aside under its kind. JSON or plain, a text is one text: it is kept under both,
/// so it travels between the two as it always did.
pub fn set_aside(left: &mut BodiesLeft, body: RequestBody) {
    match body {
        RequestBody::Json { text } | RequestBody::Text { text } => {
            left.insert(BodyKind::Json, RequestBody::Json { text: text.clone() });
            left.insert(BodyKind::Text, RequestBody::Text { text });
        }
        other => {
            left.insert(BodyKind::of(&other), other);
        }
    }
}

/// The body a kind starts with: what it held when it was left, or nothing yet.
pub fn body_of_kind(kind: BodyKind, left: &BodiesLeft) -> RequestBody {
    if let Some(kept) = left.get(&kind) {
        return kept.clone();
    }
    match kind {
        BodyKind::None => RequestBody::None,
        BodyKind::Json => RequestBody::Json {
            text: String::new(),
        },
        BodyKind::Text => RequestBody::Text {
            text: String::new(),
        },
        BodyKind::Form => RequestBody::Form { fields: Vec::new() },
        BodyKind::Multipart => RequestBody::Multipart { parts: Vec::new() },
        BodyKind::Binary => RequestBody::Binary {
            path: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_left_comes_back_as_it_was() {
        let mut left = BodiesLeft::new();

        set_aside(
            &mut left,
            RequestBody::Json {
                text: "{\"a\":1}".to_owned(),
            },
        );

        assert_eq!(
            body_of_kind(BodyKind::Text, &left),
            RequestBody::Text {
                text: "{\"a\":1}".to_owned()
            }
        );
        assert_eq!(
            body_of_kind(BodyKind::Form, &left),
            RequestBody::Form { fields: Vec::new() }
        );
        assert_eq!(body_of_kind(BodyKind::None, &left), RequestBody::None);
    }
}
