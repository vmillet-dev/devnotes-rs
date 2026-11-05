//! The reference's two typefaces, embedded (ADR S2): Inter and `JetBrains Mono` as the variable
//! files the reference loaded, one per script, so the CSS weights (650 included) apply as they
//! are; Noto Emoji before the system's fonts, so an emoji looks the same on every machine.
//! What none of them draws (CJK, say) comes from the system, found on demand by eframe.

use std::sync::{Arc, LazyLock};

use egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};

const FACES: [(&str, &[u8]); 5] = [
    (
        "inter-latin",
        include_bytes!("../../assets/fonts/inter-latin-wght-normal.ttf"),
    ),
    (
        "inter-latin-ext",
        include_bytes!("../../assets/fonts/inter-latin-ext-wght-normal.ttf"),
    ),
    (
        "jetbrains-mono-latin",
        include_bytes!("../../assets/fonts/jetbrains-mono-latin-wght-normal.ttf"),
    ),
    (
        "jetbrains-mono-latin-ext",
        include_bytes!("../../assets/fonts/jetbrains-mono-latin-ext-wght-normal.ttf"),
    ),
    (
        "noto-emoji",
        include_bytes!("../../assets/fonts/NotoEmoji-wght.ttf"),
    ),
];

const SANS: [&str; 3] = ["inter-latin", "inter-latin-ext", "noto-emoji"];
const MONO: [&str; 3] = [
    "jetbrains-mono-latin",
    "jetbrains-mono-latin-ext",
    "noto-emoji",
];

/// The body's size: every other size of the reference is written against it.
pub const BASE: f32 = 14.0;

/// What a Dessin's text is measured and pictured with: the interface's sans, the same files.
pub fn for_drawings() -> Arc<drawing::Fonts> {
    static FONTS: LazyLock<Arc<drawing::Fonts>> = LazyLock::new(|| {
        let files: Vec<&'static [u8]> = SANS
            .iter()
            .filter_map(|name| FACES.iter().find(|(face, _)| face == name))
            .map(|(_, bytes)| *bytes)
            .collect();
        Arc::new(drawing::Fonts::new(&files))
    });
    Arc::clone(&FONTS)
}

/// What a diagram's labels are measured and painted with: the interface's sans; `None` should
/// the files not read as fonts.
pub fn for_diagrams() -> Option<&'static diagram::Fonts> {
    static FONTS: LazyLock<Option<diagram::Fonts>> = LazyLock::new(|| {
        let files: Vec<&'static [u8]> = SANS
            .iter()
            .filter_map(|name| FACES.iter().find(|(face, _)| face == name))
            .map(|(_, bytes)| *bytes)
            .collect();
        diagram::Fonts::new(&files)
    });
    FONTS.as_ref()
}

pub fn definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::empty();
    for (name, bytes) in FACES {
        definitions
            .font_data
            .insert(name.to_string(), FontData::from_static(bytes).into());
    }
    let family = |names: [&str; 3]| names.iter().map(ToString::to_string).collect();
    definitions
        .families
        .insert(FontFamily::Proportional, family(SANS));
    definitions
        .families
        .insert(FontFamily::Monospace, family(MONO));
    definitions
}

/// The sizes egui's own widgets draw at, in px as the CSS writes them.
pub fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Small, FontId::proportional(12.0)),
        (TextStyle::Body, FontId::proportional(BASE)),
        (TextStyle::Button, FontId::proportional(BASE)),
        (TextStyle::Heading, FontId::proportional(18.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_names_faces_that_are_embedded() {
        let definitions = definitions();

        for faces in definitions.families.values() {
            for face in faces {
                assert!(definitions.font_data.contains_key(face), "{face}");
            }
        }
    }

    #[test]
    fn the_faces_are_truetype_files() {
        for (name, bytes) in FACES {
            assert_eq!(&bytes[..4], &[0, 1, 0, 0], "{name}");
        }
    }
}
