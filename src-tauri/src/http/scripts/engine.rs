//! `QuickJS` running one phase's scripts, with the prelude's API. It holds no network, file or
//! clock of its own: what a script asks beyond the language goes to a `Host`. In the
//! application the host is the script process's link to its parent; in the tests, a fake.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use rquickjs::context::EvalOptions;
use rquickjs::promise::PromiseState;
use rquickjs::{Array, Context, Ctx, Function, Object, Promise, Runtime, Value};

use super::libraries;
use super::model::{FailureKind, Job, LogEntry, Outcome, ScriptFailure};

const PRELUDE: &str = include_str!("prelude.js");
/// What a phase may hold: a response of a megabyte parsed several times fits, a runaway does not.
const MEMORY: usize = 128 * 1024 * 1024;
/// `QuickJS`'s own count, which Ajv's compiler needs past a megabyte; the thread running it is
/// given twice this (`child::STACK`).
pub(crate) const STACK: usize = 8 * 1024 * 1024;
/// The quota of one `crypto.getRandomValues`, as browsers set it.
const RANDOM_AT_MOST: u32 = 65_536;

/// What the scripts reach beyond the language.
pub trait Host {
    /// A call the prelude waits for — a send, the jar —, answered `{ "ok": … }` or
    /// `{ "error": … }` as JSON.
    fn call(&mut self, kind: &str, payload: &str) -> String;
    fn log(&mut self, entry: LogEntry);
}

#[derive(Debug, thiserror::Error)]
#[error("The script engine cannot start: {0}")]
pub struct StartError(String);

pub struct Engine {
    context: Context,
    deadline: Rc<Cell<Instant>>,
}

/// A deadline nobody reaches: while the prelude loads, and between two phases.
fn unbounded() -> Instant {
    Instant::now() + Duration::from_hours(24)
}

fn options(filename: &str) -> EvalOptions {
    let mut options = EvalOptions::default();
    options.strict = false;
    options.filename = Some(filename.to_string());
    options
}

fn install<'js>(ctx: &Ctx<'js>, host: &Rc<RefCell<Box<dyn Host>>>) -> rquickjs::Result<()> {
    let native = Object::new(ctx.clone())?;
    let calls = Rc::clone(host);
    native.set(
        "call",
        Function::new(ctx.clone(), move |kind: String, payload: String| {
            calls.borrow_mut().call(&kind, &payload)
        })?,
    )?;
    let logs = Rc::clone(host);
    native.set(
        "log",
        Function::new(ctx.clone(), move |entry: String| {
            if let Ok(entry) = serde_json::from_str::<LogEntry>(&entry) {
                logs.borrow_mut().log(entry);
            }
        })?,
    )?;
    native.set(
        "random",
        Function::new(ctx.clone(), |count: u32| {
            let mut bytes = vec![0_u8; count.min(RANDOM_AT_MOST) as usize];
            // Without randomness the bytes stay zero, and a uuid says so plainly.
            let _ = getrandom::fill(&mut bytes);
            base64::engine::general_purpose::STANDARD.encode(bytes)
        })?,
    )?;
    native.set(
        "module",
        Function::new(
            ctx.clone(),
            |ctx: Ctx<'js>, name: String| -> rquickjs::Result<Option<Function<'js>>> {
                let Some(source) = libraries::source(&name) else {
                    return Ok(None);
                };
                let wrapped = format!("(function (module, exports, require) {{{source}\n}})");
                ctx.eval_with_options(wrapped, options(&format!("library:{name}")))
                    .map(Some)
            },
        )?,
    )?;
    ctx.globals().set("__native", native)?;
    ctx.eval_with_options::<(), _>(PRELUDE, options("prelude"))
}

/// `script-3:12:5` in a stack: the fourth script of the job, line 12.
fn place_in(stack: &str) -> Option<(usize, u32)> {
    let mut rest = stack;
    while let Some(at) = rest.find("script-") {
        rest = &rest[at + "script-".len()..];
        let index: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let after = &rest[index.len()..];
        if let Some(line) = after.strip_prefix(':') {
            let line: String = line.chars().take_while(char::is_ascii_digit).collect();
            if let (Ok(index), Ok(line)) = (index.parse(), line.parse()) {
                return Some((index, line));
            }
        }
    }
    None
}

