#![allow(clippy::needless_pass_by_value)]

//! The HTTP client's collections: they belong to the library, not to a space, and are sealed
//! like the notes. A collection travels in a file of its own, never in the library export.

pub mod codegen;
pub mod console;
pub mod cookies;
pub mod environments;
pub mod extract;
pub mod graphql;
pub mod history;
pub mod model;
pub mod query;
pub mod response;
pub mod runner;
pub mod scripts;
pub mod search;
pub mod send;
pub mod settings;
pub mod store;
pub mod transport;
pub mod variables;
pub mod websocket;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use crate::Engine;
use crate::EngineEvent;
use crate::db::{self, lock, lock_library};
use crate::error::{AppError, FileContext};
use crate::tools::off_thread;
use model::{
    HttpCollection, HttpContents, HttpFolder, HttpItem, HttpPlace, HttpRequest, HttpRequestDraft,
    HttpRequestPatch, HttpTree, NewPlace,
};
use scripts::around::{self, Around};
use scripts::model::{
    Extracted, FailureKind, Job, LayerScope, LogEntry, Outcome, Phase, RequestFault, Script,
    ScriptError, ScriptFailure, ScriptInfo, ScriptLayer, ScriptRequest, ScriptStop, VariableWrite,
    WriteScope,
};
use scripts::network::{self, Network};
use scripts::process::ScriptProcess;
use scripts::session::Session;

pub fn http_tree(engine: &Engine) -> Result<HttpTree, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::tree(&mut connection)?)
}

/// The palette's requests, ranked here: the URL is read from each sealed document.
pub fn search_http_requests(
    engine: &Engine,
    search: String,
) -> Result<Vec<search::RequestHit>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(search::rank(
        store::search_candidates(&mut connection)?,
        &search,
    ))
}

pub fn create_http_collection(engine: &Engine, name: String) -> Result<HttpCollection, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::create_collection(
        &mut connection,
        &name,
        Utc::now(),
    )?)
}

/// `parent_id` `None` puts it at the collection's root.
pub fn create_http_folder(
    engine: &Engine,
    collection_id: String,
    parent_id: Option<String>,
    name: String,
) -> Result<HttpFolder, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::create_folder(
        &mut connection,
        &collection_id,
        parent_id.as_deref(),
        &name,
        Utc::now(),
    )?)
}

pub fn create_http_request(
    engine: &Engine,
    draft: HttpRequestDraft,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&draft.name)?;

    let mut connection = lock(db)?;

    Ok(store::create_request(
        &mut connection,
        &draft,
        &name,
        Utc::now(),
    )?)
}

/// The first save of a request where nothing holds it yet: what `create` names is created
/// with it, or nothing is. Every name is checked before anything is written.
pub fn create_http_request_in(
    engine: &Engine,
    draft: HttpRequestDraft,
    create: NewPlace,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&draft.name)?;
    let collection = create
        .collection
        .as_deref()
        .map(model::validated_name)
        .transpose()?;
    let folder = create
        .folder
        .as_deref()
        .map(model::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(store::create_request_in(
        &mut connection,
        &draft,
        &name,
        collection.as_deref(),
        folder.as_deref(),
        Utc::now(),
    )?)
}

pub fn get_http_request(engine: &Engine, id: String) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::get_request(&mut connection, &id)?)
}

pub fn save_http_request(
    engine: &Engine,
    id: String,
    patch: HttpRequestPatch,
) -> Result<HttpRequest, AppError> {
    let db = &engine.db;
    let name = patch
        .name
        .as_deref()
        .map(model::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(store::save_request(
        &mut connection,
        &id,
        &patch,
        name.as_deref(),
        Utc::now(),
    )?)
}

pub fn rename_http_item(engine: &Engine, item: HttpItem, name: String) -> Result<(), AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::rename(&mut connection, &item, &name)?)
}

/// What a deletion would take with it, for the confirmation that comes first.
pub fn count_http_contents(engine: &Engine, item: HttpItem) -> Result<HttpContents, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::contents(&mut connection, &item)?)
}

pub fn delete_http_item(engine: &Engine, item: HttpItem) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::delete(&mut connection, &item)?)
}

