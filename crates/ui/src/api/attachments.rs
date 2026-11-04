//! The files of a note.

use std::path::PathBuf;

use egui::ColorImage;
use engine::attachments::{self, model::Attachment};

use crate::backend::{Backend, Pending};

pub fn list(backend: &Backend, note: String) -> Pending<Vec<Attachment>> {
    backend.db(move |engine| attachments::list_attachments(engine, note))
}

pub fn attach(backend: &Backend, note: String, path: PathBuf) -> Pending<Attachment> {
    let path = path.to_string_lossy().into_owned();
    backend.db(move |engine| attachments::attach_file(engine, note, path))
}

/// The pixels are the core's to encode and seal.
pub fn attach_image(
    backend: &Backend,
    note: String,
    name: String,
    image: &ColorImage,
) -> Pending<Attachment> {
    let [width, height] = image.size;
    let size = (
        u32::try_from(width).unwrap_or(u32::MAX),
        u32::try_from(height).unwrap_or(u32::MAX),
    );
    let rgba: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_srgba_unmultiplied())
        .collect();
    backend.db(move |engine| attachments::attach_image(engine, note, name, size, &rgba))
}

/// The file as a `data:` URI.
pub fn read(backend: &Backend, id: String) -> Pending<String> {
    backend.db(move |engine| attachments::read_attachment(engine, id))
}

/// ⚠️ The one decrypted copy that reaches the disk: for the program the desktop picks.
pub fn decrypted_copy(backend: &Backend, id: String) -> Pending<PathBuf> {
    backend.db(move |engine| attachments::decrypted_copy(engine, id))
}

pub fn save(backend: &Backend, id: String, path: String) -> Pending<()> {
    backend.db(move |engine| attachments::save_attachment(engine, id, path))
}

pub fn delete(backend: &Backend, id: String) -> Pending<()> {
    backend.db(move |engine| attachments::delete_attachment(engine, id))
}
