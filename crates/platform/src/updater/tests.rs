//! Against a `latest.json` served here, signed by a key made for these tests alone: its secret
//! half was thrown away the day these lines were written.

use std::io::{Read, Write};
use std::net::TcpListener;

use super::system::{Source, System, built};
use super::*;

const KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDBCQTA4OEQ1OTk4NjRFNUMKUldSY1RvYVoxWWlnQzUydCtPdXFHU2g0OUdCbXBhc2ZlSlU4aGRzSzhudG1yQ1F0eFpJOWpsaDAK";
const OTHER_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDcwRkNBOUI4MjJGQzkyQkUKUldTK2t2d2l1S244Y1AyWmJObmEvVHJiRTJVSHp5MVYvQ0tkSC8zRXo2aDBIUzB6NzRHcDlaYzYK";
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3QKUlVSY1RvYVoxWWlnQzZyWmJPNDlJb2hSWittZEk4OEFpOHZxL3FYOXFhdC9kd05tbXZmUlM2emFYWUNlRERlRFMwRFFyWEFMYUlzR1gxWE5zeFVORytjS1FWUW1mTWkybFFZPQp0cnVzdGVkIGNvbW1lbnQ6IHRlc3QKV3NLaERNVGg1ZERtQmI1NmZSSk0xVFJDc1c5bWpmbjlmRDkwazQ5WlU1NllyaG92QXNvT2R5cEZRZjI5dWFwS0NHd2RFaytBQytQNDVBeFBTUlJVQXc9PQo=";
const INSTALLER: &[u8] = b"the installer of a test release";

/// Answers `/latest.json` and `/installer`, for as long as the test runs.
fn serve(version: &str, installer: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let file = format!("{origin}/installer");
    let entry = serde_json::json!({ "signature": SIGNATURE, "url": file, "format": "nsis" });
    let manifest = serde_json::json!({
        "version": version,
        "notes": "### Added\n\n- Faster search",
        "pub_date": "2026-11-06T00:00:00Z",
        "platforms": { "windows-x86_64": entry, "linux-x86_64": entry },
    })
    .to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = [0; 1024];
            let read = stream.read(&mut request).unwrap();
            let asked = String::from_utf8_lossy(&request[..read]);
            let body: &[u8] = if asked.starts_with("GET /latest.json") {
                manifest.as_bytes()
            } else {
                installer
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        }
    });
    origin
}

fn client(origin: &str, version: &str, public_key: &str) -> System {
    let endpoint = format!("{origin}/latest.json");
    let source = Source {
        version,
        endpoint: &endpoint,
        public_key,
    };
    built(&source, true).unwrap()
}

#[test]
fn a_later_version_is_offered_with_its_notes() {
    let origin = serve("0.26.0", INSTALLER);

    let found = client(&origin, "0.25.0", KEY).check().unwrap().unwrap();

    assert_eq!(found.version, "0.26.0");
    assert_eq!(found.current_version, "0.25.0");
    assert_eq!(found.notes.as_deref(), Some("### Added\n\n- Faster search"));
}

#[test]
fn the_running_version_and_an_older_one_offer_nothing() {
    let origin = serve("0.26.0", INSTALLER);

    assert_eq!(client(&origin, "0.26.0", KEY).check(), Ok(None));
    assert_eq!(client(&origin, "0.27.0", KEY).check(), Ok(None));
}

#[test]
fn versions_compare_as_numbers_and_not_as_text() {
    let origin = serve("0.100.0", INSTALLER);

    assert!(client(&origin, "0.26.0", KEY).check().unwrap().is_some());
}

#[test]
fn a_file_signed_by_our_key_is_kept_and_its_progress_said() {
    let origin = serve("0.26.0", INSTALLER);
    let updater = client(&origin, "0.25.0", KEY);
    updater.check().unwrap();
    let last = std::cell::Cell::new(0);

    updater.download(&|percent| last.set(percent)).unwrap();

    assert_eq!(last.get(), 100);
    assert_eq!(
        updater.downloaded.lock().unwrap().as_deref(),
        Some(INSTALLER)
    );
}

#[test]
fn a_tampered_file_is_refused_and_kept_nowhere() {
    let origin = serve("0.26.0", b"the installer of a test releasE");
    let updater = client(&origin, "0.25.0", KEY);
    updater.check().unwrap();

    let answer = updater.download(&|_| {});

    assert!(
        matches!(answer, Err(DownloadError::Refused(_))),
        "{answer:?}"
    );
    assert!(updater.downloaded.lock().unwrap().is_none());
}

#[test]
fn a_file_signed_by_another_key_is_refused() {
    let origin = serve("0.26.0", INSTALLER);
    let updater = client(&origin, "0.25.0", OTHER_KEY);
    updater.check().unwrap();

    let answer = updater.download(&|_| {});

    assert!(
        matches!(answer, Err(DownloadError::Refused(_))),
        "{answer:?}"
    );
}

#[test]
fn nothing_found_leaves_nothing_to_download() {
    let origin = serve("0.26.0", INSTALLER);
    let updater = client(&origin, "0.25.0", KEY);

    let answer = updater.download(&|_| {});

    assert!(matches!(answer, Err(DownloadError::Failed(_))));
}

#[test]
fn an_address_that_does_not_answer_is_an_error_and_not_a_crash() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);

    assert!(client(&origin, "0.25.0", KEY).check().is_err());
}

#[test]
fn a_version_that_does_not_read_builds_no_client() {
    let source = Source {
        version: "dev",
        endpoint: "https://exemple.fr/latest.json",
        public_key: KEY,
    };

    assert!(built(&source, true).is_none());
}

#[test]
fn an_installation_that_cannot_update_offers_nothing() {
    assert!(!NoUpdater.is_available());
    assert_eq!(NoUpdater.check(), Ok(None));
    assert!(NoUpdater.download(&|_| {}).is_err());
}
