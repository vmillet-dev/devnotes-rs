//! The one thread that touches the database, running its jobs in the order they arrive.
//!
//! Two writes sent one after the other land in that order, and the lock is never contended. A
//! job never waits on another job of the queue, nor on the network: that would deadlock it.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;

use tokio::sync::oneshot;

use crate::error::{AppError, StorageError};

type Job = Box<dyn FnOnce() + Send>;

thread_local! {
    static ON_DB_THREAD: Cell<bool> = const { Cell::new(false) };
}

/// Whether the caller runs on the queue's thread, which `lock` checks in debug builds.
pub(crate) fn on_db_thread() -> bool {
    ON_DB_THREAD.get()
}

/// Runs `job` here as if this were the db thread: `db::run_here`, for the integration tests.
pub(crate) fn as_db_thread<T>(job: impl FnOnce() -> T) -> T {
    let was = ON_DB_THREAD.replace(true);
    let answer = job();
    ON_DB_THREAD.set(was);
    answer
}

pub(crate) struct DbQueue(mpsc::Sender<Job>);

impl DbQueue {
    /// The thread ends once the queue is dropped and its last job has run.
    pub(crate) fn start() -> Self {
        let (sender, jobs) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("db".to_string())
            .spawn(move || {
                ON_DB_THREAD.set(true);
                for job in jobs {
                    job();
                }
            })
            .expect("the db thread starts");

        Self(sender)
    }

    /// Waits for `job` without holding a thread. A job that panics answers as the lock it
    /// would have poisoned, and the queue goes on with the next one.
    pub(crate) async fn run<T, F>(&self, job: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
    {
        self.submit(job).await
    }

    /// Queues `job` now, in the caller's order, and answers a future of its result: what
    /// keeps two calls made one after the other in that order when the waiting is done
    /// elsewhere.
    pub(crate) fn submit<T, F>(
        &self,
        job: F,
    ) -> impl Future<Output = Result<T, AppError>> + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
    {
        let (answer, answered) = oneshot::channel();
        let job: Job = Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(job))
                .unwrap_or_else(|_| Err(StorageError::Unavailable.into()));
            let _ = answer.send(result);
        });
        let queued = self.0.send(job).map_err(|_| StorageError::Unavailable);

        async move {
            queued?;
            answered.await.map_err(|_| StorageError::Unavailable)?
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::error::ErrorCode;

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn jobs_run_on_the_db_thread_in_the_order_they_were_sent() {
        let queue = DbQueue::start();
        let order = Arc::new(Mutex::new(Vec::new()));
        let job = |n: u32| {
            let order = Arc::clone(&order);
            queue.run(move || {
                order.lock().unwrap().push((n, on_db_thread()));
                Ok(())
            })
        };

        block_on(async {
            let (a, b, c) = tokio::join!(job(1), job(2), job(3));
            a.and(b).and(c)
        })
        .unwrap();

        assert_eq!(*order.lock().unwrap(), [(1, true), (2, true), (3, true)]);
        assert!(!on_db_thread());
    }

    #[test]
    fn a_job_that_panics_answers_an_error_and_the_next_one_runs() {
        let queue = DbQueue::start();

        let panicked = block_on(queue.run(|| -> Result<(), AppError> { panic!("on purpose") }));
        let next = block_on(queue.run(|| Ok(7)));

        assert_eq!(panicked.unwrap_err().code, ErrorCode::StorageUnavailable);
        assert_eq!(next.unwrap(), 7);
    }
}
