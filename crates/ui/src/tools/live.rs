//! Every tool's round trip to the core as its inputs change: compared by value, debounced as a
//! deadline, the previous answer kept until the next lands (the reference's `liveResult`).

use std::sync::Arc;

use i18n::keys;
use platform::clipboard::Clipboard;

use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;
use crate::services::status::StatusNotifier;
use crate::state::query::QueryCell;

/// Long enough to let a word be typed, short enough to read as live.
const DEBOUNCE: f64 = 0.150;

/// What a tool has at hand once a frame, on screen.
pub struct Turn<'a> {
    pub ctx: &'a egui::Context,
    pub backend: &'a Backend,
    pub time: f64,
    pub now: chrono::DateTime<chrono::Utc>,
    pub errors: &'a mut ErrorNotifier,
    pub status: &'a mut StatusNotifier,
    pub clipboard: &'a Arc<dyn Clipboard + Send + Sync>,
}

pub struct Live<P, T> {
    cell: QueryCell<P, T>,
    typed: Option<P>,
    due: Option<f64>,
    delay: f64,
}

impl<P: PartialEq + Clone, T> Default for Live<P, T> {
    fn default() -> Self {
        Self {
            cell: QueryCell::default(),
            typed: None,
            due: None,
            delay: DEBOUNCE,
        }
    }
}

impl<P: PartialEq + Clone, T> Live<P, T> {
    /// For what costs more to compute: two whole texts compared.
    pub fn slower(delay: f64) -> Self {
        Self {
            delay,
            ..Self::default()
        }
    }

    /// What the inputs ask now, `None` for nothing, and the answer that came. The first request
    /// goes at once: a tool opened on a text shows its answer without a wait.
    pub fn update(
        &mut self,
        turn: &mut Turn<'_>,
        request: Option<P>,
        launch: impl FnOnce(&Backend, P) -> Pending<T>,
    ) {
        if request != self.typed {
            self.due = match (&request, &self.typed) {
                (None, _) => None,
                (Some(_), None) => Some(turn.time),
                (Some(_), Some(_)) => Some(turn.time + self.delay),
            };
            if request.is_none() {
                self.cell = QueryCell::default();
            }
            self.typed = request;
        }
        self.settle(turn);
        let changed = self.cell.update(|params| launch(turn.backend, params));
        // A tool's own failure is part of its answer; what lands here is the unexpected.
        if changed && let Some(error) = self.cell.error() {
            turn.errors.report(keys::errors::TOOL_FAILED, error);
        }
    }

    fn settle(&mut self, turn: &Turn<'_>) {
        let Some(due) = self.due else { return };
        if turn.time < due {
            let wait = std::time::Duration::from_secs_f64(due - turn.time);
            turn.ctx.request_repaint_after(wait);
            return;
        }
        self.due = None;
        if let Some(params) = &self.typed {
            self.cell.want(params.clone());
        }
    }

    /// The last answer, kept on screen while the next is computed; `None` once nothing is asked.
    pub fn value(&self) -> Option<&T> {
        self.cell.value()
    }

    /// ⚠️ The request `value` answers, not the one being typed: what describes the answer reads
    /// this, or it describes the next answer over the last one.
    pub fn answered(&self) -> Option<&P> {
        self.cell.answered()
    }

    pub fn is_pending(&self) -> bool {
        self.due.is_some() || self.cell.is_loading()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    struct Bench {
        backend: Backend,
        errors: ErrorNotifier,
        status: StatusNotifier,
        clipboard: Arc<dyn Clipboard + Send + Sync>,
        ctx: egui::Context,
        live: Live<u32, u32>,
        launched: Vec<u32>,
    }

    impl Bench {
        fn new() -> (Self, tempfile::TempDir) {
            let (backend, profile) = testing::backend();
            let bench = Self {
                backend,
                errors: ErrorNotifier::default(),
                status: StatusNotifier::default(),
                clipboard: platform::clipboard::system(),
                ctx: egui::Context::default(),
                live: Live::default(),
                launched: Vec::new(),
            };
            (bench, profile)
        }

        fn at(&mut self, time: f64, request: Option<u32>) {
            let mut turn = Turn {
                ctx: &self.ctx,
                backend: &self.backend,
                time,
                now: chrono::Utc::now(),
                errors: &mut self.errors,
                status: &mut self.status,
                clipboard: &self.clipboard,
            };
            let launched = &mut self.launched;
            self.live.update(&mut turn, request, |backend, params| {
                launched.push(params);
                backend.compute(move |_| Ok(params * 2))
            });
            self.backend.settle();
        }
    }

    #[test]
    fn the_first_request_goes_at_once_and_the_next_waits_for_the_typing_to_stop() {
        let (mut bench, _profile) = Bench::new();

        bench.at(0.0, Some(1));
        bench.at(0.01, Some(2));
        bench.at(0.1, Some(3));
        assert_eq!(bench.launched, [1]);

        bench.at(0.3, Some(3));
        assert_eq!(bench.launched, [1, 3]);
    }

    #[test]
    fn the_last_answer_stays_until_the_next_and_goes_with_the_request() {
        let (mut bench, _profile) = Bench::new();
        bench.at(0.0, Some(1));
        bench.at(0.01, Some(1));
        assert_eq!(bench.live.value(), Some(&2));

        bench.at(0.02, Some(5));
        assert_eq!(bench.live.value(), Some(&2));
        assert_eq!(bench.live.answered(), Some(&1));
        assert!(bench.live.is_pending());

        bench.at(0.03, None);
        assert_eq!(bench.live.value(), None);
    }

    #[test]
    fn asking_the_same_again_asks_nothing() {
        let (mut bench, _profile) = Bench::new();

        for frame in 0..5 {
            bench.at(f64::from(frame), Some(1));
        }

        assert_eq!(bench.launched, [1]);
    }
}
