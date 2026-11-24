//! Assembles the README's tours from the screens `app::screens` draws:
//! `DEVNOTES_SCREENS=<dir> cargo test -p ui --lib app::screens`, then
//! `cargo run -p app --example tours -- <dir>`, which writes `docs/tour-*.gif`.
//!
//! One theme throughout, the dark one: a loop that ends on a light frame flashes.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba, RgbaImage};

/// A tour and its screens, in the order of the README's caption.
const TOURS: [(&str, &[&str]); 3] = [
    (
        "notes",
        &[
            "notes-list",
            "notes-board",
            "notes-search",
            "notes-facets",
            "notes-selection",
            "editor-snippet-fields",
            "notes-fields-form",
            "editor-snippet-revisions",
            "editor-note",
            "editor-note-presentation",
            "editor-checklist",
            "editor-calc",
            "notes-palette",
            "notes-trash",
        ],
    ),
    (
        "tools",
        &[
            "tools-home",
            "tool-jwt",
            "tool-json-generator",
            "tool-diff",
            "tool-hash",
            "tool-password",
            "tool-dates",
            "tool-zones",
            "tool-cron",
            "tool-cidr",
            "tool-colour",
            "tool-qr",
            "tool-identity",
            "tool-http-status",
        ],
    ),
    (
        "http",
        &[
            "http-home",
            "http-request",
            "http-graphql",
            "http-websocket",
            "http-environments",
            "http-code",
        ],
    ),
];

const SHOWN: Duration = Duration::from_secs(2);
/// What a GIF holds.
const COLOURS: usize = 256;

fn main() {
    let screens = std::env::args_os().nth(1).map(PathBuf::from);
    let screens = screens.expect("the folder the screens were drawn in");
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    for (name, frames) in TOURS {
        let path = docs.join(format!("tour-{name}.gif"));
        let frames: Vec<RgbaImage> = frames.iter().map(|one| screen(&screens, one)).collect();
        write(&path, &frames);
        let bytes = std::fs::metadata(&path).expect("written").len();
        println!("{} {} frames, {bytes} bytes", path.display(), frames.len());
    }
}

fn screen(screens: &Path, name: &str) -> RgbaImage {
    let path = screens.join(format!("{name}--dark.png"));
    let image = image::open(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    image.into_rgba8()
}

/// One palette for the whole loop, or a ground would change its shade from a frame to the next.
/// ⚠️ Each frame whole: `image` writes frames that clear what is under them.
fn write(path: &Path, frames: &[RgbaImage]) {
    let palette = palette(frames);
    let mut nearest = HashMap::new();
    let file = File::create(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite).expect("a loop");
    for frame in frames {
        let mut reduced = frame.clone();
        for pixel in reduced.pixels_mut() {
            *pixel = *nearest
                .entry(*pixel)
                .or_insert_with(|| closest(&palette, *pixel));
        }
        let delay = Delay::from_saturating_duration(SHOWN);
        encoder
            .encode_frame(Frame::from_parts(reduced, 0, 0, delay))
            .expect("a frame");
    }
}

/// The colours most seen, each far enough from the ones before it: grounds and inks stay exact,
/// and what is left goes to the edges of the letters.
fn palette(frames: &[RgbaImage]) -> Vec<Rgba<u8>> {
    let mut seen: HashMap<Rgba<u8>, u32> = HashMap::new();
    for pixel in frames.iter().flat_map(RgbaImage::pixels) {
        *seen.entry(*pixel).or_default() += 1;
    }
    let mut seen: Vec<(Rgba<u8>, u32)> = seen.into_iter().collect();
    seen.sort_unstable_by_key(|(colour, count)| (std::cmp::Reverse(*count), colour.0));
    let mut apart = 0;
    loop {
        let mut kept: Vec<Rgba<u8>> = Vec::new();
        for (colour, _) in &seen {
            if kept.iter().all(|one| distance(*one, *colour) >= apart) {
                kept.push(*colour);
            }
            if kept.len() > COLOURS {
                break;
            }
        }
        if kept.len() <= COLOURS {
            return kept;
        }
        apart += 1;
    }
}

fn closest(palette: &[Rgba<u8>], colour: Rgba<u8>) -> Rgba<u8> {
    let by_distance = palette.iter().min_by_key(|one| distance(**one, colour));
    *by_distance.expect("a frame has colours")
}

fn distance(a: Rgba<u8>, b: Rgba<u8>) -> u32 {
    let channels = a.0.iter().zip(b.0);
    channels.map(|(a, b)| u32::from(a.abs_diff(b)).pow(2)).sum()
}
