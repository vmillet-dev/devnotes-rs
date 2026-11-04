//! A Textbox's lines (D23). Fabric works them out at render time from the stored width: this is
//! its `_wrapLine`, once, for the editor and the pictures alike. It measures by the advances of
//! the embedded fonts, where a browser kerns: a word can break earlier or later (E14).

use std::sync::Arc;

use resvg::usvg::fontdb;
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};

/// A glyph no embedded font has, in ems.
const NO_GLYPH: f64 = 0.5;

/// Fabric's ratio of a line's box to the font size, and how far above the box's foot the
/// baseline sits.
const SIZE_MULTIPLIER: f64 = 1.13;
const SIZE_FRACTION: f64 = 0.222;

/// The fonts embedded in the app, in the order a character looks for its glyph.
pub struct Fonts {
    faces: Vec<FontRef<'static>>,
    pub(crate) database: Arc<fontdb::Database>,
    pub(crate) family: String,
}

impl Fonts {
    pub fn new(files: &[&'static [u8]]) -> Self {
        let mut database = fontdb::Database::new();
        for file in files {
            database.load_font_data(file.to_vec());
        }
        let family = database
            .faces()
            .next()
            .and_then(|face| face.families.first())
            .map_or_else(String::new, |(name, _)| name.clone());
        database.set_sans_serif_family(family.clone());
        Self {
            faces: files
                .iter()
                .filter_map(|file| FontRef::new(file).ok())
                .collect(),
            database: Arc::new(database),
            family,
        }
    }

    /// How far a character moves the pen, in ems.
    fn advance(&self, character: char) -> f64 {
        self.faces
            .iter()
            .find_map(|face| advance_in(face, character))
            .unwrap_or(NO_GLYPH)
    }

    fn width(&self, word: &str, font_size: f64) -> f64 {
        word.chars()
            .map(|character| self.advance(character) * font_size)
            .sum()
    }
}

fn advance_in(face: &FontRef<'_>, character: char) -> Option<f64> {
    let glyph = face.charmap().map(character)?;
    let metrics = face.glyph_metrics(Size::new(1.0), LocationRef::default());
    metrics.advance_width(glyph).map(f64::from)
}

/// A text as Fabric lays it out in a box of that width: at the spaces only, a hard line for each
/// line written, the longest word widening the box rather than broken.
pub fn wrap(text: &str, width: f64, font_size: f64, fonts: &Fonts) -> Vec<String> {
    let lines: Vec<Vec<&str>> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .map(|line| line.split([' ', '\t', '\r']).collect())
        .collect();
    let longest = lines
        .iter()
        .flatten()
        .map(|word| fonts.width(word, font_size))
        .fold(0.0, f64::max);
    let room = width.max(longest);
    let space = fonts.width(" ", font_size);
    lines
        .iter()
        .flat_map(|words| wrap_line(words, room, space, font_size, fonts))
        .collect()
}

fn wrap_line(words: &[&str], room: f64, space: f64, font_size: f64, fonts: &Fonts) -> Vec<String> {
    let mut wrapped = Vec::new();
    let mut line = String::new();
    let mut line_width = 0.0;
    let mut started = false;
    for word in words {
        let word_width = fonts.width(word, font_size);
        line_width += if started { space } else { 0.0 } + word_width;
        if line_width > room && started {
            wrapped.push(std::mem::take(&mut line));
            line_width = word_width;
            started = false;
        }
        if started {
            line.push(' ');
        }
        line.push_str(word);
        started = true;
    }
    wrapped.push(line);
    wrapped
}

/// From one baseline to the next.
pub fn line_step(font_size: f64, line_height: f64) -> f64 {
    font_size * SIZE_MULTIPLIER * line_height
}

/// The first baseline, down from the top of the text's box.
pub fn first_baseline(font_size: f64) -> f64 {
    font_size * SIZE_MULTIPLIER * (1.0 - SIZE_FRACTION)
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "a constant, read back")]
pub(crate) mod tests;
