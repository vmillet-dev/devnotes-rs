//! Where the interface hands its work to the core, and how the answer comes back (plan § 3.3.5).
//! The UI thread never does I/O, crypto or waiting: everything leaves through one of three
//! queues and returns as a [`Pending`], polled each frame, its worker waking the window.
//!
//! - `db`: anything that touches the database or the index, reads included — the core's one
//!   ordered queue (D11).
//! - `compute`: pure work, on rayon's pool.
//! - `spawn`: the network, on tokio.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};

use engine::Engine;
use engine::error::{AppError, StorageError};

use crate::inputs::Waker;

pub struct Backend {
    engine: Arc<Engine>,
    /// ⚠️ Dropped after `engine`, whose tasks run on it: fields drop in order.
    runtime: tokio::runtime::Runtime,
    waker: Waker,
    in_flight: Arc<InFlight>,
}

impl Backend {
    /// The core, born with its runtime (D20): two workers for the network and the updater.
    pub fn start(
        paths: engine::Paths,
        events: Arc<dyn engine::EventSink>,
        waker: Waker,
    ) -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("devnotes-network")
            .enable_all()
            .build()?;
        let engine = Arc::new(Engine::new(paths, events, runtime.handle().clone()));
        Ok(Self {
            engine,
            runtime,
            waker,
            in_flight: Arc::default(),
        })
    }

    pub fn engine(&self) -> &Arc<Engine> {
        &self.engine
    }

    /// The core's db queue, in the order sent.
    pub fn db<T: Send + 'static>(
        &self,
        job: impl FnOnce(&Engine) -> Result<T, AppError> + Send + 'static,
    ) -> Pending<T> {
        let (pending, answer) = self.pending();
        // Queued here, on the caller's thread: a task would reach the queue in its own time.
        let answered = engine::db::submit(&self.engine, job);
        self.runtime
            .spawn(async move { answer.send(answered.await) });
        pending
    }

    /// Pure work, on rayon's pool: neither the database nor the index.
    pub fn compute<T: Send + 'static>(
        &self,
        job: impl FnOnce(&Engine) -> Result<T, AppError> + Send + 'static,
    ) -> Pending<T> {
        let (pending, answer) = self.pending();
        let engine = Arc::clone(&self.engine);
        rayon::spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| job(&engine)));
            answer.send(result.unwrap_or_else(|_| Err(panicked())));
        });
        pending
    }

    /// The network, on tokio. A future that panics answers an error like the others.
    pub fn spawn<T, F>(&self, job: impl FnOnce(Arc<Engine>) -> F + Send + 'static) -> Pending<T>
    where
        F: Future<Output = Result<T, AppError>> + Send + 'static,
        T: Send + 'static,
    {
        let (pending, answer) = self.pending();
        let engine = Arc::clone(&self.engine);
        let handle = self.runtime.handle().clone();
        self.runtime.spawn(async move {
            let result = handle.spawn(job(engine)).await;
            answer.send(result.unwrap_or_else(|_| Err(panicked())));
        });
        pending
    }

    fn pending<T>(&self) -> (Pending<T>, Answer<T>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.in_flight.add();
        let answer = Answer {
            sender: Some(sender),
            waker: self.waker.clone(),
            in_flight: Arc::clone(&self.in_flight),
        };
        (
            Pending {
                receiver,
                answered: false,
            },
            answer,
        )
    }

    /// Tests only: waits until everything launched has answered. A condition, never a duration.
    #[cfg(test)]
    pub fn settle(&self) {
        self.in_flight.wait_for_none();
    }
}

/// What a panic answers: the same as a poisoned lock, `Unavailable` (D18). The panic hook
/// has logged where it happened.
fn panicked() -> AppError {
    StorageError::Unavailable.into()
}

