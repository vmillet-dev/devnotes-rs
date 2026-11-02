//! The rules of the db queue: commands touch the library one at a time, in the order they were
//! sent, and a send writes only into the library it read from.

use std::io::Write;
use std::net::TcpListener;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;

use engine::Engine;
use engine::db::{Db, open_in_memory};
use engine::http::model::RequestScripts;
use engine::http::scripts::process::host_from;
use engine::http::{count_http_cookies, http_environments, http_history, send_http_request};
use engine::notes::model::NotePatch;
use engine::notes::{create_note, get_note, update_note};
use tauri::Manager;

use super::Session;
use super::common::snippet;
use super::http_send::{request, response, serve};

#[test]
fn two_edits_sent_in_a_row_land_in_that_order() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = session
        .call(|app| create_note(snippet(&space), app))
        .unwrap()
        .note
        .id;
    let titled = |title: &str| NotePatch {
        title: Some(title.to_string()),
        ..NotePatch::default()
    };

    let app = session.0.handle().clone();
    let (first, second) = tauri::async_runtime::block_on(async {
        tokio::join!(
            update_note(id.clone(), titled("first"), app.clone()),
            update_note(id.clone(), titled("second"), app.clone()),
        )
    });
    first.unwrap();
    second.unwrap();

    let read = session.call(|app| get_note(id.clone(), app)).unwrap();
    assert_eq!(read.note.title, "second");
}

/// A server that answers its one request, setting a cookie, only once it is told to.
fn held() -> (u16, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (arrived, heard) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        super::http_send::read_request(&mut stream);
        arrived.send(()).unwrap();
        released.recv().unwrap();
        let answer = response("200 OK", &[("Set-Cookie", "session=old; Path=/")], b"");
        let _ = stream.write_all(&answer);
    });
    (port, heard, release)
}

#[test]
fn a_send_that_ends_after_another_library_was_opened_writes_nothing_into_it() {
    let session = Session::open();
    let (port, arrived, release) = held();

    let sending = tauri::async_runtime::spawn(send_http_request(
        request(&format!("http://127.0.0.1:{port}/")),
        session.0.handle().clone(),
    ));
    arrived.recv().unwrap();
    // What `open_library` then `unlock_vault` leave: another library, another number.
    *session.0.state::<Arc<Engine>>().db().lock().unwrap() = Some(open_in_memory().unwrap());
    release.send(()).unwrap();
    tauri::async_runtime::block_on(sending).unwrap().unwrap();

    assert_eq!(session.call(count_http_cookies).unwrap(), 0);
    assert!(session.call(|app| http_history(0, app)).unwrap().is_empty());
}

#[test]
fn two_sends_at_once_keep_both_cookies_and_both_variables() {
    host_from(PathBuf::from(env!("CARGO_BIN_EXE_script-host")));
    let session = Session::open();
    let (port, _seen) = serve(2, |seen, _| {
        let cookie = if seen.head.starts_with("GET /a ") {
            "a=1; Path=/"
        } else {
            "b=2; Path=/"
        };
        response("200 OK", &[("Set-Cookie", cookie)], b"")
    });
    let sent = |path: &str, script: &str| {
        let mut sent = request(&format!("http://127.0.0.1:{port}/{path}"));
        sent.id = format!("tab-{path}");
        sent.document.scripts = RequestScripts {
            after: script.to_string(),
            ..RequestScripts::default()
        };
        sent
    };

    let app = session.0.handle().clone();
    let (a, b) = tauri::async_runtime::block_on(async {
        tokio::join!(
            send_http_request(sent("a", "env.set('first', 'a');"), app.clone()),
            send_http_request(sent("b", "env.set('second', 'b');"), app.clone()),
        )
    });
    a.unwrap();
    b.unwrap();

    assert_eq!(session.call(count_http_cookies).unwrap(), 2);
    let mut globals: Vec<String> = session
        .call(http_environments)
        .unwrap()
        .globals
        .into_iter()
        .map(|variable| variable.name)
        .collect();
    globals.sort();
    assert_eq!(globals, ["first", "second"]);
}

#[cfg(debug_assertions)]
#[test]
fn the_library_is_never_locked_off_the_db_thread() {
    let db = Db::new(Some(open_in_memory().unwrap()));

    let locked = catch_unwind(AssertUnwindSafe(|| {
        engine::http::history::sweep_at_startup(&db);
    }));

    assert!(locked.is_err());
}