/// The copy's name comes from the front: no user-visible word is Rust's.
pub fn duplicate_http_item(
    engine: &Engine,
    item: HttpItem,
    name: String,
) -> Result<HttpItem, AppError> {
    let db = &engine.db;
    let name = model::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(store::duplicate(&mut connection, &item, &name, Utc::now())?)
}

pub fn move_http_item(engine: &Engine, item: HttpItem, place: HttpPlace) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::move_item(&mut connection, &item, &place)?)
}

pub fn http_settings(
    engine: &Engine,
    item: HttpItem,
) -> Result<model::ContainerSettings, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::settings(&mut connection, &item)?)
}

pub fn save_http_settings(
    engine: &Engine,
    item: HttpItem,
    settings: model::ContainerSettings,
) -> Result<(), AppError> {
    // Refused where it is typed: stored, it failed every request under it, one by one.
    if let Some((row, fault)) = settings
        .headers
        .iter()
        .zip(model::header_faults(&settings.headers))
        .find_map(|(row, fault)| Some((row, fault?)))
    {
        return Err(send::SendError::Header {
            name: row.key.trim().to_string(),
            fault,
            set_by: transport::SetBy::Request,
        }
        .into());
    }
    let db = &engine.db;

    let mut connection = lock(db)?;
    // The dialog does not hold the run order: the one the runner left is kept.
    let settings = model::ContainerSettings {
        run_order: store::settings(&mut connection, &item)?.run_order,
        ..settings
    };

    Ok(store::save_settings(&mut connection, &item, &settings)?)
}

/// What a request placed there inherits, and from where: « héritée de la collection … ».
pub fn inherited_http_settings(
    engine: &Engine,
    collection_id: String,
    folder_id: Option<String>,
) -> Result<settings::Inherited, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::inherited(
        &mut connection,
        &collection_id,
        folder_id.as_deref(),
    )?)
}

