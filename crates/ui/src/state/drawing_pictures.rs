//! A Dessin's pictures: the small one for its card, asked for once the card is drawn (a list
//! carries none), and the large one for a Note that embeds it, asked by its id where it is shown.
//! Decoded off the UI thread, uploaded once, and kept for the last ones shown.
//! It is library content: it lives in `LibraryCaches`. A drawing that came with no
//! picture, as an import leaves it, has its two taken here, the first time it is shown.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use egui::{ColorImage, TextureHandle, TextureOptions};
use engine::error::AppError;
use engine::notes::drawing::DrawingPictures;
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, NotePatch};

use crate::api;
use crate::backend::{Backend, Pending};

/// How many pictures stay uploaded: a few screens of cards.
const KEPT: usize = 256;
const PNG: &str = "data:image/png;base64,";

/// A drawing as it was last saved: the same drawing saved again asks again.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stamp {
    pub id: String,
    pub updated_at: DateTime<Utc>,
}

/// What a card has to draw.
pub enum Thumbnail<'a> {
    /// Never asked: the card asks.
    Unknown,
    Waiting,
    /// Nothing is drawn on it.
    Empty,
    /// No drawing, or its picture could not be read.
    Missing,
    Shown(&'a TextureHandle),
}

enum Entry {
    Reading(Pending<Option<String>>),
    /// It has no picture: its scene is read, for one to be taken.
    Scene(Pending<DisplayNote>),
    Taking(Pending<DrawingPictures>),
    Decoding(Pending<Option<ColorImage>>),
    Empty,
    Missing,
    Shown(TextureHandle),
}

impl Entry {
    fn is_done(&self) -> bool {
        matches!(self, Self::Empty | Self::Missing | Self::Shown(_))
    }
}

/// Which of a drawing's pictures a cache reads.
type Reader = fn(&Backend, String) -> Pending<Option<String>>;
/// The same one, of the two just taken.
type Taken = fn(DrawingPictures) -> String;

pub struct Thumbnails {
    entries: HashMap<Stamp, Entry>,
    /// Oldest first: what goes when more than `KEPT` are done.
    done: VecDeque<Stamp>,
    /// The pictures taken here, being written.
    writes: Vec<Pending<DisplayNote>>,
    reader: Reader,
    taken: Taken,
}

impl Default for Thumbnails {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            done: VecDeque::new(),
            writes: Vec::new(),
            reader: api::notes::drawing_thumbnail,
            taken: |pictures| pictures.thumbnail,
        }
    }
}

impl Thumbnails {
    /// The large pictures, which a Note embeds by id: its stamp is the id alone.
    pub fn large() -> Self {
        Self {
            reader: api::notes::drawing_picture,
            taken: |pictures| pictures.picture,
            ..Self::default()
        }
    }

    /// An embed's stamp: the id is all a Note holds; a drawing saved again drops it.
    pub fn embedded(id: &str) -> Stamp {
        Stamp {
            id: id.to_owned(),
            updated_at: DateTime::UNIX_EPOCH,
        }
    }

