//! The files of the note open: it follows the editor, and holds no bytes but the one image shown.
//! A file waits here until the note has a row: a draft is written for it, and only then.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use egui::{ColorImage, TextureHandle, TextureOptions};
use engine::attachments::model::Attachment;
use i18n::{Message, keys};
use platform::desktop::Opener;
use platform::dialog::FileDialog;

use super::Revision;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;

/// D28's closed list; the rest offers « Open ».
const SHOWN: [&str; 8] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/x-icon",
    "image/vnd.microsoft.icon",
    "image/svg+xml",
];

pub fn is_shown(attachment: &Attachment) -> bool {
    SHOWN.contains(&attachment.mime_type.as_str())
}

/// What the strip asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// From the system's dialog.
    Pick,
    Dropped(Vec<PathBuf>),
    /// Named after its moment, by the state, which holds the clock.
    Pasted(Arc<ColorImage>),
    Open(String),
    Save(String),
    Remove(String),
    TogglePreview(String),
    Zoom(bool),
}

/// What is left to attach once the note has a row.
enum ToAttach {
    File(PathBuf),
    Image(String, Arc<ColorImage>),
}

enum Saving {
    Choosing(String, Pending<Option<PathBuf>>),
    Writing(String, Pending<()>),
}

enum Image {
    Reading(Pending<String>),
    Decoding(Pending<Option<ColorImage>>),
    Shown(TextureHandle),
}

struct Preview {
    id: String,
    image: Image,
}

/// What the answers tell the rest of the application.
pub struct Reports<'a> {
    pub status: &'a mut StatusNotifier,
    pub errors: &'a mut ErrorNotifier,
    pub revision: &'a mut Revision,
}

#[derive(Default)]
pub struct AttachmentsState {
    note: Option<String>,
    list: Vec<Attachment>,
    reading: Option<Pending<Vec<Attachment>>>,
    queue: VecDeque<ToAttach>,
    /// One at a time: the strip says so.
    attaching: Option<Pending<Attachment>>,
    picking: Vec<Pending<Option<PathBuf>>>,
    removing: Vec<(String, Pending<()>)>,
    saving: Vec<Saving>,
    opening: Vec<Pending<PathBuf>>,
    preview: Option<Preview>,
    zoomed: bool,
}

impl AttachmentsState {
    pub fn list(&self) -> &[Attachment] {
        &self.list
    }

    pub fn is_busy(&self) -> bool {
        self.attaching.is_some() || !self.queue.is_empty()
    }

    pub fn preview_id(&self) -> Option<&str> {
        self.preview.as_ref().map(|preview| preview.id.as_str())
    }

    /// The image shown, once it is decoded.
    pub fn preview_image(&self) -> Option<&TextureHandle> {
        match &self.preview.as_ref()?.image {
            Image::Shown(texture) => Some(texture),
            _ => None,
        }
    }

    /// The name and image the lightbox shows.
    pub fn zoomed(&self) -> Option<(&str, &TextureHandle)> {
        let shown = self.preview_image().filter(|_| self.zoomed)?;
        let id = self.preview_id()?;
        let attachment = self.list.iter().find(|attachment| attachment.id == id)?;
        Some((attachment.file_name.as_str(), shown))
    }

    /// A file waits for the note's row: the draft is to be written.
    pub fn wants_row(&self) -> bool {
        self.note.is_none() && !self.queue.is_empty()
    }

    pub fn ask(
        &mut self,
        asked: Asked,
        backend: &Backend,
        dialog: &Arc<dyn FileDialog + Send + Sync>,
        pasted_name: String,
    ) {
        match asked {
            Asked::Pick => {
                let dialog = Arc::clone(dialog);
                self.picking
                    .push(backend.compute(move |_| Ok(dialog.open())));
            }
            Asked::Dropped(paths) => self.queue.extend(paths.into_iter().map(ToAttach::File)),
            Asked::Pasted(image) => self.queue.push_back(ToAttach::Image(pasted_name, image)),
            Asked::Open(id) => self
                .opening
                .push(api::attachments::decrypted_copy(backend, id)),
            Asked::Save(id) => self.save(backend, dialog, id),
            Asked::Remove(id) => {
                let removal = api::attachments::delete(backend, id.clone());
                self.removing.push((id, removal));
            }
            Asked::TogglePreview(id) => self.toggle_preview(backend, id),
            Asked::Zoom(on) => self.zoomed = on,
        }
    }