/// What a body implies, whether a JSON one reads, and whether its method seldom carries one:
/// the editor says the three as it is typed, and decides none of them.
pub async fn describe_http_body(
    body: model::RequestBody,
    method: model::HttpMethod,
) -> Result<BodyAnswer, AppError> {
    off_thread(move || BodyAnswer {
        unusual: body.is_unusual_on(method),
        content_type: settings::implied_content_type(&body).map(str::to_string),
        problem: match &body {
            model::RequestBody::Json { text } if !text.trim().is_empty() => {
                crate::json::parse::parse(text).err()
            }
            _ => None,
        },
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyAnswer {
    pub content_type: Option<String>,
    pub problem: Option<crate::json::parse::JsonError>,
    /// Something is sent as the body of a `GET` or a `HEAD`: sent all the same, and said.
    pub unusual: bool,
}

/// Each header's fault by its rank, or none: the table marks the row and says why. The rule is
/// the model's; the table holds none.
pub async fn check_http_headers(
    rows: Vec<model::KeyValue>,
) -> Result<Vec<Option<model::HeaderFault>>, AppError> {
    off_thread(move || model::header_faults(&rows)).await
}

/// `Nom: valeur` typed in a name, or lines pasted from a browser's network tab, as rows.
pub async fn read_http_headers(text: String) -> Result<Vec<model::KeyValue>, AppError> {
    off_thread(move || model::headers_from_text(&text)).await
}

/// What a request sends without a row of its own — the body's type, what it inherits, the
/// application's `User-Agent` — read as a send reads it: the editor lists the answer and
/// decides none of it.
pub fn implied_http_headers(
    engine: &Engine,
    kind: model::RequestKind,
    document: model::RequestDocument,
    collection_id: Option<String>,
    folder_id: Option<String>,
) -> Result<Vec<send::ImpliedHeader>, AppError> {
    let db = &engine.db;
    let inherited = match collection_id {
        Some(collection_id) => {
            let mut connection = lock(db)?;
            store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
        }
        None => settings::inherit(&[]),
    };
    Ok(send::implied(kind, &document, &inherited))
}

/// The edited side wins: the query string rewritten from the table, or the table from the URL.
pub async fn sync_http_query(
    url: String,
    params: Vec<model::KeyValue>,
    edited: query::QuerySide,
) -> Result<query::SyncedQuery, AppError> {
    off_thread(move || query::sync(&url, &params, edited)).await
}

pub fn reorder_http_collection(engine: &Engine, id: String, index: u32) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::reorder_collection(&mut connection, &id, index)?)
}

/// What it inherits is read under the lock, its files outside it, and the send waits on nothing
/// but the network and its scripts: a 30-second request stalls no other command.
pub async fn send_http_request(
    engine: Arc<Engine>,
    request: send::SendRequest,
) -> Result<send::SentResponse, AppError> {
    let mut flight = Flight::new(engine, &request.id);
    let answer = flight.send(request).await;
    flight.keep().await;
    answer
}

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
struct Flight {
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
    info: ScriptInfo,
    /// What each variable written says wrote it: this request.
    by: environments::FilledBy,
    /// What the last phase logged and sent, for the console.
    heard: (Vec<LogEntry>, Vec<network::ScriptSend>),
    /// `false` in a run: what was sent is the report's, not the history's.
    record: bool,
    /// `false` in a run that keeps nothing: what the scripts write lives in `overlay`, for the
    /// run alone.
    persist: bool,
    overlay: Vec<VariableWrite>,
    /// A run's data file: this iteration's row, `pm.iterationData` and the nearest level.
    data: BTreeMap<String, String>,
    /// `pm.execution.setNextRequest`, in a run.
    next: Option<runner::Next>,
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
    fn new(engine: Arc<Engine>, id: &str) -> Self {
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

    async fn send(&mut self, request: send::SendRequest) -> Result<send::SentResponse, AppError> {
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
            Err(error) => log::warn!("What a script wrote was not kept: {}", error.detail),
        }
    }

    /// The history and the jar, written under the lock again, and never at the answer's expense.
    async fn keep(self) {
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
        let kept = db::run(engine, move |engine| {
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
                error.detail
            );
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
        let inherited = match collection_id {
            Some(collection_id) => {
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
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

/// What a request resolves its `{{variables}}` from, the nearest first: its own, its folders
/// and collection, the environment chosen — none when it is gone — and the globals.
fn resolver_for(
    connection: &mut crate::db::Library,
    own: &[environments::Variable],
    inherited: &settings::Inherited,
    environment_id: Option<&str>,
) -> Result<variables::Resolver, crate::error::StorageError> {
    Ok(variables::Resolver::new(&layers_for(
        connection,
        own,
        inherited,
        environment_id,
    )?))
}

fn layers_for(
    connection: &mut crate::db::Library,
    own: &[environments::Variable],
    inherited: &settings::Inherited,
    environment_id: Option<&str>,
) -> Result<Vec<variables::VariableLayer>, crate::error::StorageError> {
    use variables::{VariableLayer, VariableScope};

    let sets = environments::store::list(connection)?;
    let mut layers = vec![VariableLayer {
        scope: VariableScope::Request,
        from: String::new(),
        variables: own.to_vec(),
    }];
    layers.extend(inherited.variables.iter().cloned());
    if let Some(environment) = sets
        .environments
        .into_iter()
        .find(|environment| Some(environment.id.as_str()) == environment_id)
    {
        layers.push(VariableLayer {
            scope: VariableScope::Environment,
            from: environment.name,
            variables: environment.variables,
        });
    }
    layers.push(VariableLayer {
        scope: VariableScope::Globals,
        from: String::new(),
        variables: sets.globals,
    });
    Ok(layers)
}

/// The request written as code for another tool, from what a send would write: its variables
/// resolved as asked, a secret left as its `{{name}}` unless it is asked for in clear.
pub fn generate_http_code(
    engine: &Engine,
    request: CodeRequestOf,
) -> Result<codegen::GeneratedCode, AppError> {
    let db = &engine.db;
    let CodeRequestOf {
        target,
        options,
        kind,
        method,
        document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    let mut connection = lock(db)?;
    let inherited = match collection_id {
        Some(collection_id) => {
            store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
        }
        None => settings::inherit(&[]),
    };
    let layers = layers_for(
        &mut connection,
        &document.variables,
        &inherited,
        environment_id.as_deref(),
    )?;
    Ok(codegen::generate(
        target, options, kind, method, &document, &inherited, &layers,
    ))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRequestOf {
    pub target: codegen::CodeTarget,
    pub options: codegen::CodeOptions,
    pub kind: model::RequestKind,
    pub method: model::HttpMethod,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// The `{{variables}}` a request holds, each with what it resolves to and from where: the
/// editor colours them by it, and names the ones nothing sets before a send does.
pub fn http_variables(
    engine: &Engine,
    request: VariablesRequest,
) -> Result<Vec<variables::UsedVariable>, AppError> {
    let db = &engine.db;
    let VariablesRequest {
        kind,
        document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    let mut connection = lock(db)?;
    let inherited = match collection_id {
        Some(collection_id) => {
            store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
        }
        None => settings::inherit(&[]),
    };
    let resolver = resolver_for(
        &mut connection,
        &document.variables,
        &inherited,
        environment_id.as_deref(),
    )?;
    Ok(resolver.used(&variables::texts(kind, &document, &inherited)))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesRequest {
    pub kind: model::RequestKind,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// The order a collection or a folder runs in: the one last arranged, the requests added since
/// at its end.
pub fn http_run_order(engine: &Engine, item: HttpItem) -> Result<Vec<model::RunEntry>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let tree = store::tree(&mut connection)?;
    let saved = store::settings(&mut connection, &item)?.run_order;

    Ok(runner::run_order(
        &saved,
        &runner::placed_under(&tree, &item),
    ))
}

/// The order the runner was left in, kept with the collection's or the folder's settings.
pub fn save_http_run_order(
    engine: &Engine,
    item: HttpItem,
    order: Vec<model::RunEntry>,
) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let settings = model::ContainerSettings {
        run_order: order,
        ..store::settings(&mut connection, &item)?
    };

    Ok(store::save_settings(&mut connection, &item, &settings)?)
}

/// A collection or a folder run, one request after the other, each sent as a send is — its
/// scripts, its tests, its extractions — but written in no history. Each step is said as an
/// event; answers once the run is over. The lock is held for reads alone, never over a send.
pub async fn run_http_collection(
    engine: Arc<Engine>,
    run: runner::RunRequest,
) -> Result<(), AppError> {
    let tree = db::run(Arc::clone(&engine), |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        Ok(store::tree(&mut connection)?)
    })
    .await?;
    let placed = runner::placed_under(&tree, &run.item);
    let order: Vec<runner::Placed> = run
        .order
        .iter()
        .filter_map(|id| placed.iter().find(|known| &known.id == id).cloned())
        .collect();
    let rows = run
        .data
        .as_ref()
        .map(|data| data.rows.clone())
        .unwrap_or_default();
    let iterations = if rows.is_empty() {
        run.iterations.max(1)
    } else {
        u32::try_from(rows.len()).unwrap_or(u32::MAX)
    };
    let runs = &engine.runs;
    runs.start(&run.id);
    let started = std::time::Instant::now();
    let mut overlay = Vec::new();
    let mut cancelled = false;
    'run: for iteration in 0..iterations {
        let row = rows.get(iteration as usize).cloned().unwrap_or_default();
        let mut at = (!order.is_empty()).then_some(0);
        let mut steps = 0_u32;
        while let Some(index) = at {
            if runs.cancelled(&run.id) {
                cancelled = true;
                break 'run;
            }
            steps += 1;
            let placed = &order[index];
            let step = RunStep {
                run: &run,
                iteration,
                iterations,
                index,
                steps,
                placed,
                row: &row,
            };
            let Some((result, next)) = run_step(&engine, step, &mut overlay).await else {
                cancelled = true;
                break 'run;
            };
            let failed = result.failed();
            say_run(
                &engine,
                &run.id,
                runner::RunStep::Ended {
                    result: Box::new(result),
                },
            );
            if run.stop_on_failure && failed {
                break 'run;
            }
            // `setNextRequest(null)` ends the run, as it ends a collection run elsewhere.
            if next == Some(runner::Next::End) {
                break 'run;
            }
            at = runner::next_index(&order, index, next.as_ref());
            if at.is_some() && !wait(runs, &run.id, run.delay_ms).await {
                cancelled = true;
                break 'run;
            }
        }
    }
    runs.finish(&run.id);
    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    say_run(
        &engine,
        &run.id,
        runner::RunStep::Finished { cancelled, millis },
    );
    Ok(())
}

/// One request of one iteration of a run.
struct RunStep<'a> {
    run: &'a runner::RunRequest,
    iteration: u32,
    iterations: u32,
    index: usize,
    steps: u32,
    placed: &'a runner::Placed,
    row: &'a BTreeMap<String, String>,
}

/// A request of a run, sent; `None` when « Annuler » stopped it. Beside its result, where a
/// script said to go next.
async fn run_step(
    engine: &Arc<Engine>,
    step: RunStep<'_>,
    overlay: &mut Vec<VariableWrite>,
) -> Option<(runner::RunResult, Option<runner::Next>)> {
    let RunStep {
        run,
        iteration,
        iterations,
        index,
        steps,
        placed,
        row,
    } = step;
    say_run(
        engine,
        &run.id,
        runner::RunStep::Started {
            iteration,
            index: u32::try_from(index).unwrap_or(u32::MAX),
            name: placed.name.clone(),
        },
    );
    let id = placed.id.clone();
    let request = db::run(Arc::clone(engine), move |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        Ok(store::get_request(&mut connection, &id)?)
    })
    .await;
    let request = match request {
        Ok(request) => request,
        Err(error) => return Some((runner::gone(iteration, placed, error), None)),
    };
    let send_id = format!("run:{}#{iteration}.{steps}", run.id);
    engine.runs.at(&run.id, &send_id);
    let mut flight = Flight::new(Arc::clone(engine), &send_id);
    flight.record = false;
    flight.persist = run.keep_variables;
    flight.overlay = std::mem::take(overlay);
    flight.data.clone_from(row);
    flight.info.iteration = iteration;
    flight.info.iteration_count = iterations;
    let answer = flight
        .send(send::SendRequest {
            id: send_id,
            kind: request.kind,
            method: request.method,
            document: request.document.clone(),
            collection_id: Some(request.collection_id.clone()),
            folder_id: request.folder_id.clone(),
            name: request.name.clone(),
            request_id: Some(request.id.clone()),
            environment_id: run.environment_id.clone(),
            confirmed: run.confirmed,
            // No question halfway through a run: a file a script asks for fails its step.
            files: Vec::new(),
        })
        .await;
    *overlay = std::mem::take(&mut flight.overlay);
    let next = flight.next.take();
    flight.keep().await;
    if matches!(&answer, Err(error) if error.code == crate::error::ErrorCode::HttpCancelled) {
        return None;
    }
    let result = runner::result_of(
        iteration,
        &request,
        placed.path.clone(),
        answer,
        run.keep_responses,
    );
    Some((result, next))
}

/// The delay between two requests, cut short by « Annuler »: `false` when it was.
async fn wait(runs: &runner::Runs, id: &str, millis: u32) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_millis(u64::from(millis));
    while std::time::Instant::now() < until {
        if runs.cancelled(id) {
            return false;
        }
        let left = until.saturating_duration_since(std::time::Instant::now());
        tokio::time::sleep(left.min(std::time::Duration::from_millis(50))).await;
    }
    !runs.cancelled(id)
}

fn say_run(engine: &Engine, id: &str, step: runner::RunStep) {
    engine.emit(EngineEvent::Run(runner::RunEvent {
        id: id.to_string(),
        step,
    }));
}

/// « Annuler » on a run: the request under way is cancelled with it. `false` when no run of that
/// id was under way.
pub fn cancel_http_run(engine: &Engine, id: String) -> Result<bool, AppError> {
    let Some(at) = engine.runs.cancel(&id) else {
        return Ok(false);
    };
    if let Some(send) = at {
        engine.sending.cancel(&send);
    }
    Ok(true)
}

/// A run's data file read: a CSV with its header, or a JSON array of objects.
pub async fn read_http_run_data(path: String) -> Result<runner::RunData, AppError> {
    off_thread(move || runner::read_file(&path)).await?
}

/// `false` when that send had already ended.
pub fn cancel_http_send(engine: &Engine, id: String) -> Result<bool, AppError> {
    Ok(engine.sending.cancel(&id))
}

/// The whole body of the last answer to `id`, not the text shown: `false` when there is none.
pub async fn save_http_response(
    engine: Arc<Engine>,
    id: String,
    path: String,
) -> Result<bool, AppError> {
    crate::transfer::model::validate_path(&path)?;
    off_thread(move || engine.sending.save(&id, Path::new(&path)).context(&path))
        .await?
        .map_err(Into::into)
}

/// A closed tab lets its last body go.
pub fn forget_http_response(engine: &Engine, id: String) -> Result<(), AppError> {
    engine.sending.forget(&id);
    Ok(())
}

/// The last answer to `id` as an image to show, when it is one.
pub async fn http_response_image(
    engine: Arc<Engine>,
    id: String,
) -> Result<Option<String>, AppError> {
    off_thread(move || engine.sending.image(&id)).await
}

/// An answer's text made a note: the secrets the request sent, masked wherever it says them back.
pub async fn http_response_note(
    engine: Arc<Engine>,
    id: Option<String>,
    exchange: send::Exchange,
    text: String,
) -> Result<send::NoteText, AppError> {
    off_thread(move || engine.sending.note_text(id.as_deref(), &exchange, &text)).await
}

/// What was sent, newest first, a group a local day.
pub fn http_history(
    engine: &Engine,
    tz_offset_minutes: i32,
) -> Result<Vec<history::HistoryDay>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::by_day(
        history::store::list(&mut connection)?,
        tz_offset_minutes,
    ))
}

pub fn http_history_entry(engine: &Engine, id: String) -> Result<history::HistoryEntry, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::entry(&mut connection, &id)?)
}

/// What « Vider l'historique » says it removes.
pub fn count_http_history(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::count(&mut connection)?)
}

