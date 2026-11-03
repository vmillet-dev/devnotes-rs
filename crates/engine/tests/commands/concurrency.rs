//! The rules of the db queue: commands touch the library one at a time, in the order they were
//! sent, and a send writes only into the library it read from.

use std::io::Write;
use std::net::TcpListener;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;

use engine::db::{Db, open_in_memory};
use engine::error::ErrorCode;
use engine::http::model::RequestScripts;
use engine::http::scripts::process::host_from;
use engine::http::{count_http_cookies, http_environments, http_history, send_http_request};
use engine::libraries::{create_library, open_library};
use engine::notes::model::NotePatch;
use engine::notes::{create_note, get_note, update_note};

use super::Session;
use super::common::snippet;
use super::http_send::{request, response, serve};

#[test]
fn two_edits_sent_in_a_row_land_in_that_order() {
    let session = Session::open();
    let space = session.space("Personal");
    let id = session
        .call(|e| create_note(e, snippet(&space)))
        .unwrap()
        .note
        .id;
    let titled = |title: &str| NotePatch {
        title: Some(title.to_string()),
        ..NotePatch::default()
    };

    let edit = |title: &'static str| {
        let id = id.clone();
        engine::db::run(Arc::clone(&session.engine), move |e| {
            update_note(e, id, titled(title))
        })
    };
    let (first, second) = session
        .runtime
        .block_on(async { tokio::join!(edit("first"), edit("second")) });
    first.unwrap();
    second.unwrap();

    let read = session.call(|e| get_note(e, id.clone())).unwrap();
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

    let sending = session.runtime.spawn(send_http_request(
        Arc::clone(&session.engine),
        request(&format!("http://127.0.0.1:{port}/")),
    ));
    arrived.recv().unwrap();
    // What `open_library` then `unlock_vault` leave: another library, another number.
    *session.engine.db().lock().unwrap() = Some(open_in_memory().unwrap());
    release.send(()).unwrap();
    session.runtime.block_on(sending).unwrap().unwrap();

    assert_eq!(session.call(count_http_cookies).unwrap(), 0);
    assert!(session.call(|e| http_history(e, 0)).unwrap().is_empty());
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

    let engine = &session.engine;
    let (a, b) = session.runtime.block_on(async {
        tokio::join!(
            send_http_request(Arc::clone(engine), sent("a", "env.set('first', 'a');")),
            send_http_request(Arc::clone(engine), sent("b", "env.set('second', 'b');")),
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

#[test]
fn opening_another_library_cancels_the_send_under_way() {
    let session = Session::open();
    let (port, arrived, _release) = held();
    let other = session
        .call(|e| create_library(e, "Other".to_string()))
        .unwrap();

    let sending = session.runtime.spawn(send_http_request(
        Arc::clone(&session.engine),
        request(&format!("http://127.0.0.1:{port}/")),
    ));
    arrived.recv().unwrap();
    session.call(|e| open_library(e, other.id)).unwrap();

    let answer = session.runtime.block_on(sending).unwrap();
    assert_eq!(answer.unwrap_err().code, ErrorCode::HttpCancelled);
}