/// A result to come, read by [`Pending::poll`] each frame. Dropping it does not stop work
/// begun on the database; a send is cancelled explicitly.
#[must_use = "a Pending is the only way to its answer"]
pub struct Pending<T> {
    receiver: Receiver<Result<T, AppError>>,
    answered: bool,
}

impl<T> Pending<T> {
    /// The answer, the one frame it arrives; `None` before and after.
    pub fn poll(&mut self) -> Option<Result<T, AppError>> {
        if self.answered {
            return None;
        }
        let answer = match self.receiver.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The worker is gone without a word: still an answer, never a silence.
            Err(TryRecvError::Disconnected) => Err(panicked()),
        };
        self.answered = true;
        Some(answer)
    }
}

/// The worker's end: sending wakes the window; dropping without sending still counts.
struct Answer<T> {
    sender: Option<SyncSender<Result<T, AppError>>>,
    waker: Waker,
    in_flight: Arc<InFlight>,
}

impl<T> Answer<T> {
    fn send(mut self, result: Result<T, AppError>) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(result);
        }
    }
}

impl<T> Drop for Answer<T> {
    fn drop(&mut self) {
        self.sender = None;
        self.waker.wake();
        self.in_flight.remove();
    }
}

/// How many answers are still to come.
#[derive(Default)]
struct InFlight {
    count: Mutex<usize>,
    changed: Condvar,
}

impl InFlight {
    fn add(&self) {
        if let Ok(mut count) = self.count.lock() {
            *count += 1;
        }
    }

    fn remove(&self) {
        if let Ok(mut count) = self.count.lock() {
            *count = count.saturating_sub(1);
            self.changed.notify_all();
        }
    }

    #[cfg(test)]
    fn wait_for_none(&self) {
        let mut count = self.count.lock().unwrap();
        while *count > 0 {
            count = self.changed.wait(count).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use engine::error::ErrorCode;

    use super::*;
    use crate::testing;

    #[test]
    fn each_queue_answers_and_wakes_the_window() {
        let (backend, _profile) = testing::backend();
        let ctx = egui::Context::default();
        backend.waker.fill(&ctx);

        let mut read = backend.db(|_| Ok(1));
        let mut computed = backend.compute(|_| Ok(2));
        let mut sent = backend.spawn(|_| async { Ok(3) });
        backend.settle();

        assert_eq!(read.poll().unwrap().unwrap(), 1);
        assert_eq!(computed.poll().unwrap().unwrap(), 2);
        assert_eq!(sent.poll().unwrap().unwrap(), 3);
        assert!(read.poll().is_none(), "an answer is read once");
        assert!(ctx.has_requested_repaint());
    }

    #[test]
    fn a_job_that_panics_answers_an_error_on_every_queue() {
        let (backend, _profile) = testing::backend();

        let mut read = backend.db::<()>(|_| panic!("in the db queue"));
        let mut computed = backend.compute::<()>(|_| panic!("in the pool"));
        let mut sent = backend.spawn::<(), _>(|_| async { panic!("in the runtime") });
        backend.settle();

        for answer in [read.poll(), computed.poll(), sent.poll()] {
            let error = answer.expect("no Pending is left unanswered").unwrap_err();
            assert_eq!(error.code, ErrorCode::StorageUnavailable);
        }
        let mut next = backend.db(|_| Ok("the queue goes on"));
        backend.settle();
        assert_eq!(next.poll().unwrap().unwrap(), "the queue goes on");
    }

    #[test]
    fn two_writes_sent_in_a_row_land_in_that_order() {
        let (backend, _profile) = testing::backend();
        let order = Arc::new(Mutex::new(Vec::new()));

        let pendings: Vec<_> = (0..20)
            .map(|step| {
                let order = Arc::clone(&order);
                backend.db(move |_| {
                    order.lock().unwrap().push(step);
                    Ok(())
                })
            })
            .collect();
        backend.settle();

        drop(pendings);
        assert_eq!(*order.lock().unwrap(), (0..20).collect::<Vec<_>>());
    }
}