pub fn clear_http_history(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::clear(&mut connection)?)
}

/// One entry gone, answered whole: its undo hands it back to `restore_http_history_entry`.
pub fn delete_http_history_entry(
    engine: &Engine,
    id: String,
) -> Result<history::HistoryEntry, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::delete(&mut connection, &id)?)
}

pub fn restore_http_history_entry(
    engine: &Engine,
    entry: history::HistoryEntry,
) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::store::restore(&mut connection, &entry)?)
}

/// A send put back onto the request it came from, as changes to save or not, the request's own
/// secrets where the record holds a mask. `None` when that request is gone, or the send came
/// from a draft: the front then opens one, as `http_history_draft` rebuilds it.
pub fn http_history_onto_request(
    engine: &Engine,
    id: String,
) -> Result<Option<history::HistoryDraft>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let entry = history::store::entry(&mut connection, &id)?;
    let Some(request_id) = entry.item.request_id.as_deref() else {
        return Ok(None);
    };
    let current = match store::get_request(&mut connection, request_id) {
        Ok(current) => current,
        Err(crate::error::StorageError::HttpItemNotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let above = store::inherited(
        &mut connection,
        &current.collection_id,
        current.folder_id.as_deref(),
    )?;
    Ok(Some(history::onto(&entry, &current, &above.headers)))
}

/// An entry rebuilt as a request to send again or save: its secrets are typed again.
pub fn http_history_draft(engine: &Engine, id: String) -> Result<history::HistoryDraft, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(history::draft_of(&history::store::entry(
        &mut connection,
        &id,
    )?))
}

