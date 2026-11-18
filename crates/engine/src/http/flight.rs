//! The course of one send and the scripts around it, from the command's side: what it reads
//! under the lock before it leaves, its phases, and what it keeps once it ends — however it ends.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;

use super::scripts::around::{self, Around};
use super::scripts::model::{
    Extracted, FailureKind, Job, LayerScope, LogEntry, Outcome, Phase, RequestFault, Script,
    ScriptError, ScriptFailure, ScriptInfo, ScriptLayer, ScriptRequest, ScriptStop, VariableWrite,
    WriteScope,
};
use super::scripts::network::{self, Network};
use super::scripts::process::ScriptProcess;
use super::scripts::session::Session;
use super::{
    console, cookies, environments, extract, history, inherited_of, layers_for, model, runner,
    send, settings, store, transport, variables,
};
use crate::db::{self, lock, lock_library};
use crate::error::AppError;
use crate::{Engine, EngineEvent};

/// What a send reads under the lock before it leaves.
struct Prepared {
    inherited: settings::Inherited,
    layers: Vec<variables::VariableLayer>,
    environment: Option<environments::Environment>,
    jar: Option<Vec<cookies::JarCookie>>,
    /// The number of the library read from: the writes go to it or nowhere.
    library: u64,
}

/// One send and the scripts around it, from the command's side: their host, the jar's copy
/// they change, what they wrote, and what is kept once it ends — however it ends.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct Flight {
    engine: Arc<Engine>,
    id: String,
    /// The library the send read from (`Prepared::library`); 0 until it has read.
    library: u64,
    session: Option<Session>,
    /// The jar's copy while no session holds it.
    jar: Option<Vec<cookies::JarCookie>>,
    transport: transport::Transport,
    environment: Option<environments::Environment>,
    /// The user said yes to what the protected environment asks.
    confirmed: bool,
    /// The files a script asked for that the user said yes to.
    files: Vec<String>,
    collection_id: Option<String>,
    pub(super) info: ScriptInfo,
    /// What each variable written says wrote it: this request.
    by: environments::FilledBy,
    /// What the last phase logged and sent, for the console.
    heard: (Vec<LogEntry>, Vec<network::ScriptSend>),
    /// `false` in a run: what was sent is the report's, not the history's.
    pub(super) record: bool,
    /// `false` in a run that keeps nothing: what the scripts write lives in `overlay`, for the
    /// run alone.
    pub(super) persist: bool,
    pub(super) overlay: Vec<VariableWrite>,
    /// A run's data file: this iteration's row, `pm.iterationData` and the nearest level.
    pub(super) data: BTreeMap<String, String>,
    /// `pm.execution.setNextRequest`, in a run.
    pub(super) next: Option<runner::Next>,
    /// What the scripts and the send did to the jar, in their order.
    changes: Vec<cookies::Change>,
    wrote: bool,
    sent: Option<history::Sent>,
}

/// The request as the scripts leave it: what it sends, and the levels it resolves from.
struct Going {
    kind: model::RequestKind,
    method: model::HttpMethod,
    document: model::RequestDocument,
    layers: Vec<variables::VariableLayer>,
}

impl Flight {
    pub(super) fn new(engine: Arc<Engine>, id: &str) -> Self {
        Self {
            engine,
            id: id.to_string(),
            library: 0,
            session: None,
            jar: None,
            transport: transport::Transport::default(),
            environment: None,
            confirmed: false,
            files: Vec::new(),
            collection_id: None,
            info: ScriptInfo {
                iteration_count: 1,
                ..ScriptInfo::default()
            },
            by: environments::FilledBy::default(),
            heard: (Vec::new(), Vec::new()),
            record: true,
            persist: true,
            overlay: Vec::new(),
            data: BTreeMap::new(),
            next: None,
            changes: Vec::new(),
            wrote: false,
            sent: None,
        }
    }