    fn save(&mut self, backend: &Backend, dialog: &Arc<dyn FileDialog + Send + Sync>, id: String) {
        let Some(attachment) = self.list.iter().find(|attachment| attachment.id == id) else {
            return;
        };
        let name = attachment.file_name.clone();
        let dialog = Arc::clone(dialog);
        let chosen = backend.compute(move |_| Ok(dialog.save(&name)));
        self.saving.push(Saving::Choosing(id, chosen));
    }

    /// A toggle: asking again for the image shown closes it.
    fn toggle_preview(&mut self, backend: &Backend, id: String) {
        self.zoomed = false;
        if self.preview_id() == Some(id.as_str()) {
            self.preview = None;
            return;
        }
        let reading = api::attachments::read(backend, id.clone());
        self.preview = Some(Preview {
            id,
            image: Image::Reading(reading),
        });
    }

    /// Once a frame. `note` is the note open, by id; another one clears the image shown: the
    /// last note's screenshot would be worse than nothing.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        note: Option<&str>,
        opener: &Arc<dyn Opener + Send + Sync>,
        reports: &mut Reports<'_>,
    ) {
        self.follow(backend, note);
        self.adopt_list(reports);
        self.adopt_picked(reports);
        self.attach_next(backend);
        self.adopt_attached(backend, reports);
        self.adopt_removed(backend, reports);
        self.adopt_saved(backend, reports);
        self.adopt_opened(opener, reports);
        self.adopt_preview(ctx, backend, reports);
    }

    fn follow(&mut self, backend: &Backend, note: Option<&str>) {
        if self.note.as_deref() == note {
            return;
        }
        self.note = note.map(str::to_owned);
        self.list.clear();
        self.preview = None;
        self.zoomed = false;
        self.reading = None;
        self.load(backend);
    }

    fn load(&mut self, backend: &Backend) {
        if let Some(note) = &self.note {
            self.reading = Some(api::attachments::list(backend, note.clone()));
        }
    }

    fn adopt_list(&mut self, reports: &mut Reports<'_>) {
        let Some(answer) = self.reading.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.reading = None;
        match answer {
            Ok(list) => self.list = list,
            Err(error) => reports
                .errors
                .report(keys::errors::ATTACHMENTS_LOAD_FAILED, &error),
        }
    }

    fn adopt_picked(&mut self, reports: &mut Reports<'_>) {
        let queue = &mut self.queue;
        self.picking.retain_mut(|pending| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            match answer {
                Ok(Some(path)) => queue.push_back(ToAttach::File(path)),
                Ok(None) => {}
                Err(error) => reports.errors.report(keys::errors::ATTACH_FAILED, &error),
            }
            false
        });
    }

    fn attach_next(&mut self, backend: &Backend) {
        let Some(note) = self.note.clone().filter(|_| self.attaching.is_none()) else {
            return;
        };
        self.attaching = match self.queue.pop_front() {
            Some(ToAttach::File(path)) => Some(api::attachments::attach(backend, note, &path)),
            Some(ToAttach::Image(name, image)) => {
                Some(api::attachments::attach_image(backend, note, name, &image))
            }
            None => None,
        };
    }

    /// The file name is announced, and an image shown: otherwise attaching is only visible in
    /// the strip.
    fn adopt_attached(&mut self, backend: &Backend, reports: &mut Reports<'_>) {
        let Some(answer) = self.attaching.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.attaching = None;
        match answer {
            Ok(added) => {
                self.load(backend);
                reports.revision.bump();
                let said =
                    Message::new(keys::attachments::ADDED).with("name", added.file_name.as_str());
                reports.status.report(said);
                if is_shown(&added) {
                    self.toggle_preview(backend, added.id);
                }
            }
            Err(error) => reports.errors.report(keys::errors::ATTACH_FAILED, &error),
        }
    }

    fn adopt_removed(&mut self, backend: &Backend, reports: &mut Reports<'_>) {
        let mut removed = Vec::new();
        self.removing.retain_mut(|(id, pending)| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            match answer {
                Ok(()) => removed.push(std::mem::take(id)),
                Err(error) => reports
                    .errors
                    .report(keys::errors::ATTACHMENT_DELETE_FAILED, &error),
            }
            false
        });
        for id in removed {
            if self.preview_id() == Some(id.as_str()) {
                self.preview = None;
                self.zoomed = false;
            }
            self.load(backend);
            reports.revision.bump();
        }
    }

    fn adopt_saved(&mut self, backend: &Backend, reports: &mut Reports<'_>) {
        let saving = std::mem::take(&mut self.saving);
        for step in saving {
            match step {
                Saving::Choosing(id, mut chosen) => match chosen.poll() {
                    None => self.saving.push(Saving::Choosing(id, chosen)),
                    Some(Ok(Some(path))) => {
                        let path = path.to_string_lossy().into_owned();
                        let writing = api::attachments::save(backend, id, path.clone());
                        self.saving.push(Saving::Writing(path, writing));
                    }
                    Some(Ok(None)) => {}
                    Some(Err(error)) => reports
                        .errors
                        .report(keys::errors::ATTACHMENT_SAVE_FAILED, &error),
                },
                Saving::Writing(path, mut writing) => match writing.poll() {
                    None => self.saving.push(Saving::Writing(path, writing)),
                    Some(Ok(())) => {
                        let said = Message::new(keys::attachments::SAVED).with("path", path);
                        reports.status.report(said);
                    }
                    Some(Err(error)) => reports
                        .errors
                        .report(keys::errors::ATTACHMENT_SAVE_FAILED, &error),
                },
            }
        }
    }

    fn adopt_opened(&mut self, opener: &Arc<dyn Opener + Send + Sync>, reports: &mut Reports<'_>) {
        self.opening.retain_mut(|pending| {
            let Some(answer) = pending.poll() else {
                return true;
            };
            match answer {
                Ok(path) => {
                    if opener.open(&path).is_err() {
                        reports
                            .errors
                            .notify(Message::new(keys::errors::ATTACHMENT_OPEN_FAILED));
                    }
                }
                Err(error) => reports
                    .errors
                    .report(keys::errors::ATTACHMENT_OPEN_FAILED, &error),
            }
            false
        });
    }

    fn adopt_preview(&mut self, ctx: &egui::Context, backend: &Backend, reports: &mut Reports<'_>) {
        let Some(preview) = &mut self.preview else {
            return;
        };
        match &mut preview.image {
            Image::Reading(pending) => match pending.poll() {
                None => {}
                Some(Ok(uri)) => {
                    preview.image = Image::Decoding(backend.compute(move |_| Ok(decode(&uri))));
                }
                Some(Err(error)) => {
                    reports
                        .errors
                        .report(keys::errors::ATTACHMENT_READ_FAILED, &error);
                    self.preview = None;
                }
            },
            Image::Decoding(pending) => match pending.poll() {
                None => {}
                Some(Ok(Some(image))) => {
                    let name = format!("attachment-{}", preview.id);
                    let texture = ctx.load_texture(name, image, TextureOptions::LINEAR);
                    preview.image = Image::Shown(texture);
                }
                // A file that says it is an image and is none: « Open » still offers it.
                Some(_) => self.preview = None,
            },
            Image::Shown(_) => {}
        }
    }
}

