//! A collection or a folder run: what is run and how, its order, and the report of the last run,
//! kept for the session and never in the history. Rust runs; this follows its steps.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use engine::error::ErrorCode;
use engine::http::environments::Environment;
use engine::http::model::{HttpItem, HttpMethod, HttpTree, RequestKind, RunEntry};
use engine::http::runner::{RunData, RunEvent, RunRequest, RunResult, RunStep, RunSummary};
use i18n::keys;
use platform::dialog::FileDialog;

use super::tree;
use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::state::Revision;

/// What a run is of: a collection or a folder, by its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub item: HttpItem,
    pub name: String,
}

/// A request of the run's order, and whether it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: String,
    /// The folders between it and what is run.
    pub path: Vec<String>,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    /// A protected environment waits for its yes.
    Asking,
    Running,
    Done,
}

/// Where a run stands while it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub iteration: u32,
    pub index: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ended {
    pub cancelled: bool,
    pub millis: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Passed,
    Failed,
}

impl Filter {
    pub const ALL: [Self; 3] = [Self::All, Self::Passed, Self::Failed];

    pub fn holds(self, result: &RunResult) -> bool {
        self == Self::All || (self == Self::Failed) == result.failed()
    }
}

/// The data file chosen: its name, and its rows as Rust read them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFile {
    pub name: String,
    pub data: RunData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Open(Target),
    Move {
        from: usize,
        to: usize,
    },
    Toggle(usize),
    Iterations(u32),
    Delay(u32),
    StopOnFailure(bool),
    KeepVariables(bool),
    KeepResponses(bool),
    ChooseData,
    ClearData,
    Start,
    /// The one yes a protected environment asked for.
    StartAnyway,
    StopAsking,
    Cancel,
    Filter(Filter),
    Fold(usize),
}

/// What a run reads from the area around it.
#[derive(Clone, Copy)]
pub struct Around<'a> {
    pub tree: &'a HttpTree,
    pub environment: Option<&'a Environment>,
}

pub struct Runner {
    target: Option<Target>,
    rows: Vec<Row>,
    iterations: u32,
    delay_ms: u32,
    stop_on_failure: bool,
    keep_variables: bool,
    keep_responses: bool,
    data: Option<DataFile>,
    phase: Phase,
    results: Vec<RunResult>,
    progress: Option<Progress>,
    ended: Option<Ended>,
    /// The requests the last run was to send: an order changed since is not it.
    planned: u32,
    started: u32,
    run_id: Option<String>,
    filter: Filter,
    /// The results unfolded, by their rank: a failed one opens on its assertions.
    open: BTreeSet<usize>,
    /// The one kept answer read under the report: the last unfolded.
    answer: Option<usize>,
    /// The tree the order was last read after: a request added or deleted, and it follows.
    followed: Option<Revision>,
    order: Option<(String, Pending<Vec<RunEntry>>)>,
    saving: Vec<Pending<()>>,
    picking: Option<Pending<Option<PathBuf>>>,
    reading: Option<(String, Pending<RunData>)>,
    running: Option<Pending<()>>,
    cancelling: Option<Pending<bool>>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            target: None,
            rows: Vec::new(),
            iterations: 1,
            delay_ms: 0,
            stop_on_failure: false,
            keep_variables: true,
            keep_responses: false,
            data: None,
            phase: Phase::default(),
            results: Vec::new(),
            progress: None,
            ended: None,
            planned: 0,
            started: 0,
            run_id: None,
            filter: Filter::default(),
            open: BTreeSet::new(),
            answer: None,
            followed: None,
            order: None,
            saving: Vec::new(),
            picking: None,
            reading: None,
            running: None,
            cancelling: None,
        }
    }
}

fn rows_under(tree: &HttpTree, item: &HttpItem) -> Vec<Row> {
    api::http::placed_under(tree, item)
        .into_iter()
        .filter_map(|placed| {
            let request = tree::find_request(tree, &placed.id)?;
            Some(Row {
                kind: request.kind,
                method: request.method,
                id: placed.id,
                name: placed.name,
                path: placed.path,
                enabled: true,
            })
        })
        .collect()
}

fn file_name(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next();
    name.unwrap_or(path).to_owned()
}