    pub(super) async fn send(
        &mut self,
        request: send::SendRequest,
    ) -> Result<send::SentResponse, AppError> {
        let send::SendRequest {
            id,
            kind,
            method,
            document,
            collection_id,
            folder_id,
            name,
            request_id,
            environment_id,
            confirmed,
            files,
        } = request;
        let Prepared {
            inherited,
            mut layers,
            environment,
            jar,
            library,
        } = prepare(
            &self.engine,
            &document,
            collection_id.clone(),
            folder_id,
            environment_id,
        )
        .await?;
        let around = Around::of(&document.scripts, &name, &inherited);
        self.jar = jar;
        self.library = library;
        self.transport = inherited.transport.over(&document.transport);
        self.environment = environment;
        self.confirmed = confirmed;
        self.files = files;
        self.collection_id = collection_id;
        self.named(&name, request_id.as_deref());
        self.layered(&mut layers);
        // What the request reads as the user wrote it: a file the scripts add waits for a yes.
        let own = (!around.before.is_empty()).then(|| files_read(&document, &inherited, &layers));
        let mut going = Going {
            kind,
            method,
            document,
            layers,
        };
        if !around.before.is_empty() {
            self.before(&around.before, &mut going).await?;
        }

        let resolver = variables::Resolver::new(&going.layers);
        let (filled, above) = variables::resolve(&going.document, &inherited, &resolver);
        if let Some(own) = &own
            && let Some(path) = filled.body.files().into_iter().find(|path| {
                !own.iter().any(|read| read == path) && !self.files.iter().any(|yes| yes == path)
            })
        {
            return Err(ScriptError::File(path.to_owned()).into());
        }
        let method = going.method;
        let mut outgoing = self
            .engine
            .runtime
            .spawn_blocking(move || send::compose(kind, method, &filled, &above))
            .await
            .map_err(|error| AppError::from(send::SendError::Network(error.to_string())))??;
        outgoing.secret_values = resolver.secrets();
        if outgoing.transport.use_cookies {
            outgoing.jar = self.jar();
        }
        let exchange = send::exchange(&outgoing);
        let secrets = outgoing.secrets.clone();
        let secret_values = outgoing.secret_values.clone();
        self.say(Utc::now(), console::out(&exchange, &secret_values, None));
        let mut answer = self.leave(&id, outgoing).await?;

        let mut finished = true;
        if let Ok(response) = &mut answer {
            finished = self.answered(&around, &mut going, response).await;
        }
        self.sent = self.record.then(|| history::Sent {
            kind,
            name,
            request_id,
            exchange,
            secrets,
            secret_values,
            environment: self.sent_environment(),
            outcome: match &answer {
                Ok(response) => Ok(response.clone()),
                Err(error) => Err((error.code, error.params.clone())),
            },
        });
        if !finished {
            return Err(send::SendError::Cancelled.into());
        }
        answer
    }

    fn named(&mut self, name: &str, request_id: Option<&str>) {
        self.info.request_name = name.to_string();
        self.info.request_id = request_id.map(str::to_string);
        self.by = environments::FilledBy {
            request_id: request_id.map(str::to_string),
            name: name.to_string(),
        };
    }

    /// What a run lays over the levels read: what its scripts wrote, kept for it alone, and
    /// this iteration's row, the nearest of all.
    fn layered(&self, layers: &mut Vec<variables::VariableLayer>) {
        around::apply(layers, &self.overlay);
        if !self.data.is_empty() {
            layers.insert(0, around::locals_layer(&self.data));
        }
    }

    fn sent_environment(&self) -> Option<history::SentEnvironment> {
        self.environment
            .as_ref()
            .map(|environment| history::SentEnvironment {
                name: environment.name.clone(),
                colour: environment.colour,
            })
    }