/// What the engine threw, as a failure: the message, and the script and line it came from.
fn caught(ctx: &Ctx<'_>, error: &rquickjs::Error, job: &Job, kind: FailureKind) -> ScriptFailure {
    let mut failure = ScriptFailure {
        kind,
        message: error.to_string(),
        ..ScriptFailure::default()
    };
    if !matches!(error, rquickjs::Error::Exception) {
        return failure;
    }
    let value: Value<'_> = ctx.catch();
    if let Some(object) = value.as_object() {
        let read = |key: &str| object.get::<_, Option<String>>(key).ok().flatten();
        let message = read("message").unwrap_or_default();
        failure.message = match read("name") {
            Some(name) if !name.is_empty() => format!("{name}: {message}"),
            _ => message,
        };
        if let Some((index, line)) = read("stack").as_deref().and_then(place_in) {
            failure.script = job.scripts.get(index).map(|script| script.id.clone());
            failure.line = Some(line);
        }
    } else if let Some(text) = value.as_string().and_then(|text| text.to_string().ok()) {
        failure.message = text;
    }
    classify(&mut failure);
    failure
}

/// `QuickJS`'s own two: the interrupt at the deadline, and the heap's end. Thrown inside a script,
/// the prelude reports them as any error.
fn classify(failure: &mut ScriptFailure) {
    if failure.message.contains("InternalError: interrupted") {
        failure.kind = FailureKind::Timeout;
    } else if failure.message.contains("out of memory") {
        failure.kind = FailureKind::Memory;
    }
}

/// Each script as the body of an async function, compiled under a name of its own: its lines
/// stay the user's, and an `await` at its top level reads.
fn compile<'js>(ctx: &Ctx<'js>, job: &Job) -> Result<Array<'js>, ScriptFailure> {
    let failed = |error: &rquickjs::Error, kind| caught(ctx, error, job, kind);
    let functions = Array::new(ctx.clone()).map_err(|error| failed(&error, FailureKind::Error))?;
    for (index, script) in job.scripts.iter().enumerate() {
        let wrapped = format!("(async function () {{{}\n}})", script.source);
        let function: Function<'js> = ctx
            .eval_with_options(wrapped, options(&format!("script-{index}")))
            .map_err(|error| {
                let mut failure = failed(&error, FailureKind::Syntax);
                failure.script.get_or_insert_with(|| script.id.clone());
                failure
            })?;
        functions
            .set(index, function)
            .map_err(|error| failed(&error, FailureKind::Error))?;
    }
    Ok(functions)
}

fn timeout(time: Duration) -> ScriptFailure {
    ScriptFailure {
        kind: FailureKind::Timeout,
        message: format!("The scripts ran past {} ms", time.as_millis()),
        ..ScriptFailure::default()
    }
}

struct Loop<'js> {
    next_timer: Function<'js>,
    fire_timers: Function<'js>,
    wake_idle: Function<'js>,
}