/// The operations a GraphQL document names, and why its variables do not read.
pub async fn describe_graphql(
    document: graphql::GraphqlDocument,
) -> Result<graphql::GraphqlAnswer, AppError> {
    off_thread(move || graphql::describe(&document)).await
}

/// The variables with a `null` for each one the operation declares and they lack: « Ajouter ».
pub async fn add_graphql_variables(document: graphql::GraphqlDocument) -> Result<String, AppError> {
    off_thread(move || graphql::with_missing(&document)).await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketRequest {
    /// Chosen by the front: what it sends on and closes, and what each event names.
    pub id: String,
    pub document: model::RequestDocument,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub environment_id: Option<String>,
}

/// Its headers and auth composed like a request's, under the lock for what it inherits; the
/// handshake waited on without it. Answers once the socket is open.
pub async fn connect_websocket(
    engine: Arc<Engine>,
    request: WebsocketRequest,
) -> Result<(), AppError> {
    let WebsocketRequest {
        id,
        mut document,
        collection_id,
        folder_id,
        environment_id,
    } = request;
    document.body = model::RequestBody::None;
    let outgoing = db::run(Arc::clone(&engine), move |engine| {
        let db = &engine.db;
        let mut connection = lock(db)?;
        let inherited = match collection_id {
            Some(collection_id) => {
                store::inherited(&mut connection, &collection_id, folder_id.as_deref())?
            }
            None => settings::inherit(&[]),
        };
        let resolver = resolver_for(
            &mut connection,
            &document.variables,
            &inherited,
            environment_id.as_deref(),
        )?;
        let (mut document, inherited) = variables::resolve(&document, &inherited, &resolver);
        document.url = websocket::socket_url(&document.url)?;
        Ok(send::compose(
            model::RequestKind::Http,
            model::HttpMethod::Get,
            &document,
            &inherited,
        )
        .map(|outgoing| (outgoing, document.websocket.protocols))?)
    })
    .await?;

    let (outgoing, protocols) = outgoing;
    Ok(engine
        .sockets
        .connect(
            &engine.runtime,
            Arc::clone(&engine.events),
            id,
            outgoing,
            &protocols,
        )
        .await?)
}

/// `false` when no socket is open under that id.
pub fn send_websocket(engine: &Engine, id: String, text: String) -> Result<bool, AppError> {
    Ok(engine.sockets.send(&id, text))
}

pub fn close_websocket(engine: &Engine, id: String) -> Result<bool, AppError> {
    Ok(engine.sockets.close(&id))
}

/// The jar by domain, for the manager.
pub fn http_cookies(engine: &Engine) -> Result<Vec<cookies::store::CookieDomain>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::by_domain(&mut connection)?)
}

