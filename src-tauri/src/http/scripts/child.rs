//! The script host: the application started again with `ARG`, read in `main` before Tauri, so
//! no window, no single instance and no library. It reads a phase at a time on its standard
//! input, runs it, and writes on its standard output what the scripts ask and what they left —
//! one JSON message a line.

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::rc::Rc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::engine::{Engine, Host};
use super::model::{FailureKind, Job, LogEntry, Outcome, ScriptFailure};

pub const ARG: &str = "--devnotes-script-host";

/// The thread `QuickJS` runs on: a script that recurses meets its own limit long before the
/// thread's end.
const STACK: usize = 16 * 1024 * 1024;

/// What the application writes to the host.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToHost {
    Run { job: Box<Job>, millis: u64 },
    Reply { id: u64, answer: String },
}

/// What the host writes back.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FromHost {
    Call {
        id: u64,
        kind: String,
        payload: String,
    },
    Log {
        entry: LogEntry,
    },
    Done {
        outcome: Box<Outcome>,
    },
}

pub(crate) fn write_line<W: Write, T: Serialize>(
    output: &mut W,
    message: &T,
) -> std::io::Result<()> {
    serde_json::to_writer(&mut *output, message)?;
    output.write_all(b"\n")?;
    output.flush()
}

/// The next message, `None` once the other side is gone. A line that does not read is skipped.
pub(crate) fn read_line<R: BufRead, T: for<'de> Deserialize<'de>>(input: &mut R) -> Option<T> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if let Ok(message) = serde_json::from_str(&line) {
            return Some(message);
        }
    }
}

struct Channel<R, W> {
    input: R,
    output: W,
    last: u64,
}

struct Stdio<R, W>(Rc<RefCell<Channel<R, W>>>);

fn refusal(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

impl<R: BufRead, W: Write> Host for Stdio<R, W> {
    fn call(&mut self, kind: &str, payload: &str) -> String {
        let mut channel = self.0.borrow_mut();
        channel.last += 1;
        let id = channel.last;
        let asked = FromHost::Call {
            id,
            kind: kind.to_string(),
            payload: payload.to_string(),
        };
        if write_line(&mut channel.output, &asked).is_err() {
            return refusal("The application stopped answering");
        }
        loop {
            match read_line(&mut channel.input) {
                Some(ToHost::Reply {
                    id: answered,
                    answer,
                }) if answered == id => return answer,
                Some(_) => {}
                None => return refusal("The application stopped answering"),
            }
        }
    }

    fn log(&mut self, entry: LogEntry) {
        let _ = write_line(&mut self.0.borrow_mut().output, &FromHost::Log { entry });
    }
}

/// Serves until the application closes the input: `0`, or `1` when it could not answer.
pub fn serve_on<R: BufRead + 'static, W: Write + 'static>(input: R, output: W) -> i32 {
    let channel = Rc::new(RefCell::new(Channel {
        input,
        output,
        last: 0,
    }));
    let engine = Engine::new(Box::new(Stdio(Rc::clone(&channel))));
    loop {
        let message = read_line(&mut channel.borrow_mut().input);
        let (job, millis) = match message {
            Some(ToHost::Run { job, millis }) => (job, millis),
            Some(ToHost::Reply { .. }) => continue,
            None => return 0,
        };
        let outcome = match &engine {
            Ok(engine) => engine.run(&job, Duration::from_millis(millis)),
            Err(error) => Outcome::failed(ScriptFailure {
                kind: FailureKind::Host,
                message: error.to_string(),
                ..ScriptFailure::default()
            }),
        };
        let done = FromHost::Done {
            outcome: Box::new(outcome),
        };
        if write_line(&mut channel.borrow_mut().output, &done).is_err() {
            return 1;
        }
    }
}

/// The host's `main`, on a thread with room for `QuickJS`.
pub fn serve() -> i32 {
    std::thread::Builder::new()
        .name("script-host".to_string())
        .stack_size(STACK)
        .spawn(|| serve_on(std::io::stdin().lock(), std::io::stdout().lock()))
        .ok()
        .and_then(|worker| worker.join().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::scripts::model::{Phase, Script};
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn run_message(source: &str) -> String {
        let job = Job {
            phase: Phase::Before,
            scripts: vec![Script {
                id: "request:before".to_string(),
                source: source.to_string(),
                ..Script::default()
            }],
            ..Job::default()
        };
        serde_json::to_string(&ToHost::Run {
            job: Box::new(job),
            millis: 2000,
        })
        .unwrap()
    }

    fn written(output: &Shared) -> Vec<FromHost> {
        let bytes = output.0.lock().unwrap().clone();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn a_job_is_answered_with_its_logs_its_calls_and_its_outcome_then_the_host_ends() {
        let reply = serde_json::to_string(&ToHost::Reply {
            id: 1,
            answer: serde_json::json!({ "ok": [] }).to_string(),
        })
        .unwrap();
        let input = format!(
            "{}\nnot json\n{reply}\n",
            run_message(
                "console.log('hi'); pm.cookies.jar().getAll('https://x.y', (e, all) => env.set('n', all.length));"
            )
        );
        let output = Shared::default();
        assert_eq!(serve_on(Cursor::new(input), output.clone()), 0);

        let messages = written(&output);
        assert!(matches!(&messages[0], FromHost::Log { entry } if entry.text == "hi"));
        assert!(matches!(&messages[1], FromHost::Call { id: 1, kind, .. } if kind == "jarAll"));
        let FromHost::Done { outcome } = &messages[2] else {
            panic!("{messages:?}");
        };
        assert_eq!(outcome.failure, None);
        assert_eq!(outcome.writes[0].value.as_deref(), Some("0"));
    }

    #[test]
    fn a_call_left_unanswered_fails_the_script_rather_than_hanging() {
        let input = format!("{}\n", run_message("await pm.sendRequest('https://x.y');"));
        let output = Shared::default();
        serve_on(Cursor::new(input), output.clone());
        let messages = written(&output);
        let Some(FromHost::Done { outcome }) = messages.last() else {
            panic!("{messages:?}");
        };
        assert!(
            outcome
                .failure
                .as_ref()
                .is_some_and(|failure| failure.message.contains("stopped answering"))
        );
    }
}