    /// The request sent, the lock free; `Err` alone when « Annuler » stopped it, a failure to
    /// send being the answer the history keeps.
    async fn leave(
        &mut self,
        id: &str,
        outgoing: send::Outgoing,
    ) -> Result<Result<send::SentResponse, AppError>, AppError> {
        let (answer, cookies_set) = self
            .engine
            .sending
            .run(&self.engine.runtime, id, outgoing)
            .await;
        if matches!(answer, Err(send::SendError::Cancelled)) {
            return Err(send::SendError::Cancelled.into());
        }
        self.received(cookies_set);
        let answer = answer.map_err(AppError::from);
        self.say(
            Utc::now(),
            match &answer {
                Ok(response) => console::answered(response, None),
                Err(error) => console::ConsoleLine::Failed {
                    code: error.code,
                    params: error.params.clone(),
                    by: None,
                },
            },
        );
        Ok(answer)
    }

    /// What follows an answer: the scripts after it and the tests, then the extractions; and what
    /// the answer carries of them. `false` when « Annuler » stopped the scripts.
    async fn answered(
        &mut self,
        around: &Around,
        going: &mut Going,
        response: &mut send::SentResponse,
    ) -> bool {
        let answered = around.answered();
        let outcome = if answered.is_empty() {
            Some(Outcome::default())
        } else {
            self.after(&answered, going, response).await
        };
        let finished = outcome.is_some();
        if let Some(outcome) = outcome {
            let extractions = std::mem::take(&mut going.document.extractions);
            response.scripts.extracted = self
                .extract(&extractions, response, &mut going.layers)
                .await;
            // Said once the extractions ran: a value one makes a secret is hidden in the logs.
            self.report(&answered, &outcome, &going.layers);
            self.follow(&outcome);
            for missed in response
                .scripts
                .extracted
                .iter()
                .filter(|extracted| !extracted.found)
            {
                self.say(
                    Utc::now(),
                    console::ConsoleLine::Missed {
                        path: missed.path.clone(),
                        variable: missed.variable.clone(),
                    },
                );
            }
        }
        let shown = variables::Resolver::new(&going.layers).secrets();
        let tests = std::mem::take(&mut response.scripts.tests);
        response.scripts.tests = around::shown_tests(tests, &answered, &shown);
        response.scripts.wrote = self.wrote;
        finished
    }

    /// A phase's job, `url` the one its cookies are read for.
    fn job(&self, phase: Phase, scripts: &[Script], going: &Going, url: &str) -> Job {
        Job {
            phase,
            scripts: scripts.to_vec(),
            request: ScriptRequest::of(going.kind, going.method, &going.document),
            response: None,
            layers: self.script_layers(going),
            environment: self
                .environment
                .as_ref()
                .map(|environment| environment.name.clone()),
            cookies: network::cookies_for(self.jar().as_deref(), url),
            info: self.info.clone(),
        }
    }

    fn script_layers(&self, going: &Going) -> Vec<ScriptLayer> {
        let mut layers = around::layers_of(&going.layers);
        if !self.data.is_empty() {
            layers.insert(
                0,
                ScriptLayer {
                    scope: LayerScope::Data,
                    from: String::new(),
                    values: self.data.clone(),
                },
            );
        }
        layers
    }

    /// The scripts before the send: what they change goes out, what stops them keeps it in.
    async fn before(&mut self, scripts: &[Script], going: &mut Going) -> Result<(), AppError> {
        let url = variables::Resolver::new(&going.layers).fill(&going.document.url);
        let job = self.job(Phase::Before, scripts, going, &url);
        let outcome = self.run(job).await?;
        self.write(&outcome.writes, &mut going.layers).await;
        self.report(scripts, &outcome, &going.layers);
        self.follow(&outcome);
        let refused = self
            .session
            .as_mut()
            .and_then(|session| session.network.refused_file.take());
        if let Some(path) = refused {
            return Err(ScriptError::File(path).into());
        }
        if let Some(failure) = &outcome.failure {
            return Err(ScriptError::Stopped(ScriptStop::of(failure, scripts)).into());
        }
        if outcome.skip {
            return Err(ScriptError::Skipped.into());
        }
        if let Some(request) = &outcome.request {
            going.method = request
                .onto(going.kind, &mut going.document)
                .map_err(|RequestFault::Method(method)| ScriptError::Method(method))?;
        }
        // The front asked about the method it saw; the one the script left is checked here.
        if let Some(environment) = &self.environment
            && environment.protected
            && !self.confirmed
            && !going.method.changes_nothing()
        {
            return Err(ScriptError::Protected {
                method: going.method,
                environment: environment.name.clone(),
            }
            .into());
        }
        if !outcome.locals.is_empty() {
            going
                .layers
                .insert(0, around::locals_layer(&outcome.locals));
        }
        Ok(())
    }