fn drive(ctx: &Ctx<'_>, job: &Job, deadline: Instant, time: Duration) -> Outcome {
    let failed =
        |error: &rquickjs::Error| Outcome::failed(caught(ctx, error, job, FailureKind::Error));
    let functions = match compile(ctx, job) {
        Ok(functions) => functions,
        Err(failure) => return Outcome::failed(failure),
    };
    let started = (|| -> rquickjs::Result<(Promise<'_>, Loop<'_>)> {
        let devnotes: Object<'_> = ctx.globals().get("__devnotes")?;
        let phase: Function<'_> = devnotes.get("phase")?;
        let given = ctx.json_parse(serde_json::to_string(job).unwrap_or_default())?;
        let promise: Promise<'_> = phase.call((given, functions))?;
        let driven = Loop {
            next_timer: devnotes.get("nextTimer")?,
            fire_timers: devnotes.get("fireTimers")?,
            wake_idle: devnotes.get("wakeIdle")?,
        };
        Ok((promise, driven))
    })();
    let (promise, driven) = match started {
        Ok(started) => started,
        Err(error) => return failed(&error),
    };
    loop {
        while Instant::now() < deadline && ctx.execute_pending_job() {}
        match promise.state() {
            PromiseState::Resolved => {
                return match promise.result::<String>() {
                    Some(Ok(answer)) => {
                        let mut outcome: Outcome =
                            serde_json::from_str(&answer).unwrap_or_else(|error| {
                                Outcome::failed(ScriptFailure {
                                    message: error.to_string(),
                                    ..ScriptFailure::default()
                                })
                            });
                        if let Some(failure) = outcome.failure.as_mut() {
                            classify(failure);
                        }
                        outcome
                    }
                    Some(Err(error)) => failed(&error),
                    None => Outcome::failed(timeout(time)),
                };
            }
            PromiseState::Rejected => {
                return match promise.result::<Value<'_>>() {
                    Some(Err(error)) => failed(&error),
                    _ => Outcome::failed(timeout(time)),
                };
            }
            PromiseState::Pending => {}
        }
        let now = Instant::now();
        if now >= deadline {
            return Outcome::failed(timeout(time));
        }
        let next: f64 = match driven.next_timer.call(()) {
            Ok(next) => next,
            Err(error) => return failed(&error),
        };
        if next >= 0.0 {
            // A whole number of milliseconds, under a day: the cast loses nothing.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let wait = Duration::from_millis(next.ceil() as u64);
            std::thread::sleep(wait.min(deadline - now));
            if let Err(error) = driven.fire_timers.call::<_, ()>(()) {
                return failed(&error);
            }
            continue;
        }
        match driven.wake_idle.call::<_, bool>(()) {
            Ok(true) => {}
            Ok(false) => {
                return Outcome::failed(ScriptFailure {
                    kind: FailureKind::Timeout,
                    message: "The scripts wait on a promise that nothing settles".to_string(),
                    ..ScriptFailure::default()
                });
            }
            Err(error) => return failed(&error),
        }
    }
}