pub fn delete_http_cookie(engine: &Engine, id: String) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::delete(&mut connection, &id)?)
}

pub fn delete_http_cookie_domain(engine: &Engine, domain: String) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::delete_domain(&mut connection, &domain)?)
}

/// What « Vider le bocal » says it removes.
pub fn count_http_cookies(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::count(&mut connection)?)
}

pub fn clear_http_cookies(engine: &Engine) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(cookies::store::clear(&mut connection)?)
}

/// The globals and every environment, in their order.
pub fn http_environments(engine: &Engine) -> Result<environments::HttpEnvironments, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::list(&mut connection)?)
}

pub fn create_http_environment(
    engine: &Engine,
    name: String,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = environments::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(environments::store::create(
        &mut connection,
        &name,
        Utc::now(),
    )?)
}

pub fn save_http_environment(
    engine: &Engine,
    id: String,
    patch: environments::EnvironmentPatch,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = patch
        .name
        .as_deref()
        .map(environments::validated_name)
        .transpose()?;

    let mut connection = lock(db)?;

    Ok(environments::store::save(
        &mut connection,
        &id,
        &patch,
        name.as_deref(),
    )?)
}

pub fn save_http_globals(
    engine: &Engine,
    variables: Vec<environments::Variable>,
) -> Result<Vec<environments::Variable>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::save_globals(
        &mut connection,
        &variables,
        Utc::now(),
    )?)
}

/// The copy's name comes from the front: no user-visible word is Rust's.
pub fn duplicate_http_environment(
    engine: &Engine,
    id: String,
    name: String,
) -> Result<environments::Environment, AppError> {
    let db = &engine.db;
    let name = environments::validated_name(&name)?;

    let mut connection = lock(db)?;

    Ok(environments::store::duplicate(
        &mut connection,
        &id,
        &name,
        Utc::now(),
    )?)
}

pub fn delete_http_environment(engine: &Engine, id: String) -> Result<(), AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(environments::store::delete(&mut connection, &id)?)
}

/// Each variable's fault by its rank, or none: the table marks the row, the rule is the model's.
pub async fn check_http_variables(
    variables: Vec<environments::Variable>,
) -> Result<Vec<Option<environments::VariableFault>>, AppError> {
    off_thread(move || environments::variable_faults(&variables)).await
}