    /// The scripts once it is answered, then the tests: what stops them is shown with the answer.
    /// `None` when « Annuler » stopped them.
    async fn after(
        &mut self,
        scripts: &[Script],
        going: &mut Going,
        response: &mut send::SentResponse,
    ) -> Option<Outcome> {
        let job = Job {
            response: Some(network::response_of(response)),
            ..self.job(Phase::After, scripts, going, &response.url)
        };
        let outcome = self.run(job).await.ok()?;
        self.write(&outcome.writes, &mut going.layers).await;
        response.scripts.tests.clone_from(&outcome.tests);
        response.scripts.failure = outcome
            .failure
            .as_ref()
            .map(|failure| ScriptStop::of(failure, scripts));
        Some(outcome)
    }

    /// One phase in the host, started for the first one; `Err` when « Annuler » stopped it.
    async fn run(&mut self, job: Job) -> Result<Outcome, AppError> {
        let mut session = match self.session.take() {
            Some(session) => session,
            None => match ScriptProcess::start() {
                Ok(host) => {
                    let mut network = Network::new(
                        tokio::runtime::Handle::current(),
                        self.transport,
                        self.jar.take(),
                        self.confirmed
                            || !self
                                .environment
                                .as_ref()
                                .is_some_and(|environment| environment.protected),
                    );
                    network.files.clone_from(&self.files);
                    Session::new(host, network)
                }
                Err(error) => return Ok(host_failed(&error.to_string())),
            },
        };
        let killer = session.killer();
        self.engine.sending.watch(&self.id, Some(killer.clone()));
        let ran = self
            .engine
            .runtime
            .spawn_blocking(move || {
                let outcome = session.run(&job);
                (session, outcome)
            })
            .await;
        self.engine.sending.watch(&self.id, None);
        let outcome = match ran {
            Ok((mut session, outcome)) => {
                self.changes.append(&mut session.network.changes);
                self.heard = (
                    std::mem::take(&mut session.logs),
                    std::mem::take(&mut session.network.sent),
                );
                self.session = Some(session);
                outcome
            }
            Err(error) => host_failed(&error.to_string()),
        };
        if killer.killed() {
            return Err(send::SendError::Cancelled.into());
        }
        Ok(outcome)
    }

    /// The request's extractions, after its scripts: what each path reads is written into the
    /// active environment, the globals with « Aucun »; a path that leads nowhere leaves its
    /// variable as it was.
    async fn extract(
        &mut self,
        extractions: &[model::Extraction],
        response: &send::SentResponse,
        layers: &mut [variables::VariableLayer],
    ) -> Vec<Extracted> {
        let scope = if self.environment.is_some() {
            WriteScope::Environment
        } else {
            WriteScope::Globals
        };
        let mut writes = Vec::new();
        let extracted = extractions
            .iter()
            .filter(|extraction| !extraction.variable.trim().is_empty())
            .map(|extraction| {
                let value = extract::extract(&response.body, &extraction.path);
                let found = value.is_some();
                if let Some(value) = value {
                    writes.push(VariableWrite {
                        scope,
                        name: extraction.variable.trim().to_string(),
                        value: Some(value),
                        secret: extraction.secret,
                    });
                }
                Extracted {
                    path: extraction.path.clone(),
                    variable: extraction.variable.trim().to_string(),
                    found,
                }
            })
            .collect();
        self.write(&writes, layers).await;
        extracted
    }

