//! Regenerates `crates/app/icons/` from `devnotes.svg`: `cargo run -p app --example icons`.
//!
//! ⚠️ 16, 24 and 32 px of the Windows `.ico` come from the hand-hinted drawings in
//! `icons/source/`, whose lines sit on whole pixels. Scaled from the master they blur, and 24 px
//! is the taskbar, 16 px the tray.

use std::path::{Path, PathBuf};

use resvg::{tiny_skia, usvg};

const HINTED: [u32; 3] = [16, 24, 32];
const SCALED: [u32; 3] = [48, 64, 256];
/// The window's icon, the packages' and the desktop entries'.
const PNGS: [(&str, u32); 3] = [("32x32.png", 32), ("128x128.png", 128), ("icon.png", 512)];

fn main() {
    let icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
    let master = icons.join("devnotes.svg");
    for (name, size) in PNGS {
        let source = if HINTED.contains(&size) {
            hinted(&icons, size)
        } else {
            master.clone()
        };
        write(&icons.join(name), &render(&source, size));
    }
    let entries: Vec<(u32, Vec<u8>)> = HINTED
        .iter()
        .map(|&size| (size, render(&hinted(&icons, size), size)))
        .chain(SCALED.iter().map(|&size| (size, render(&master, size))))
        .collect();
    write(&icons.join("icon.ico"), &ico(&entries));
}

fn hinted(icons: &Path, size: u32) -> PathBuf {
    icons.join("source").join(format!("{size}.svg"))
}

/// The drawing at `size` × `size` px, as a PNG.
fn render(source: &Path, size: u32) -> Vec<u8> {
    let data =
        std::fs::read(source).unwrap_or_else(|error| panic!("{}: {error}", source.display()));
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default())
        .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a non-empty size");
    #[allow(
        clippy::cast_precision_loss,
        reason = "an icon is a few hundred pixels wide"
    )]
    let scale = size as f32 / tree.size().width().max(tree.size().height());
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().expect("a PNG of a pixmap")
}

/// An `.ico` of PNG entries, which Windows reads since Vista.
fn ico(entries: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(entries.len()).expect("a handful of sizes");
    let mut header = Vec::new();
    header.extend_from_slice(&0u16.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * entries.len();
    for (size, data) in entries {
        // 0 stands for 256.
        let side = u8::try_from(*size).unwrap_or(0);
        header.extend_from_slice(&[side, side, 0, 0]);
        header.extend_from_slice(&1u16.to_le_bytes());
        header.extend_from_slice(&32u16.to_le_bytes());
        header.extend_from_slice(
            &u32::try_from(data.len())
                .expect("a small PNG")
                .to_le_bytes(),
        );
        header.extend_from_slice(&u32::try_from(offset).expect("a small file").to_le_bytes());
        offset += data.len();
    }
    entries.iter().fold(header, |mut file, (_, data)| {
        file.extend_from_slice(data);
        file
    })
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}
