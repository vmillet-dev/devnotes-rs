//! The README's tours: one theme throughout, and light enough for a page.

use std::path::Path;

use image::AnimationDecoder;
use image::codecs::gif::GifDecoder;

const HEAVIEST: usize = 1_000_000;
/// In the title bar's corner, clear of everything on it: its ground, in every frame but a
/// presentation's, whose ground is as dark.
const GROUND: (u32, u32) = (3, 3);
const DARK: u8 = 40;

#[test]
fn every_frame_of_every_tour_is_dark_and_each_file_under_a_megabyte() {
    for name in ["notes", "tools", "http"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = path.join(format!("../../docs/tour-{name}.gif"));
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() < HEAVIEST, "{name}: {} bytes", bytes.len());

        let decoder = GifDecoder::new(std::io::Cursor::new(bytes)).unwrap();
        let mut drawn = 0;
        for frame in decoder.into_frames() {
            let frame = frame.unwrap();
            let whole = frame.buffer().pixels().all(|pixel| pixel.0[3] == 255);
            assert!(whole, "{name}, frame {drawn}: seen through");
            let ground = frame.buffer().get_pixel(GROUND.0, GROUND.1);
            let brightest = ground.0[..3].iter().max().unwrap();
            assert!(*brightest < DARK, "{name}, frame {drawn}: {ground:?}");
            drawn += 1;
        }
        assert!(drawn > 5, "{name}: {drawn} frames");
    }
}
