//! A Dessin's small picture, for its card: asked for once the card is drawn (a list carries
//! none), decoded off the UI thread, uploaded once, and kept for the last cards that showed one
//! (plan § 3.6 « Images »). It is library content: it lives in `LibraryCaches`.

use std::collections::{HashMap, VecDeque};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use egui::{ColorImage, TextureHandle, TextureOptions};
use engine::error::AppError;

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
    /// The drawing has no picture, or it could not be read.
    Missing,
    Shown(&'a TextureHandle),
}

enum Entry {
    Reading(Pending<Option<String>>),
    Decoding(Pending<Option<ColorImage>>),
    Done(Option<TextureHandle>),
}

#[derive(Default)]
pub struct Thumbnails {
    entries: HashMap<Stamp, Entry>,
    /// Oldest first: what goes when more than `KEPT` are done.
    done: VecDeque<Stamp>,
}

impl Thumbnails {
    pub fn get(&self, stamp: &Stamp) -> Thumbnail<'_> {
        match self.entries.get(stamp) {
            None => Thumbnail::Unknown,
            Some(Entry::Reading(_) | Entry::Decoding(_)) => Thumbnail::Waiting,
            Some(Entry::Done(None)) => Thumbnail::Missing,
            Some(Entry::Done(Some(texture))) => Thumbnail::Shown(texture),
        }
    }

    pub fn ask(&mut self, backend: &Backend, stamp: Stamp) {
        if self.entries.contains_key(&stamp) {
            return;
        }
        let reading = api::notes::drawing_thumbnail(backend, stamp.id.clone());
        self.entries.insert(stamp, Entry::Reading(reading));
    }

    /// Its pictures were taken, which moves no date: what is held of it is asked again.
    pub fn redrawn(&mut self, id: &str) {
        self.entries.retain(|stamp, _| stamp.id != id);
        self.done.retain(|stamp| stamp.id != id);
    }

    /// Once a frame: what was read goes to be decoded, what was decoded is uploaded.
    pub fn update(&mut self, ctx: &egui::Context, backend: &Backend) {
        let mut finished = Vec::new();
        for (stamp, entry) in &mut self.entries {
            match entry {
                Entry::Reading(pending) => match pending.poll() {
                    Some(Ok(Some(uri))) => {
                        *entry = Entry::Decoding(backend.compute(move |_| Ok(decode(&uri))));
                    }
                    Some(answer) => {
                        log_failure(answer.err());
                        *entry = Entry::Done(None);
                        finished.push(stamp.clone());
                    }
                    None => {}
                },
                Entry::Decoding(pending) => {
                    if let Some(answer) = pending.poll() {
                        let image = answer.ok().flatten();
                        let name = format!("drawing-{}", stamp.id);
                        let texture = image
                            .map(|image| ctx.load_texture(name, image, TextureOptions::LINEAR));
                        *entry = Entry::Done(texture);
                        finished.push(stamp.clone());
                    }
                }
                Entry::Done(_) => {}
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

fn log_failure(error: Option<AppError>) {
    if let Some(error) = error {
        log::warn!("A drawing's picture was not read: {:?}", error.code);
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