    /// The jar as the scripts left it.
    fn jar(&self) -> Option<Vec<cookies::JarCookie>> {
        match &self.session {
            Some(session) => session.network.jar.clone(),
            None => self.jar.clone(),
        }
    }

    /// What the send did to the jar, for the scripts after it to read.
    fn received(&mut self, changes: Vec<cookies::Change>) {
        if let Some(jar) = self
            .session
            .as_mut()
            .and_then(|session| session.network.jar.as_mut())
        {
            cookies::apply(jar, &changes);
        }
        self.changes.extend(changes);
    }

    /// `pm.execution.setNextRequest` in a phase: what a run goes to next.
    fn follow(&mut self, outcome: &Outcome) {
        if outcome.next.set {
            self.next = Some(match &outcome.next.name {
                Some(name) => runner::Next::To(name.clone()),
                None => runner::Next::End,
            });
        }
    }

    /// A line of this send's console, at once: the front shows it as it comes.
    fn say(&self, at: chrono::DateTime<Utc>, line: console::ConsoleLine) {
        let event = console::ConsoleEvent {
            id: self.id.clone(),
            at,
            request: self.info.request_name.clone(),
            iteration: (self.info.iteration_count > 1).then_some(self.info.iteration + 1),
            line,
        };
        self.engine.emit(EngineEvent::Console(event));
    }

    /// What the last phase logged and sent, the tests that failed and what stopped it, each
    /// secret in force — those the phase wrote included — written as its name.
    fn report(
        &mut self,
        scripts: &[Script],
        outcome: &Outcome,
        layers: &[variables::VariableLayer],
    ) {
        let (logs, sends) = std::mem::take(&mut self.heard);
        let secrets = variables::Resolver::new(layers).secrets();
        for (at, line) in console::phase(logs, sends, outcome, scripts, &secrets) {
            self.say(at, line);
        }
    }

    /// What a phase wrote: kept under the lock, and in the levels the send resolves from.
    async fn write(&mut self, writes: &[VariableWrite], layers: &mut [variables::VariableLayer]) {
        if writes.is_empty() {
            return;
        }
        around::apply(layers, writes);
        if !self.persist {
            self.overlay.extend_from_slice(writes);
            return;
        }
        let writes = writes.to_vec();
        let environment_id = self
            .environment
            .as_ref()
            .map(|environment| environment.id.clone());
        let collection_id = self.collection_id.clone();
        let by = self.by.clone();
        let library = self.library;
        let kept = db::run(Arc::clone(&self.engine), move |engine| {
            let db = &engine.db;
            let Some(mut connection) = lock_library(db, library)? else {
                log::warn!("A script wrote into a library closed since: nothing kept");
                return Ok(false);
            };
            Ok(keep_writes(
                &mut connection,
                &writes,
                environment_id.as_deref(),
                collection_id.as_deref(),
                &by,
            )?)
        })
        .await;
        match kept {
            Ok(wrote) => self.wrote |= wrote,
            Err(error) => {
                log::warn!("What a script wrote was not kept: {}", error.for_log());
                self.engine.emit(EngineEvent::SendNotKept);
            }
        }
    }

    /// The history and the jar, written under the lock again, and never at the answer's expense.
    pub(super) async fn keep(self) {
        let Self {
            engine,
            library,
            sent,
            changes,
            ..
        } = self;
        if sent.is_none() && changes.is_empty() {
            return;
        }
        let kept = db::run(Arc::clone(&engine), move |engine| {
            let db = &engine.db;
            let now = Utc::now();
            let Some(mut connection) = lock_library(db, library)? else {
                log::warn!("A send ended after its library was closed: its history and cookies are not kept");
                return Ok(());
            };
            if let Some(sent) = &sent {
                let (summary, record) = history::record_of(sent);
                history::store::record(
                    &mut connection,
                    sent.request_id.as_deref(),
                    &summary,
                    &record,
                    now,
                )?;
            }
            cookies::store::apply(&mut connection, &changes, now)?;
            Ok(())
        })
        .await;
        if let Err(error) = kept {
            log::warn!(
                "A send's history or cookies were not kept: {}",
                error.for_log()
            );
            engine.emit(EngineEvent::SendNotKept);
        }
    }
}

