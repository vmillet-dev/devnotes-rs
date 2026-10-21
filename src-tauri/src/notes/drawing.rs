//! A Dessin: its body is the scene the front draws, its two pictures sit in a table of their
//! own (`drawing_pictures`). Rust reads the scene only for its words; a list opens every body
//! it searches, so the pictures, which are most of a drawing's bytes, are kept out of it.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::ValidationError;

/// The two pictures the front takes of a drawing when it saves it, as PNG data URIs: a small
/// one for its card, a large one for a Note that embeds it. Both empty: the drawing was emptied,
/// and has no picture any more.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DrawingPictures {
    pub thumbnail: String,
    pub picture: String,
}

/// A drawing a Note can embed: what its menu names it by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DrawingRef {
    pub id: String,
    pub title: String,
}

/// By title, then by id for two of the same name, folded as a search folds them.
pub(crate) fn sort_drawings(drawings: &mut [DrawingRef]) {
    drawings.sort_by(|a, b| {
        crate::notes::view::fold(&a.title)
            .cmp(&crate::notes::view::fold(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// The same, decoded: what is sealed and stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pictures {
    pub thumbnail: Vec<u8>,
    pub picture: Vec<u8>,
}

/// No bigger once decoded: the large picture's side is capped by the front, this is the guard.
pub(crate) const PICTURE_MAX_BYTES: usize = 4 * 1024 * 1024;
const PNG: &str = "data:image/png;base64,";
const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

impl DrawingPictures {
    /// Each a PNG, read before anything is locked; `None` for the pictures taken away.
    pub fn decoded(&self) -> Result<Option<Pictures>, ValidationError> {
        if self.thumbnail.is_empty() && self.picture.is_empty() {
            return Ok(None);
        }
        Ok(Some(Pictures {
            thumbnail: png("pictures.thumbnail", &self.thumbnail)?,
            picture: png("pictures.picture", &self.picture)?,
        }))
    }
}

fn png(field: &'static str, uri: &str) -> Result<Vec<u8>, ValidationError> {
    let payload = uri
        .strip_prefix(PNG)
        .ok_or_else(|| ValidationError::new(field, "not a PNG data URI"))?;
    // Measured before decoding: base64 is four characters for three bytes.
    if payload.len() / 4 * 3 > PICTURE_MAX_BYTES {
        return Err(ValidationError::new(field, "too large"));
    }
    let bytes = BASE64
        .decode(payload)
        .map_err(|_| ValidationError::new(field, "not base64"))?;
    if !bytes.starts_with(SIGNATURE) {
        return Err(ValidationError::new(field, "not a PNG"));
    }
    Ok(bytes)
}

/// What an `<img>` is given back.
pub(crate) fn uri_of(png: &[u8]) -> String {
    format!("{PNG}{}", BASE64.encode(png))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Body {
    scene: Group,
}

/// Fabric's canvas and its groups hold their objects alike.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Group {
    objects: Vec<Shape>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Shape {
    text: Option<String>,
    objects: Vec<Shape>,
}

/// The words written on the drawing, one text a line: what a search reads and quotes. A body
/// that does not parse is a drawing with nothing on it.
pub(crate) fn words(content: &str) -> String {
    let body: Body = serde_json::from_str(content).unwrap_or_default();
    let mut lines = Vec::new();
    gather(&body.scene.objects, &mut lines);
    lines.join("\n")
}

fn gather(shapes: &[Shape], lines: &mut Vec<String>) {
    for shape in shapes {
        if let Some(text) = shape.text.as_deref().map(str::trim)
            && !text.is_empty()
        {
            lines.push(text.replace('\n', " "));
        }
        gather(&shape.objects, lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eight bytes every PNG opens with, and a few after them.
    fn png_uri() -> String {
        uri_of(b"\x89PNG\r\n\x1a\nIHDR")
    }

    #[test]
    fn the_words_are_read_through_the_groups_a_line_each() {
        let body = r#"{
            "version": 1,
            "scene": {
                "version": "7.4.0",
                "objects": [
                    { "type": "Rect", "left": 10 },
                    { "type": "Textbox", "text": "API gateway" },
                    { "type": "Group", "objects": [{ "type": "Textbox", "text": "  Postgres\nprimary " }] },
                    { "type": "Textbox", "text": "   " }
                ]
            }
        }"#;

        assert_eq!(words(body), "API gateway\nPostgres primary");
    }

    #[test]
    fn a_body_that_does_not_parse_is_an_empty_drawing() {
        for body in ["", "not json", "[1, 2]", r#"{ "scene": 3 }"#] {
            assert_eq!(words(body), "");
        }
    }

    #[test]
    fn two_pngs_are_taken_as_bytes_and_given_back_as_they_came() {
        let sent = DrawingPictures {
            thumbnail: png_uri(),
            picture: png_uri(),
        };

        let decoded = sent.decoded().unwrap().unwrap();

        assert!(decoded.thumbnail.starts_with(SIGNATURE));
        assert_eq!(uri_of(&decoded.picture), png_uri());
    }

    #[test]
    fn drawings_are_named_in_the_order_of_their_titles() {
        let drawing = |id: &str, title: &str| DrawingRef {
            id: id.to_string(),
            title: title.to_string(),
        };
        let mut drawings = vec![
            drawing("3", "Réseau"),
            drawing("2", "archi"),
            drawing("1", "Archi"),
        ];

        sort_drawings(&mut drawings);

        assert_eq!(
            drawings
                .iter()
                .map(|each| each.id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn two_empty_pictures_take_them_away() {
        let emptied = DrawingPictures {
            thumbnail: String::new(),
            picture: String::new(),
        };

        assert_eq!(emptied.decoded().unwrap(), None);
    }

    /// Shown by an `<img>` alone: anything but a PNG is no picture, nor one too big.
    #[test]
    fn what_is_not_a_png_is_refused_and_named() {
        let refused = |thumbnail: &str| {
            DrawingPictures {
                thumbnail: thumbnail.to_string(),
                picture: png_uri(),
            }
            .decoded()
            .unwrap_err()
        };

        assert_eq!(
            refused("data:image/svg+xml;base64,PHN2Zz4=").field,
            "pictures.thumbnail"
        );
        assert_eq!(refused("javascript:alert(1)").detail, "not a PNG data URI");
        assert_eq!(refused("data:image/png;base64,***").detail, "not base64");
        assert_eq!(refused(&uri_of(b"GIF89a")).detail, "not a PNG");
        let huge = format!("{PNG}{}", "A".repeat(PICTURE_MAX_BYTES / 3 * 4 + 8));
        assert_eq!(refused(&huge).detail, "too large");
    }
}
