//! The two links of the About menu: a click hands the address to the desktop's browser.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use engine::app_info::METADATA;

use super::tests::{click, open_library};

#[derive(Default)]
struct Opened(Mutex<Vec<PathBuf>>);

impl platform::desktop::Opener for Opened {
    fn open(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.0.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

#[test]
fn the_repository_opens_in_the_browser_from_the_about_card() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    let opened = Arc::new(Opened::default());
    harness.state_mut().state.opener = Arc::clone(&opened) as _;

    click(&mut harness, "about-open");
    click(&mut harness, "about-about");
    click(&mut harness, "about-repository");

    assert_eq!(
        *opened.0.lock().unwrap(),
        [PathBuf::from(METADATA.repository)]
    );
    assert!(METADATA.repository.starts_with("https://"));
}

#[test]
fn the_releases_open_in_the_browser_from_whats_new() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    let opened = Arc::new(Opened::default());
    harness.state_mut().state.opener = Arc::clone(&opened) as _;

    click(&mut harness, "about-open");
    click(&mut harness, "about-whats-new");
    click(&mut harness, "news-releases");

    let wanted = format!("{}/releases", METADATA.repository);
    assert_eq!(*opened.0.lock().unwrap(), [PathBuf::from(wanted)]);
}