/// The files a request reads as its own rows say, its variables resolved before any script.
fn files_read(
    document: &model::RequestDocument,
    inherited: &settings::Inherited,
    layers: &[variables::VariableLayer],
) -> Vec<String> {
    let (filled, _) = variables::resolve(document, inherited, &variables::Resolver::new(layers));
    filled.body.files().into_iter().map(str::to_owned).collect()
}

fn host_failed(message: &str) -> Outcome {
    Outcome::failed(ScriptFailure {
        kind: FailureKind::Host,
        message: message.to_string(),
        ..ScriptFailure::default()
    })
}

async fn prepare(
    engine: &Arc<Engine>,
    document: &model::RequestDocument,
    collection_id: Option<String>,
    folder_id: Option<String>,
    environment_id: Option<String>,
) -> Result<Prepared, AppError> {
    let own = document.variables.clone();
    let own_transport = document.transport;
    db::run(Arc::clone(engine), move |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        let inherited = inherited_of(
            &mut connection,
            collection_id.as_deref(),
            folder_id.as_deref(),
        )?;
        let layers = layers_for(&mut connection, &own, &inherited, environment_id.as_deref())?;
        let environment = environment_id
            .as_deref()
            .map(|id| environments::store::get(&mut connection, id))
            .transpose()
            .ok()
            .flatten();
        let jar = if inherited.transport.over(&own_transport).use_cookies {
            Some(cookies::store::jar(&mut connection, Utc::now())?)
        } else {
            None
        };
        Ok(Prepared {
            inherited,
            layers,
            environment,
            jar,
            library: connection.number(),
        })
    })
    .await
}

/// What a phase wrote into the tables that hold it: the active environment, the globals, the
/// collection's variables. A run's data row is none of them.
fn keep_writes(
    connection: &mut crate::db::Library,
    writes: &[VariableWrite],
    environment_id: Option<&str>,
    collection_id: Option<&str>,
    by: &environments::FilledBy,
) -> Result<bool, crate::error::StorageError> {
    let wrote_to = |scope| around::writes_to(writes, scope).next().is_some();
    let mut wrote = false;
    if let Some(id) = environment_id
        && wrote_to(WriteScope::Environment)
    {
        let environment = environments::store::get(connection, id)?;
        let patch = environments::EnvironmentPatch {
            variables: Some(around::written(
                &environment.variables,
                around::writes_to(writes, WriteScope::Environment),
                Some(by),
            )),
            ..environments::EnvironmentPatch::default()
        };
        environments::store::save(connection, id, &patch, None)?;
        wrote = true;
    }
    if wrote_to(WriteScope::Globals) {
        let globals = environments::store::list(connection)?.globals;
        let globals = around::written(
            &globals,
            around::writes_to(writes, WriteScope::Globals),
            Some(by),
        );
        environments::store::save_globals(connection, &globals, Utc::now())?;
        wrote = true;
    }
    if let Some(id) = collection_id
        && wrote_to(WriteScope::Collection)
    {
        let item = model::HttpItem {
            kind: model::HttpItemKind::Collection,
            id: id.to_string(),
        };
        let mut held = store::settings(connection, &item)?;
        held.variables = around::written(
            &held.variables,
            around::writes_to(writes, WriteScope::Collection),
            Some(by),
        );
        store::save_settings(connection, &item, &held)?;
        wrote = true;
    }
    Ok(wrote)
}