/// An image in the closed list, as pixels; `None` for anything else.
fn decode(uri: &str) -> Option<ColorImage> {
    let (head, payload) = uri.split_once(";base64,")?;
    let bytes = BASE64.decode(payload).ok()?;
    if head == "data:image/svg+xml" {
        return decode_svg(&bytes);
    }
    let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let size = [
        usize::try_from(image.width()).ok()?,
        usize::try_from(image.height()).ok()?,
    ];
    Some(ColorImage::from_rgba_unmultiplied(size, image.as_raw()))
}

fn decode_svg(bytes: &[u8]) -> Option<ColorImage> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let size = [
        usize::try_from(pixmap.width()).ok()?,
        usize::try_from(pixmap.height()).ok()?,
    ];
    Some(ColorImage::from_rgba_premultiplied(size, pixmap.data()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    fn attachment(mime: &str) -> Attachment {
        Attachment {
            id: "a".to_owned(),
            note_id: "n".to_owned(),
            file_name: "a".to_owned(),
            mime_type: mime.to_owned(),
            byte_size: 1,
            created_at: chrono::DateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn only_the_closed_list_is_shown() {
        assert!(is_shown(&attachment("image/png")));
        assert!(is_shown(&attachment("image/svg+xml")));
        assert!(!is_shown(&attachment("image/tiff")));
        assert!(!is_shown(&attachment("application/pdf")));
    }

    #[test]
    fn a_png_and_an_svg_decode_to_their_pixels() {
        let png = decode(&format!("data:image/png;base64,{PNG}")).unwrap();
        assert_eq!(png.size, [1, 1]);

        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="3"/>"#;
        let uri = format!("data:image/svg+xml;base64,{}", BASE64.encode(svg));
        assert_eq!(decode(&uri).unwrap().size, [4, 3]);
    }

    #[test]
    fn what_is_no_image_decodes_to_nothing() {
        assert_eq!(decode("not a uri"), None);
        assert_eq!(decode("data:image/png;base64,AAAA"), None);
        let uri = format!("data:image/svg+xml;base64,{}", BASE64.encode("<nope"));
        assert_eq!(decode(&uri), None);
    }
}
