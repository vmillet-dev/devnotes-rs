//! The course of a collection's run: one request of one iteration at a time, each a send of its
//! own, the delay between them cut short by « Annuler ».

use std::collections::BTreeMap;
use std::sync::Arc;

use super::flight::Flight;
use super::scripts::model::VariableWrite;
use super::{runner, send, store};
use crate::db::{self, lock};
use crate::{Engine, EngineEvent};

/// One request of one iteration of a run.
pub(super) struct RunStep<'a> {
    pub(super) run: &'a runner::RunRequest,
    pub(super) iteration: u32,
    pub(super) iterations: u32,
    pub(super) index: usize,
    pub(super) steps: u32,
    pub(super) placed: &'a runner::Placed,
    pub(super) row: &'a BTreeMap<String, String>,
}

/// A request of a run, sent; `None` when « Annuler » stopped it. Beside its result, where a
/// script said to go next.
pub(super) async fn run_step(
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
pub(super) async fn wait(runs: &runner::Runs, id: &str, millis: u32) -> bool {
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

pub(super) fn say_run(engine: &Engine, id: &str, step: runner::RunStep) {
    engine.emit(EngineEvent::Run(runner::RunEvent {
        id: id.to_string(),
        step,
    }));
}
