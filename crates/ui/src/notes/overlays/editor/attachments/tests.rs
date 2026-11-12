use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::{ColorImage, Event};
use egui_kittest::kittest::Queryable;
use engine::attachments::model::Attachment;

use crate::notes::bench::{Bench, click_id, expanded_area, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

const PNG: [u8; 67] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xe4, 0xfb, 0xcf, 0x50,
    0x0f, 0x00, 0x03, 0x86, 0x01, 0x80, 0x5a, 0x34, 0x7d, 0x6b, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae,
];

#[derive(Default)]
struct Opened(Mutex<Vec<PathBuf>>);

impl platform::desktop::Opener for Opened {
    fn open(&self, path: &Path) -> std::io::Result<()> {
        self.0.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

/// A snippet open in the editor, and a folder for its files.
fn opened() -> (Harness, String, tempfile::TempDir) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "with files").id.clone();
    let mut harness = expanded_area(backend, profile);
    harness.state_mut().state.editor.open(id.clone());
    settle(&mut harness);
    (harness, id, tempfile::tempdir().unwrap())
}

fn files(harness: &Harness, id: &str) -> Vec<Attachment> {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::attachments::list_attachments(engine, id)
    })
    .unwrap()
}

fn answering(harness: &mut Harness, path: &Path) {
    let dialog = platform::dialog::Fixed(path.to_path_buf());
    harness.state_mut().state.dialog = Arc::new(dialog);
}

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}

fn drop_file(harness: &mut Harness, path: &Path) {
    let file = Dropped(path.to_path_buf());
    harness.input_mut().dropped_files.push(Arc::new(file));
    settle(harness);
}

/// Big enough to be clicked.
fn paste_image(harness: &mut Harness) {
    let image = ColorImage::filled([80, 60], egui::Color32::RED);
    harness.event(Event::PasteImage(Arc::new(image)));
    settle(harness);
}

fn has(harness: &Harness, id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(id))
        .is_some()
}

fn text_file(folder: &Path) -> PathBuf {
    let path = folder.join("runbook.txt");
    std::fs::write(&path, "step one\n").unwrap();
    path
}

fn image_file(folder: &Path) -> PathBuf {
    let path = folder.join("shot.png");
    std::fs::write(&path, PNG).unwrap();
    path
}

#[test]
fn the_strip_says_it_is_empty_until_a_file_is_chosen_and_names_it() {
    let (mut harness, id, folder) = opened();
    assert!(has(&harness, "attachment-empty"));
    answering(&mut harness, &text_file(folder.path()));

    click_id(&mut harness, "attachment-add");
    settle(&mut harness);

    assert_eq!(files(&harness, &id)[0].file_name, "runbook.txt");
    assert!(!has(&harness, "attachment-empty"));
    assert!(has(&harness, "attachment-open"));
    let state = &harness.state().state;
    let said = state.status.latest().unwrap().1.text(state.language);
    assert!(said.contains("runbook.txt"), "{said}");
}

#[test]
fn a_file_dropped_on_the_window_is_attached() {
    let (mut harness, id, folder) = opened();

    drop_file(&mut harness, &text_file(folder.path()));

    assert_eq!(files(&harness, &id).len(), 1);
}

#[test]
fn an_image_pasted_is_attached_under_the_moment_and_shown() {
    let (mut harness, id, _folder) = opened();

    paste_image(&mut harness);

    let attached = files(&harness, &id);
    assert_eq!(attached[0].mime_type, "image/png");
    assert!(attached[0].file_name.starts_with("capture-"));
    assert!(has(&harness, "attachment-zoom"));
}

#[test]
fn an_image_shows_at_once_grows_in_the_lightbox_and_goes_with_escape() {
    let (mut harness, _, _folder) = opened();
    paste_image(&mut harness);
    assert!(has(&harness, "attachment-zoom"));

    click_id(&mut harness, "attachment-zoom");
    assert!(has(&harness, "lightbox-image"));

    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(!has(&harness, "lightbox-image"));
    assert!(has(&harness, "editor-title"), "only the lightbox went");

    click_id(&mut harness, "attachment-preview-close");
    assert!(!has(&harness, "attachment-zoom"));
    click_id(&mut harness, "attachment-preview");
    settle(&mut harness);
    assert!(has(&harness, "attachment-zoom"));
}

#[test]
fn a_lightbox_closes_by_its_button() {
    let (mut harness, _, _folder) = opened();
    paste_image(&mut harness);
    click_id(&mut harness, "attachment-zoom");

    click_id(&mut harness, "lightbox-close");

    assert!(!has(&harness, "lightbox-image"));
}