    pub fn get(&self, stamp: &Stamp) -> Thumbnail<'_> {
        match self.entries.get(stamp) {
            None => Thumbnail::Unknown,
            Some(Entry::Empty) => Thumbnail::Empty,
            Some(Entry::Missing) => Thumbnail::Missing,
            Some(Entry::Shown(texture)) => Thumbnail::Shown(texture),
            Some(_) => Thumbnail::Waiting,
        }
    }

    pub fn ask(&mut self, backend: &Backend, stamp: Stamp) {
        if self.entries.contains_key(&stamp) {
            return;
        }
        let reading = (self.reader)(backend, stamp.id.clone());
        self.entries.insert(stamp, Entry::Reading(reading));
    }

    /// Its pictures were taken, which moves no date: what is held of it is asked again.
    pub fn redrawn(&mut self, id: &str) {
        self.entries.retain(|stamp, _| stamp.id != id);
        self.done.retain(|stamp| stamp.id != id);
    }

    /// Once a frame: each picture under way goes to its next step.
    pub fn update(&mut self, ctx: &egui::Context, backend: &Backend) {
        self.writes.retain_mut(|write| match write.poll() {
            None => true,
            Some(answer) => {
                log_failure(answer.err());
                false
            }
        });
        let mut finished = Vec::new();
        for (stamp, entry) in &mut self.entries {
            if entry.is_done() {
                continue;
            }
            if let Some(next) = advance(ctx, backend, stamp, entry, (&mut self.writes, self.taken))
            {
                *entry = next;
            }
            if entry.is_done() {
                finished.push(stamp.clone());
            }
        }
        self.done.extend(finished);
        while self.done.len() > KEPT {
            if let Some(oldest) = self.done.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

/// What an entry becomes once what it waits for has answered.
fn advance(
    ctx: &egui::Context,
    backend: &Backend,
    stamp: &Stamp,
    entry: &mut Entry,
    (writes, taken): (&mut Vec<Pending<DisplayNote>>, Taken),
) -> Option<Entry> {
    let decoding = |uri: String| Entry::Decoding(backend.compute(move |_| Ok(decode(&uri))));
    let failed = |error: AppError| {
        log_failure(Some(error));
        Entry::Missing
    };
    Some(match entry {
        Entry::Reading(pending) => match pending.poll()? {
            Ok(Some(uri)) => decoding(uri),
            Ok(None) => Entry::Scene(api::notes::get(backend, stamp.id.clone())),
            Err(error) => failed(error),
        },
        Entry::Scene(pending) => match pending.poll()? {
            Ok(note) if note.kind == NoteKind::Drawing => {
                let palette = crate::notes::picture_palette(crate::theme::palette(ctx));
                let scene = note.content.clone();
                Entry::Taking(api::notes::picture_drawing(
                    backend,
                    scene,
                    Arc::new(palette),
                ))
            }
            Ok(_) => Entry::Missing,
            Err(error) => failed(error),
        },
        Entry::Taking(pending) => match pending.poll()? {
            Ok(pictures) if pictures.thumbnail.is_empty() => Entry::Empty,
            Ok(pictures) => {
                let patch = NotePatch {
                    pictures: Some(pictures.clone()),
                    ..NotePatch::default()
                };
                writes.push(api::notes::update(backend, stamp.id.clone(), patch));
                decoding(taken(pictures))
            }
            Err(error) => failed(error),
        },
        Entry::Decoding(pending) => match pending.poll()?.ok().flatten() {
            Some(image) => {
                let name = format!("drawing-{}", stamp.id);
                Entry::Shown(ctx.load_texture(name, image, TextureOptions::LINEAR))
            }
            None => Entry::Missing,
        },
        Entry::Empty | Entry::Missing | Entry::Shown(_) => return None,
    })
}

fn log_failure(error: Option<AppError>) {
    if let Some(error) = error {
        log::warn!("A drawing's picture was not read or kept: {:?}", error.code);
    }
}

/// The PNG the core keeps, as pixels; `None` for anything else.
fn decode(uri: &str) -> Option<ColorImage> {
    let bytes = BASE64.decode(uri.strip_prefix(PNG)?).ok()?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut pixels).ok()?;
    let size = [
        usize::try_from(frame.width).ok()?,
        usize::try_from(frame.height).ok()?,
    ];
    let pixels = &pixels[..frame.buffer_size()];
    match frame.color_type {
        png::ColorType::Rgba => Some(ColorImage::from_rgba_unmultiplied(size, pixels)),
        png::ColorType::Rgb => Some(ColorImage::from_rgb(size, pixels)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 × 1 PNG, red then transparent.
    fn png_uri() -> String {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[255, 0, 0, 255, 0, 0, 0, 0])
                .unwrap();
        }
        format!("{PNG}{}", BASE64.encode(bytes))
    }

    #[test]
    fn a_picture_the_core_keeps_decodes_to_its_pixels() {
        let image = decode(&png_uri()).unwrap();

        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels[0], egui::Color32::RED);
    }

    #[test]
    fn anything_else_decodes_to_nothing() {
        assert!(decode("data:image/png;base64,***").is_none());
        assert!(decode("data:image/svg+xml,<svg/>").is_none());
    }
}