impl Engine {
    pub fn new(host: Box<dyn Host>) -> Result<Self, StartError> {
        let fault = |error: rquickjs::Error| StartError(error.to_string());
        let runtime = Runtime::new().map_err(fault)?;
        runtime.set_memory_limit(MEMORY);
        runtime.set_max_stack_size(STACK);
        let deadline = Rc::new(Cell::new(unbounded()));
        let watched = Rc::clone(&deadline);
        runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= watched.get())));
        let context = Context::full(&runtime).map_err(fault)?;
        let host = Rc::new(RefCell::new(host));
        context.with(|ctx| install(&ctx, &host)).map_err(fault)?;
        Ok(Self { context, deadline })
    }

    /// One phase, within `time`: what a later phase reads of this one (`pm.variables`) stays in
    /// the engine.
    pub fn run(&self, job: &Job, time: Duration) -> Outcome {
        let deadline = Instant::now() + time;
        self.deadline.set(deadline);
        let outcome = self.context.with(|ctx| drive(&ctx, job, deadline, time));
        self.deadline.set(unbounded());
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::scripts::model::{
        LayerScope, Phase, Script, ScriptKind, ScriptLayer, ScriptLevel, ScriptRequest,
        ScriptResponse, ScriptRow, WriteScope,
    };

    #[derive(Default)]
    struct Seen {
        calls: Vec<(String, serde_json::Value)>,
        logs: Vec<LogEntry>,
    }

    /// Answers every send with a 200 carrying the request's URL, and keeps what it was asked.
    struct Fake(Rc<RefCell<Seen>>);

    impl Host for Fake {
        fn call(&mut self, kind: &str, payload: &str) -> String {
            let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
            self.0
                .borrow_mut()
                .calls
                .push((kind.to_string(), payload.clone()));
            match kind {
                "send" if payload["method"] == "DELETE" => {
                    serde_json::json!({ "error": "refused", "code": "httpScriptProtected" })
                        .to_string()
                }
                "send" => serde_json::json!({ "ok": {
                    "code": 200,
                    "reason": "OK",
                    "headers": [{ "key": "Content-Type", "value": "application/json" }],
                    "body": serde_json::json!({ "url": payload["url"] }).to_string(),
                    "millis": 12,
                    "size": 40,
                }})
                .to_string(),
                _ => serde_json::json!({ "ok": [] }).to_string(),
            }
        }

        fn log(&mut self, entry: LogEntry) {
            self.0.borrow_mut().logs.push(entry);
        }
    }

    fn engine() -> (Engine, Rc<RefCell<Seen>>) {
        let seen = Rc::new(RefCell::new(Seen::default()));
        (Engine::new(Box::new(Fake(Rc::clone(&seen)))).unwrap(), seen)
    }

    fn script(id: &str, kind: ScriptKind, source: &str) -> Script {
        Script {
            id: id.to_string(),
            level: ScriptLevel::Request,
            kind,
            name: "Payer".to_string(),
            owner: None,
            source: source.to_string(),
        }
    }

    fn layer(scope: LayerScope, values: &[(&str, &str)]) -> ScriptLayer {
        ScriptLayer {
            scope,
            from: String::new(),
            values: values
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        }
    }

    fn before(source: &str) -> Job {
        Job {
            phase: Phase::Before,
            scripts: vec![script("request:before", ScriptKind::Before, source)],
            request: ScriptRequest {
                method: "POST".to_string(),
                url: "{{baseUrl}}/pay".to_string(),
                headers: vec![ScriptRow {
                    key: "Accept".to_string(),
                    value: "application/json".to_string(),
                    ..ScriptRow::default()
                }],
                ..ScriptRequest::default()
            },
            layers: vec![
                layer(
                    LayerScope::Environment,
                    &[("baseUrl", "https://api.example.com")],
                ),
                layer(LayerScope::Globals, &[("tenant", "acme")]),
            ],
            environment: Some("Staging".to_string()),
            ..Job::default()
        }
    }

    fn after(source: &str, body: &str) -> Job {
        Job {
            phase: Phase::After,
            scripts: vec![script("request:tests", ScriptKind::Tests, source)],
            response: Some(ScriptResponse {
                code: 402,
                reason: "Payment Required".to_string(),
                headers: vec![ScriptRow {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    ..ScriptRow::default()
                }],
                body: body.to_string(),
                millis: 203,
                size: 40,
                cookies: Vec::new(),
            }),
            ..before("")
        }
    }

    /// `QuickJS` is given more stack than a test thread has: each test runs on a thread like the host's.
    fn hosted<T: Send + 'static>(test: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(2 * STACK)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }

    const SECOND: Duration = Duration::from_secs(5);

    fn run(source: &str) -> Outcome {
        engine().0.run(&before(source), SECOND)
    }

    #[test]
    fn a_before_script_changes_the_request_through_req_and_pm_request() {
        hosted(|| {
            let outcome = run("req.method = 'put';
                 req.url = req.url + '?retry=1';
                 req.headers.set('X-Signature', 'abc');
                 pm.request.headers.add({ key: 'X-Trace', value: 1 });
                 pm.request.body.update(JSON.stringify({ amount: 2 }));");
            assert_eq!(outcome.failure, None);
            let request = outcome.request.unwrap();
            assert_eq!(request.method, "PUT");
            assert_eq!(request.url, "{{baseUrl}}/pay?retry=1");
            let keys: Vec<&str> = request.headers.iter().map(|row| row.key.as_str()).collect();
            assert_eq!(keys, ["Accept", "X-Signature", "X-Trace"]);
            assert_eq!(request.headers[2].value, "1");
            assert_eq!(request.body.mode, "raw");
            assert_eq!(request.body.raw, "{\"amount\":2}");
        });
    }

    #[test]
    fn env_writes_the_active_environment_and_with_none_the_globals() {
        hosted(|| {
            let source = "env.set('token', 'abc'); env.secret('key', 42); env.unset('old');
                          pm.globals.set('g', { a: 1 }); pm.collectionVariables.set('c', true);";
            let outcome = run(source);
            let writes: Vec<(WriteScope, &str, Option<&str>, bool)> = outcome
                .writes
                .iter()
                .map(|write| {
                    (
                        write.scope,
                        write.name.as_str(),
                        write.value.as_deref(),
                        write.secret,
                    )
                })
                .collect();
            assert_eq!(
                writes,
                [
                    (WriteScope::Environment, "token", Some("abc"), false),
                    (WriteScope::Environment, "key", Some("42"), true),
                    (WriteScope::Environment, "old", None, false),
                    (WriteScope::Globals, "g", Some("{\"a\":1}"), false),
                    (WriteScope::Collection, "c", Some("true"), false),
                ]
            );

            let mut job = before("env.set('token', 'abc');");
            job.environment = None;
            let outcome = engine().0.run(&job, SECOND);
            assert_eq!(outcome.writes[0].scope, WriteScope::Globals);
        });
    }

    #[test]
    fn a_variable_resolves_from_the_nearest_level_and_what_a_script_set_is_read_back() {
        hosted(|| {
            let outcome = run(
                "if (env.get('baseUrl') !== 'https://api.example.com') throw new Error('env');
                 if (pm.variables.get('tenant') !== 'acme') throw new Error('globals');
                 env.set('baseUrl', 'http://localhost');
                 if (pm.environment.get('baseUrl') !== 'http://localhost') throw new Error('write');
                 const filled = pm.variables.replaceIn('{{baseUrl}}/{{tenant}}/{{missing}}');
                 if (filled !== 'http://localhost/acme/{{missing}}') throw new Error(filled);",
            );
            assert_eq!(outcome.failure, None);
        });
    }

    #[test]
    fn a_local_variable_lasts_from_the_script_before_to_the_tests_after() {
        hosted(|| {
            let (engine, _) = engine();
            engine.run(&before("pm.variables.set('started', 'yes');"), SECOND);
            let outcome = engine.run(
                &after(
                    "test('kept', () => expect(pm.variables.get('started')).toBe('yes'));",
                    "{}",
                ),
                SECOND,
            );
            assert!(outcome.tests[0].passed, "{:?}", outcome.tests);
        });
    }

    #[test]
    fn the_dynamic_variables_fill_a_text_each_time_anew() {
        hosted(|| {
            let outcome = run(
                "const a = pm.variables.replaceIn('{{$guid}}');
                 const b = pm.variables.replaceIn('{{$randomUUID}}');
                 if (!/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(a)) throw new Error(a);
                 if (a === b) throw new Error('same');
                 if (!/^\\d{10}$/.test(pm.variables.replaceIn('{{$timestamp}}'))) throw new Error('timestamp');
                 if (Number.isNaN(Date.parse(pm.variables.replaceIn('{{$isoTimestamp}}')))) throw new Error('iso');
                 const n = Number(pm.variables.replaceIn('{{$randomInt}}'));
                 if (!(n >= 0 && n <= 1000)) throw new Error('int');",
            );
            assert_eq!(outcome.failure, None);
        });
    }

    #[test]
    fn a_test_reports_what_was_expected_what_came_and_the_line_in_both_syntaxes() {
        hosted(|| {
            let source = "test('Le statut est 200 ou 402', () => {
      expect([200, 402]).toContain(res.status);
    });
    pm.test('Le montant est débité en euros', () => {
      pm.expect(res.json().currency).to.equal('eur');
    });
    test('Répond vite', () => expect(res.time).toBeLessThan(100));
    test.skip('Plus tard');";
            let outcome = engine()
                .0
                .run(&after(source, "{\"currency\":\"usd\"}"), SECOND);
            assert_eq!(outcome.failure, None);
            let tests = &outcome.tests;
            assert_eq!(tests.len(), 4);
            assert!(tests[0].passed);
            assert!(!tests[1].passed);
            assert_eq!(tests[1].expected.as_deref(), Some("\"eur\""));
            assert_eq!(tests[1].actual.as_deref(), Some("\"usd\""));
            assert_eq!(tests[1].line, Some(5));
            assert_eq!(tests[1].script.as_deref(), Some("request:tests"));
            assert_eq!(tests[2].expected.as_deref(), Some("100"));
            assert_eq!(tests[2].actual.as_deref(), Some("203"));
            assert!(tests[3].skipped);
        });
    }

    #[test]
    fn the_jest_words_read_like_jest_and_say_not() {
        hosted(|| {
            let source = "
    test('a', () => expect({ a: { b: [1, 2] } }).toEqual({ a: { b: [1, 2] } }));
    test('b', () => expect(res.headers).toHaveProperty('content-type', 'application/json'));
    test('c', () => expect('DevNotes').toMatch(/notes/i));
    test('d', () => expect(null).not.toBeDefined());
    test('e', () => expect(() => { throw new TypeError('no'); }).toThrow(TypeError));
    test('f', () => expect({ a: 1, b: 2 }).toMatchObject({ a: 1 }));
    test('g', () => expect([1, 2, 3]).toHaveLength(3));
    test('h', () => expect(0.1 + 0.2).toBeCloseTo(0.3));
    test('i', () => expect(1).not.toBe(1));";
            let outcome = engine().0.run(&after(source, "{}"), SECOND);
            let passed: Vec<bool> = outcome.tests.iter().map(|test| test.passed).collect();
            assert_eq!(
                passed,
                [true, true, true, false, true, true, true, true, false],
                "{:?}",
                outcome.tests
            );
        });
    }

    #[test]
    fn postman_s_response_assertions_and_older_names_work() {
        hosted(|| {
            let source = "
    pm.test('status', () => pm.response.to.have.status(402));
    pm.test('client error', () => pm.response.to.be.clientError);
    pm.test('header', () => pm.response.to.have.header('content-type', 'application/json'));
    pm.test('json', () => pm.response.to.have.jsonBody('currency', 'usd'));
    pm.test('not ok', () => pm.response.to.be.ok);
    pm.test('plain ok', () => pm.expect(1).to.be.ok);
    tests['legacy'] = responseCode.code === 402;";
            let outcome = engine()
                .0
                .run(&after(source, "{\"currency\":\"usd\"}"), SECOND);
            let passed: Vec<(&str, bool)> = outcome
                .tests
                .iter()
                .map(|test| (test.name.as_str(), test.passed))
                .collect();
            assert_eq!(
                passed,
                [
                    ("status", true),
                    ("client error", true),
                    ("header", true),
                    ("json", true),
                    ("not ok", false),
                    ("plain ok", true),
                    ("legacy", true),
                ]
            );
        });
    }

    #[test]
    fn an_error_names_its_script_and_line_and_stops_the_phase() {
        hosted(|| {
            let mut job = before("const a = 1;\nconst b = 2;\ntokn.read();\nenv.set('never', 1);");
            job.scripts.push(script(
                "collection:before",
                ScriptKind::Before,
                "env.set('later', 1);",
            ));
            let outcome = engine().0.run(&job, SECOND);
            let failure = outcome.failure.unwrap();
            assert_eq!(failure.kind, FailureKind::Error);
            assert_eq!(failure.script.as_deref(), Some("request:before"));
            assert_eq!(failure.line, Some(3));
            assert!(
                failure.message.starts_with("ReferenceError"),
                "{}",
                failure.message
            );
            assert!(outcome.writes.is_empty());
        });
    }

    #[test]
    fn a_script_that_does_not_read_is_a_syntax_failure_on_its_line() {
        hosted(|| {
            let mut job = before("env.set('ran', 1);");
            job.scripts.push(script(
                "folder:before",
                ScriptKind::Before,
                "const ok = 1;\nif (ok {\n}",
            ));
            let outcome = engine().0.run(&job, SECOND);
            let failure = outcome.failure.unwrap();
            assert_eq!(failure.kind, FailureKind::Syntax);
            assert_eq!(failure.script.as_deref(), Some("folder:before"));
            assert_eq!(failure.line, Some(2));
            assert!(
                outcome.writes.is_empty(),
                "nothing runs when one script does not read"
            );
        });
    }

    #[test]
    fn an_endless_loop_ends_at_the_deadline_as_a_timeout() {
        hosted(|| {
            let started = Instant::now();
            let outcome = engine()
                .0
                .run(&before("while (true) {}"), Duration::from_millis(300));
            assert_eq!(outcome.failure.unwrap().kind, FailureKind::Timeout);
            assert!(started.elapsed() < Duration::from_secs(3));
        });
    }

    #[test]
    fn a_script_that_takes_all_memory_ends_as_a_memory_failure() {
        hosted(|| {
            let outcome = run("const kept = []; while (true) kept.push(new Array(1e6).fill(1));");
            assert_eq!(outcome.failure.unwrap().kind, FailureKind::Memory);
        });
    }

    #[test]
    fn a_wait_nothing_ends_is_said_rather_than_waited_for() {
        hosted(|| {
            let outcome = run("await new Promise(() => {});");
            assert_eq!(outcome.failure.unwrap().kind, FailureKind::Timeout);
        });
    }

    #[test]
    fn timers_run_before_the_phase_ends_and_in_their_order() {
        hosted(|| {
            let outcome = run("const order = [];
                 setTimeout(() => { order.push('late'); env.set('order', order.join(',')); }, 40);
                 setTimeout(() => order.push('soon'), 10);
                 const id = setInterval(() => { order.push('tick'); clearInterval(id); }, 5);
                 await new Promise((resolve) => setTimeout(resolve, 1));");
            assert_eq!(outcome.failure, None);
            assert_eq!(outcome.writes[0].value.as_deref(), Some("tick,soon,late"));
        });
    }

    #[test]
    fn there_is_no_file_system_no_process_and_no_module_beyond_the_list() {
        hosted(|| {
            let outcome = run(
                "for (const name of ['fs', 'child_process', 'net', 'os', 'http']) {
                   try { require(name); throw new Error(name + ' was found'); }
                   catch (error) { if (error.code !== 'MODULE_NOT_FOUND') throw error; }
                 }
                 if (typeof std !== 'undefined' || typeof os !== 'undefined') throw new Error('quickjs-libc');
                 if (Object.keys(process.env).length !== 0) throw new Error('env');",
            );
            assert_eq!(outcome.failure, None);
        });
    }

    #[test]
    fn every_library_a_postman_script_requires_loads_and_works() {
        hosted(|| {
            let outcome = run(r#"
    const _ = require('lodash');
    if (_.chunk([1, 2, 3], 2).length !== 2) throw new Error('lodash');
    if (require('moment')('2026-10-19').format('DD/MM/YYYY') !== '19/10/2026') throw new Error('moment');
    require('chai').expect(1).to.equal(1);
    const CryptoJS = require('crypto-js');
    if (CryptoJS.SHA256('abc').toString() !== 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad') throw new Error('crypto-js');
    if (!/^[0-9a-f-]{36}$/.test(require('uuid').v4())) throw new Error('uuid');
    const Ajv = require('ajv');
    if (!new Ajv().validate({ type: 'object', required: ['id'] }, { id: 1 })) throw new Error('ajv');
    if (!require('tv4').validate({ id: 1 }, { type: 'object' })) throw new Error('tv4');
    let parsed;
    require('xml2js').parseString('<a><b>1</b></a>', { async: false }, (error, result) => { parsed = result; });
    if (parsed.a.b[0] !== '1') throw new Error('xml2js');
    if (require('cheerio').load('<p class="x">Bonjour</p>')('.x').text() !== 'Bonjour') throw new Error('cheerio');
    if (require('csv-parse/lib/sync')('a,b\n1,2', { columns: true })[0].b !== '2') throw new Error('csv-parse');
    if (require('csv-parse/sync').parse('a,b\n1,2')[1][0] !== '1') throw new Error('csv-parse/sync');
    if (Buffer.from('DevNotes').toString('base64') !== 'RGV2Tm90ZXM=') throw new Error('buffer');
    if (atob(btoa('é')) !== 'é') throw new Error('base64');
    const events = new (require('events'))(); let heard = false; events.on('x', () => { heard = true; }); events.emit('x');
    if (!heard) throw new Error('events');
    if (require('path').join('/a', 'b', '../c') !== '/a/c') throw new Error('path');
    if (require('querystring').stringify({ a: 1, b: 'x y' }) !== 'a=1&b=x%20y') throw new Error('querystring');
    if (require('url').parse('https://example.com:8080/p?q=1').port !== '8080') throw new Error('url');
    if (require('util').format('%s=%d', 'a', 1) !== 'a=1') throw new Error('util');
    require('assert').deepStrictEqual({ a: [1] }, { a: [1] });
    if (require('punycode').toASCII('mañana.com') !== 'xn--maana-pta.com') throw new Error('punycode');
    if (new (require('string_decoder').StringDecoder)('utf8').write(Buffer.from('é')) !== 'é') throw new Error('string_decoder');
    if (typeof require('stream').Readable !== 'function') throw new Error('stream');
    if (typeof require('timers').setTimeout !== 'function') throw new Error('timers');
    if (typeof _ !== 'function' || typeof CryptoJS.MD5 !== 'function') throw new Error('globals');
    "#);
            assert_eq!(outcome.failure, None);
        });
    }

    #[test]
    fn send_request_goes_to_the_host_and_answers_by_callback_or_promise() {
        hosted(|| {
            let (engine, seen) = engine();
            let outcome = engine.run(
                &before(
                    "pm.sendRequest('https://example.com/a', (error, response) => {
                       if (error) throw error;
                       env.set('first', response.json().url);
                     });
                     const second = await pm.sendRequest({
                       url: 'https://example.com/b', method: 'POST',
                       header: { 'Content-Type': 'application/json' },
                       body: { mode: 'raw', raw: '{}' },
                     });
                     env.set('second', second.code);
                     try { await pm.sendRequest({ url: 'https://example.com/c', method: 'DELETE' }); }
                     catch (error) { env.set('refused', error.code); }",
                ),
                SECOND,
            );
            assert_eq!(outcome.failure, None);
            let values: Vec<Option<&str>> = outcome
                .writes
                .iter()
                .map(|write| write.value.as_deref())
                .collect();
            assert_eq!(
                values,
                [
                    Some("https://example.com/a"),
                    Some("200"),
                    Some("httpScriptProtected")
                ]
            );
            let calls = &seen.borrow().calls;
            assert_eq!(calls.len(), 3);
            assert_eq!(calls[0].1["method"], "GET");
            assert_eq!(calls[1].1["headers"][0]["key"], "Content-Type");
            assert_eq!(calls[1].1["body"]["raw"], "{}");
        });
    }

    #[test]
    fn console_logs_reach_the_host_with_their_script() {
        hosted(|| {
            let (engine, seen) = engine();
            engine.run(
                &before("console.log('signé', { a: 1 }, 2); console.error(new TypeError('x'));"),
                SECOND,
            );
            let logs = &seen.borrow().logs;
            assert_eq!(logs[0].text, "signé {\"a\":1} 2");
            assert_eq!(logs[0].script.as_deref(), Some("request:before"));
            assert_eq!(logs[1].text, "TypeError: x");
        });
    }

    #[test]
    fn a_script_can_skip_the_request_or_name_the_next_one() {
        hosted(|| {
            let outcome = run("pm.execution.skipRequest(); pm.execution.setNextRequest('Reçu');");
            assert!(outcome.skip);
            assert!(outcome.next.set);
            assert_eq!(outcome.next.name.as_deref(), Some("Reçu"));
            assert!(!run("").next.set);
        });
    }

    #[test]
    fn a_stack_names_the_script_and_the_line() {
        assert_eq!(place_in("    at f (script-2:12:5)\n"), Some((2, 12)));
        assert_eq!(
            place_in("    at prelude:40\n    at <eval> (script-0:3)\n"),
            Some((0, 3))
        );
        assert_eq!(place_in("    at prelude:40\n"), None);
    }
}
