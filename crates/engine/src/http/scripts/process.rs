//! The application's side of the script host: one process for one request — its phase before
//! and its phase after share `pm.variables` —, its calls answered here, killed when a phase
//! runs past its time and when dropped.

use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::child::{self, FromHost, read_line, write_line};
use super::model::{FailureKind, Job, LogEntry, Outcome, ScriptFailure};

/// What the host keeps of the environment: what the system needs to load the executable. A
/// token in a variable of the user's session has no business in it.
const KEPT_ENVIRONMENT: &[&str] = &["SystemRoot", "LD_LIBRARY_PATH"];

/// Past its own deadline a phase ends by itself; past this one more, the host is killed.
const GRACE: Duration = Duration::from_secs(2);

/// Set once by a test, whose harness is not the application.
static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();

/// Starts every host from `executable` instead of the application itself.
pub fn host_from(executable: PathBuf) {
    let _ = EXECUTABLE.set(executable);
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A call a script made, refused: what the script's `Error` says, and its `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub message: String,
    pub code: Option<String>,
}

impl Refusal {
    #[must_use]
    pub fn new(message: impl Into<String>, code: Option<&str>) -> Self {
        Self {
            message: message.into(),
            code: code.map(str::to_string),
        }
    }
}

/// What the scripts of a request reach through the host.
pub trait Services {
    /// `kind` with its JSON payload, as the prelude asked; `Ok` is what its `ok` carries.
    fn call(&mut self, kind: &str, payload: &str) -> Result<serde_json::Value, Refusal>;
    fn log(&mut self, entry: LogEntry);
}

/// Stops a host from another thread: what « Annuler » reaches while a script runs.
#[derive(Clone)]
pub struct Killer {
    child: Arc<Mutex<Child>>,
    killed: Arc<AtomicBool>,
}

impl Killer {
    pub fn kill(&self) {
        self.killed.store(true, Ordering::SeqCst);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }

    #[must_use]
    pub fn killed(&self) -> bool {
        self.killed.load(Ordering::SeqCst)
    }
}

pub struct ScriptProcess {
    child: Arc<Mutex<Child>>,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    killed: Arc<AtomicBool>,
}

fn host_failure(message: &str) -> Outcome {
    Outcome::failed(ScriptFailure {
        kind: FailureKind::Host,
        message: message.to_string(),
        ..ScriptFailure::default()
    })
}

impl ScriptProcess {
    /// The application itself, started as a script host.
    pub fn start() -> std::io::Result<Self> {
        match EXECUTABLE.get() {
            Some(executable) => Self::start_at(executable),
            None => Self::start_at(&std::env::current_exe()?),
        }
    }

    pub fn start_at(executable: &Path) -> std::io::Result<Self> {
        let mut command = Command::new(executable);
        command
            .arg(child::ARG)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for name in KEPT_ENVIRONMENT {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn()?;
        let missing = || std::io::Error::other("the script host has no standard streams");
        let input = child.stdin.take().ok_or_else(missing)?;
        let output = BufReader::new(child.stdout.take().ok_or_else(missing)?);
        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            input: Some(input),
            output,
            killed: Arc::default(),
        })
    }

    #[must_use]
    pub fn killer(&self) -> Killer {
        Killer {
            child: Arc::clone(&self.child),
            killed: Arc::clone(&self.killed),
        }
    }

    /// One phase, within `time`, its calls answered by `services`.
    pub fn run(&mut self, job: &Job, time: Duration, services: &mut dyn Services) -> Outcome {
        let Some(input) = self.input.as_mut() else {
            return host_failure("The script host is closed");
        };
        let millis = u64::try_from(time.as_millis()).unwrap_or(u64::MAX);
        let asked = serde_json::json!({ "type": "run", "job": job, "millis": millis });
        if write_line(input, &asked).is_err() {
            return host_failure("The script host cannot be reached");
        }

        let (finished, watched) = mpsc::channel::<()>();
        let child = Arc::clone(&self.child);
        let started = Instant::now();
        let watchdog = std::thread::spawn(move || {
            if watched.recv_timeout(time + GRACE) == Err(RecvTimeoutError::Timeout)
                && let Ok(mut child) = child.lock()
            {
                let _ = child.kill();
            }
        });

        let outcome = loop {
            match read_line::<_, FromHost>(&mut self.output) {
                Some(FromHost::Call { id, kind, payload }) => {
                    let answer = match services.call(&kind, &payload) {
                        Ok(ok) => serde_json::json!({ "ok": ok }),
                        Err(refusal) => {
                            serde_json::json!({ "error": refusal.message, "code": refusal.code })
                        }
                    };
                    let reply = child::ToHost::Reply {
                        id,
                        answer: answer.to_string(),
                    };
                    if self
                        .input
                        .as_mut()
                        .is_none_or(|input| write_line(input, &reply).is_err())
                    {
                        break host_failure("The script host cannot be reached");
                    }
                }
                Some(FromHost::Log { entry }) => services.log(entry),
                Some(FromHost::Done { outcome }) => break *outcome,
                None if started.elapsed() >= time => {
                    break Outcome::failed(ScriptFailure {
                        kind: FailureKind::Timeout,
                        message: format!("The scripts ran past {millis} ms"),
                        ..ScriptFailure::default()
                    });
                }
                None => break host_failure("The script host stopped"),
            }
        };
        let _ = finished.send(());
        let _ = watchdog.join();
        if outcome
            .failure
            .as_ref()
            .is_some_and(|failure| matches!(failure.kind, FailureKind::Timeout | FailureKind::Host))
        {
            self.stop();
        }
        outcome
    }

    /// A host that timed out may be anywhere in a script: it is not given another phase.
    fn stop(&mut self) {
        self.input = None;
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ScriptProcess {
    fn drop(&mut self) {
        // Its input closed, the host ends by itself; the kill is for one that would not.
        if let Some(mut input) = self.input.take() {
            let _ = input.flush();
        }
        self.stop();
    }
}