#[test]
fn a_file_that_is_no_image_offers_no_preview() {
    let (mut harness, _, folder) = opened();

    drop_file(&mut harness, &text_file(folder.path()));

    assert!(!has(&harness, "attachment-preview"));
}

#[test]
fn a_file_that_calls_itself_an_image_and_is_none_shows_nothing() {
    let (mut harness, _, folder) = opened();
    let path = folder.path().join("broken.png");
    std::fs::write(&path, "no image").unwrap();

    drop_file(&mut harness, &path);

    assert!(has(&harness, "attachment-open"));
    assert!(!has(&harness, "attachment-zoom"));
}

#[test]
fn the_first_click_arms_the_cross_and_the_second_removes() {
    let (mut harness, id, folder) = opened();
    drop_file(&mut harness, &text_file(folder.path()));

    click_id(&mut harness, "attachment-remove");
    settle(&mut harness);
    assert_eq!(files(&harness, &id).len(), 1);

    click_id(&mut harness, "attachment-remove");
    settle(&mut harness);
    assert!(files(&harness, &id).is_empty());
    assert!(has(&harness, "attachment-empty"));
}

#[test]
fn removing_the_image_shown_closes_it() {
    let (mut harness, _, folder) = opened();
    drop_file(&mut harness, &image_file(folder.path()));

    click_id(&mut harness, "attachment-remove");
    click_id(&mut harness, "attachment-remove");
    settle(&mut harness);

    assert!(!has(&harness, "attachment-zoom"));
}

#[test]
fn a_file_is_saved_where_the_dialog_says_and_the_toast_says_where() {
    let (mut harness, _, folder) = opened();
    drop_file(&mut harness, &text_file(folder.path()));
    let target = folder.path().join("copy.txt");
    answering(&mut harness, &target);

    click_id(&mut harness, "attachment-save");
    settle(&mut harness);

    assert_eq!(std::fs::read_to_string(&target).unwrap(), "step one\n");
    let state = &harness.state().state;
    let said = state.status.latest().unwrap().1.text(state.language);
    assert!(said.contains("copy.txt"), "{said}");
}

#[test]
fn a_name_clicked_goes_to_the_desktop_as_a_decrypted_copy() {
    let (mut harness, _, folder) = opened();
    drop_file(&mut harness, &text_file(folder.path()));
    let opener = Arc::new(Opened::default());
    harness.state_mut().state.opener = opener.clone();

    click_id(&mut harness, "attachment-open");
    settle(&mut harness);

    let paths = opener.0.lock().unwrap();
    assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), "step one\n");
}

#[test]
fn a_file_over_the_limit_is_refused_and_says_so() {
    let (mut harness, id, folder) = opened();
    let path = folder.path().join("capture.mov");
    std::fs::write(&path, vec![0; 10 * 1024 * 1024 + 1]).unwrap();

    drop_file(&mut harness, &path);

    assert!(files(&harness, &id).is_empty());
    assert!(harness.state().state.errors.notice().is_some());
}

#[test]
fn a_draft_is_written_for_the_file_dropped_on_it() {
    let (backend, profile) = testing::unlocked();
    let space = testing::note(&backend, "seed").space_id.clone();
    let mut harness = expanded_area(backend, profile);
    let state = &mut harness.state_mut().state;
    state
        .editor
        .draft(engine::notes::kind::NoteKind::Snippet, space, None);
    settle(&mut harness);
    let folder = tempfile::tempdir().unwrap();

    drop_file(&mut harness, &text_file(folder.path()));
    settle(&mut harness);

    let id = harness.state().state.editor.open_id().map(str::to_owned);
    assert_eq!(files(&harness, &id.unwrap()).len(), 1);
}

#[test]
fn another_note_shows_its_own_files() {
    let (mut harness, id, folder) = opened();
    drop_file(&mut harness, &image_file(folder.path()));
    let other = testing::note(&harness.state().backend, "another")
        .id
        .clone();

    harness.state_mut().state.editor.open(other);
    settle(&mut harness);

    assert!(has(&harness, "attachment-empty"));
    assert!(!has(&harness, "attachment-zoom"));
    harness.state_mut().state.editor.open(id);
    settle(&mut harness);
    assert!(has(&harness, "attachment-open"));
}

#[test]
fn a_dialog_cancelled_attaches_nothing() {
    struct Cancelled;
    impl platform::dialog::FileDialog for Cancelled {
        fn open(&self) -> Option<PathBuf> {
            None
        }
        fn save(&self, _name: &str) -> Option<PathBuf> {
            None
        }
    }
    let (mut harness, id, _folder) = opened();
    harness.state_mut().state.dialog = Arc::new(Cancelled);

    click_id(&mut harness, "attachment-add");
    settle(&mut harness);

    assert!(files(&harness, &id).is_empty());
}