impl Runner {
    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn enabled(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|row| row.enabled)
    }

    pub fn enabled_count(&self) -> u32 {
        u32::try_from(self.enabled().count()).unwrap_or(u32::MAX)
    }

    /// What a protected environment asks about before the run.
    pub fn changing(&self) -> impl Iterator<Item = &Row> {
        self.enabled()
            .filter(|row| api::http::asks_before_sending(row.kind, row.method))
    }

    /// With a data file, one iteration a row: the field says so, greyed.
    pub fn runs(&self) -> u32 {
        self.data.as_ref().map_or(self.iterations.max(1), |file| {
            u32::try_from(file.data.rows.len()).unwrap_or(u32::MAX)
        })
    }

    pub fn delay_ms(&self) -> u32 {
        self.delay_ms
    }

    pub fn stop_on_failure(&self) -> bool {
        self.stop_on_failure
    }

    pub fn keep_variables(&self) -> bool {
        self.keep_variables
    }

    pub fn keep_responses(&self) -> bool {
        self.keep_responses
    }

    pub fn data(&self) -> Option<&DataFile> {
        self.data.as_ref()
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_running(&self) -> bool {
        self.phase == Phase::Running
    }

    pub fn results(&self) -> &[RunResult] {
        &self.results
    }

    pub fn summary(&self) -> RunSummary {
        RunSummary::of(&self.results)
    }

    pub fn progress(&self) -> Option<&Progress> {
        self.progress.as_ref()
    }

    pub fn ended(&self) -> Option<Ended> {
        self.ended
    }

    pub fn planned(&self) -> u32 {
        self.planned
    }

    pub fn filter(&self) -> Filter {
        self.filter
    }

    pub fn is_open(&self, at: usize) -> bool {
        self.open.contains(&at)
    }

    /// The kept answer read under the report, with its rank.
    pub fn answer(&self) -> Option<(usize, &engine::http::send::SentResponse)> {
        let at = self.answer?;
        Some((at, self.results.get(at)?.response.as_ref()?))
    }

    /// The console of the last run, under a key of its own.
    pub fn console_key(&self) -> Option<String> {
        let id = self.run_id.as_ref()?;
        Some(format!("run:{id}"))
    }

    pub fn ask(
        &mut self,
        (backend, dialog): (&Backend, &Arc<dyn FileDialog + Send + Sync>),
        asked: Asked,
        around: Around<'_>,
    ) {
        match asked {
            Asked::Open(target) => self.open(around.tree, target),
            Asked::Move { from, to } => {
                if from != to && from < self.rows.len() {
                    let row = self.rows.remove(from);
                    self.rows.insert(to.min(self.rows.len()), row);
                    self.keep(backend);
                }
            }
            Asked::Toggle(at) => {
                if let Some(row) = self.rows.get_mut(at) {
                    row.enabled = !row.enabled;
                    self.keep(backend);
                }
            }
            Asked::Iterations(count) => self.iterations = count,
            Asked::Delay(millis) => self.delay_ms = millis,
            Asked::StopOnFailure(on) => self.stop_on_failure = on,
            Asked::KeepVariables(on) => self.keep_variables = on,
            Asked::KeepResponses(on) => self.keep_responses = on,
            Asked::ChooseData => {
                let dialog = Arc::clone(dialog);
                self.picking = Some(backend.compute(move |_| Ok(dialog.open())));
            }
            Asked::ClearData => self.data = None,
            Asked::Start => self.start(backend, around, false),
            Asked::StartAnyway => self.start(backend, around, true),
            Asked::StopAsking => self.stop_asking(),
            Asked::Cancel => self.cancel(backend),
            Asked::Filter(filter) => self.filter = filter,
            Asked::Fold(at) => self.fold(at),
        }
    }

    /// « Run… » on a collection or a folder: its requests in the tree's order at once, then in
    /// the order last arranged, kept by Rust with the requests added since at its end.
    fn open(&mut self, tree: &HttpTree, target: Target) {
        let same = self
            .target
            .as_ref()
            .is_some_and(|open| open.item == target.item);
        if !same {
            self.rows = rows_under(tree, &target.item);
            self.clear_report();
        }
        self.target = Some(target);
        self.followed = None;
    }

    fn keep(&mut self, backend: &Backend) {
        let Some(target) = &self.target else {
            return;
        };
        let order = self
            .rows
            .iter()
            .map(|row| RunEntry {
                id: row.id.clone(),
                enabled: row.enabled,
            })
            .collect();
        let saved = api::http::save_run_order(backend, target.item.clone(), order);
        self.saving.push(saved);
        // An order read before this write would undo it: read again behind it.
        self.order = None;
        self.followed = None;
    }

    fn start(&mut self, backend: &Backend, around: Around<'_>, confirmed: bool) {
        if self.is_running() || self.enabled_count() == 0 {
            return;
        }
        let Some(target) = self.target.clone() else {
            return;
        };
        let protected = around.environment.is_some_and(|active| active.protected);
        if !confirmed && protected && self.changing().next().is_some() {
            self.phase = Phase::Asking;
            return;
        }
        self.clear_report();
        self.started += 1;
        let id = self.started.to_string();
        self.run_id = Some(id.clone());
        self.planned = self.enabled_count().saturating_mul(self.runs());
        self.phase = Phase::Running;
        let run = RunRequest {
            id,
            item: target.item,
            order: self.enabled().map(|row| row.id.clone()).collect(),
            iterations: self.runs(),
            delay_ms: self.delay_ms,
            data: self.data.as_ref().map(|file| file.data.clone()),
            stop_on_failure: self.stop_on_failure,
            keep_variables: self.keep_variables,
            keep_responses: self.keep_responses,
            environment_id: around.environment.map(|active| active.id.clone()),
            confirmed,
        };
        self.running = Some(api::http::run_collection(backend, run));
    }

    fn stop_asking(&mut self) {
        if self.phase == Phase::Asking {
            self.phase = if self.results.is_empty() {
                Phase::Idle
            } else {
                Phase::Done
            };
        }
    }

    fn cancel(&mut self, backend: &Backend) {
        if let Some(id) = self.run_id.clone().filter(|_| self.is_running()) {
            self.cancelling = Some(api::http::cancel_run(backend, id));
        }
    }

    fn fold(&mut self, at: usize) {
        if self.open.remove(&at) {
            if self.answer == Some(at) {
                self.answer = None;
            }
            return;
        }
        self.open.insert(at);
        if self.results.get(at).is_some_and(|it| it.response.is_some()) {
            self.answer = Some(at);
        }
    }

    fn clear_report(&mut self) {
        self.results.clear();
        self.progress = None;
        self.ended = None;
        self.filter = Filter::default();
        self.open.clear();
        self.answer = None;
        if !self.is_running() {
            self.phase = Phase::Idle;
        }
    }

    pub fn receive(&mut self, event: RunEvent) {
        if self.run_id.as_deref() != Some(event.id.as_str()) {
            return;
        }
        match event.step {
            RunStep::Started {
                iteration,
                index,
                name,
            } => {
                self.progress = Some(Progress {
                    iteration,
                    index,
                    name,
                });
            }
            RunStep::Ended { result } => {
                if result.failed() {
                    self.open.insert(self.results.len());
                }
                self.results.push(*result);
            }
            RunStep::Finished { cancelled, millis } => {
                self.ended = Some(Ended { cancelled, millis });
            }
        }
    }

    /// `true` when a run ended that may have written variables.
    pub fn update(
        &mut self,
        backend: &Backend,
        (tree, read): (&HttpTree, Option<Revision>),
        errors: &mut ErrorNotifier,
    ) -> bool {
        if let Some(target) = &self.target
            && read.is_some()
            && self.followed != read
        {
            self.followed = read;
            let order = api::http::run_order(backend, target.item.clone());
            self.order = Some((target.item.id.clone(), order));
        }
        self.adopt_order(tree, errors);
        self.saving.retain_mut(|saving| match saving.poll() {
            None => true,
            Some(Ok(())) => false,
            Some(Err(error)) => {
                errors.report(keys::http::runner::FAILED, &error);
                false
            }
        });
        if let Some(Some(Err(error))) = self.cancelling.as_mut().map(Pending::poll) {
            errors.report(keys::http::runner::FAILED, &error);
        }
        self.adopt_data(backend, errors);
        self.adopt_end(errors)
    }

    fn adopt_order(&mut self, tree: &HttpTree, errors: &mut ErrorNotifier) {
        let Some((id, pending)) = &mut self.order else {
            return;
        };
        let Some(answer) = pending.poll() else {
            return;
        };
        let id = id.clone();
        self.order = None;
        let order = match answer {
            Ok(order) => order,
            Err(error) => return errors.report(keys::http::runner::FAILED, &error),
        };
        let Some(target) = self.target.as_ref().filter(|open| open.item.id == id) else {
            return;
        };
        let known = rows_under(tree, &target.item);
        self.rows = order
            .into_iter()
            .filter_map(|entry| {
                let row = known.iter().find(|row| row.id == entry.id)?;
                Some(Row {
                    enabled: entry.enabled,
                    ..row.clone()
                })
            })
            .collect();
    }

    fn adopt_data(&mut self, backend: &Backend, errors: &mut ErrorNotifier) {
        if let Some(answer) = self.picking.as_mut().and_then(Pending::poll) {
            self.picking = None;
            match answer {
                Ok(Some(path)) => {
                    let path = path.display().to_string();
                    let read = api::http::read_run_data(backend, path.clone());
                    self.reading = Some((file_name(&path), read));
                }
                Ok(None) => {}
                Err(error) => errors.report(keys::http::runner::DATA_FAILED, &error),
            }
        }
        let Some((name, pending)) = &mut self.reading else {
            return;
        };
        let Some(answer) = pending.poll() else {
            return;
        };
        let name = name.clone();
        self.reading = None;
        match answer {
            Ok(data) => self.data = Some(DataFile { name, data }),
            Err(error) => errors.report(keys::http::runner::DATA_FAILED, &error),
        }
    }

    fn adopt_end(&mut self, errors: &mut ErrorNotifier) -> bool {
        let Some(answer) = self.running.as_mut().and_then(Pending::poll) else {
            return false;
        };
        self.running = None;
        self.phase = Phase::Done;
        self.progress = None;
        if let Err(error) = answer
            && error.code != ErrorCode::HttpCancelled
        {
            errors.report(keys::http::runner::FAILED, &error);
        }
        self.keep_variables
    }
}
