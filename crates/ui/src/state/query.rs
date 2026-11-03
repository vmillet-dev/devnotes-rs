//! A value the core is asked for again whenever what it depends on changes: the list for a
//! query, the board for a space. It replaces the reference's `resource()`, `linkedSignal` and
//! `OneInFlight` (plan § 3.3.5).

use engine::error::AppError;

use crate::backend::Pending;

/// Keeps the last value while the next one loads, with **one call in flight and the latest
/// waiting**: typing a search sends no more calls than the core can answer. Parameters are
/// compared by value, so asking twice for the same thing asks once.
pub struct QueryCell<P, T> {
    value: Option<T>,
    error: Option<AppError>,
    /// The parameters of the last answer adopted, value or error.
    answered: Option<P>,
    wanted: Option<P>,
    in_flight: Option<(P, Pending<T>)>,
}

impl<P: PartialEq + Clone, T> Default for QueryCell<P, T> {
    fn default() -> Self {
        Self {
            value: None,
            error: None,
            answered: None,
            wanted: None,
            in_flight: None,
        }
    }
}

impl<P: PartialEq + Clone, T> QueryCell<P, T> {
    /// What the screen now depends on. Asked of the core at the next [`Self::update`].
    pub fn want(&mut self, params: P) {
        self.wanted = Some(params);
    }

    /// Once a frame: adopts the answer that arrived, unless newer parameters were wanted since
    /// — a stale search is dropped — then asks for what is wanted, if nothing is in flight.
    /// `true` when the value or the error changed.
    pub fn update(&mut self, launch: impl FnOnce(P) -> Pending<T>) -> bool {
        let mut changed = false;
        if let Some((params, pending)) = &mut self.in_flight
            && let Some(answer) = pending.poll()
        {
            let params = params.clone();
            self.in_flight = None;
            if self.wanted.as_ref() == Some(&params) {
                match answer {
                    Ok(value) => {
                        self.value = Some(value);
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
                self.answered = Some(params);
                changed = true;
            }
        }
        if self.in_flight.is_none()
            && let Some(wanted) = &self.wanted
            && self.answered.as_ref() != Some(wanted)
        {
            let params = wanted.clone();
            self.in_flight = Some((params.clone(), launch(params)));
        }
        changed
    }

    /// The last value adopted, kept while the next one loads.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// The last answer was an error; the value before it is still there.
    pub fn error(&self) -> Option<&AppError> {
        self.error.as_ref()
    }

    pub fn is_loading(&self) -> bool {
        self.in_flight.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::backend::Backend;
    use crate::testing;

    /// A core whose answer to a query is its parameter, each call counted.
    fn asker(backend: &Backend, calls: &Arc<Mutex<Vec<u32>>>) -> impl FnOnce(u32) -> Pending<u32> {
        let calls = Arc::clone(calls);
        move |params| {
            calls.lock().unwrap().push(params);
            backend.db(move |_| Ok(params))
        }
    }

    #[test]
    fn the_same_parameters_are_asked_once() {
        let (backend, _profile) = testing::backend();
        let calls = Arc::default();
        let mut cell = QueryCell::default();

        for _ in 0..3 {
            cell.want(7);
            cell.update(asker(&backend, &calls));
            backend.settle();
        }

        assert_eq!(cell.value(), Some(&7));
        assert_eq!(*calls.lock().unwrap(), [7]);
    }

    #[test]
    fn while_a_call_is_in_flight_only_the_latest_waits() {
        let (backend, _profile) = testing::backend();
        let calls = Arc::default();
        let mut cell = QueryCell::default();

        cell.want(1);
        cell.update(asker(&backend, &calls));
        for typed in 2..=5 {
            cell.want(typed);
        }
        backend.settle();
        // The answer for 1 is stale: 5 is what the screen wants now.
        cell.update(asker(&backend, &calls));
        assert_eq!(cell.value(), None);
        assert!(cell.is_loading());
        backend.settle();
        cell.update(asker(&backend, &calls));

        assert_eq!(cell.value(), Some(&5));
        assert_eq!(*calls.lock().unwrap(), [1, 5]);
    }

    #[test]
    fn the_last_value_is_kept_while_the_next_loads_and_after_an_error() {
        let (backend, _profile) = testing::backend();
        let calls = Arc::default();
        let mut cell = QueryCell::default();
        cell.want(1);
        cell.update(asker(&backend, &calls));
        backend.settle();
        cell.update(asker(&backend, &calls));

        cell.want(2);
        cell.update(|_| backend.db(|_| Err(engine::error::AppError::storage_unavailable())));
        assert_eq!(cell.value(), Some(&1));
        backend.settle();
        assert!(cell.update(asker(&backend, &calls)));

        assert_eq!(cell.value(), Some(&1));
        assert!(cell.error().is_some());
        assert!(
            !cell.update(asker(&backend, &calls)),
            "an error is not asked again in a loop"
        );
    }
}
