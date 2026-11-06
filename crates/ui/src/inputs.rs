//! What reaches the interface from other threads, drained at the start of every frame. The
//! channel exists before the window does (D20): what arrives first waits in it.

use std::sync::{Arc, OnceLock};

use crossbeam_channel::{Receiver, Sender};
use engine::{EngineEvent, EventSink};

pub enum Input {
    Engine(EngineEvent),
    /// What a second launch on the same profile asked.
    Instance(platform::instance::Message),
    Tray(platform::tray::Asked),
}

/// Wakes the interface from any thread, once its context exists.
#[derive(Clone, Default)]
pub struct Waker(Arc<OnceLock<egui::Context>>);

impl Waker {
    pub fn fill(&self, ctx: &egui::Context) {
        let _ = self.0.set(ctx.clone());
    }

    pub fn wake(&self) {
        if let Some(ctx) = self.0.get() {
            ctx.request_repaint();
        }
    }
}

/// The sending end every emitter holds: it posts, then wakes.
#[derive(Clone)]
pub struct Inbox {
    sender: Sender<Input>,
    waker: Waker,
}

impl Inbox {
    pub fn post(&self, input: Input) {
        // Refused only once the interface is gone, when nobody is left to tell.
        if self.sender.send(input).is_ok() {
            self.waker.wake();
        }
    }
}

impl EventSink for Inbox {
    fn emit(&self, event: EngineEvent) {
        self.post(Input::Engine(event));
    }
}

/// The channel, its waker, and the end the interface drains.
pub fn channel() -> (Inbox, Waker, Receiver<Input>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    let waker = Waker::default();
    let inbox = Inbox {
        sender,
        waker: waker.clone(),
    };
    (inbox, waker, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_engine_event_reaches_the_interface_and_wakes_it() {
        let (inbox, waker, inputs) = channel();
        let ctx = egui::Context::default();
        waker.fill(&ctx);

        inbox.emit(EngineEvent::IndexReady);

        assert!(matches!(
            inputs.try_recv(),
            Ok(Input::Engine(EngineEvent::IndexReady))
        ));
        assert!(ctx.has_requested_repaint());
    }

    #[test]
    fn what_arrives_before_the_window_waits_in_the_channel() {
        let (inbox, _waker, inputs) = channel();

        inbox.emit(EngineEvent::IndexProgress { done: 1, total: 2 });

        assert_eq!(inputs.len(), 1);
    }

    #[test]
    fn what_arrives_after_the_interface_is_gone_is_dropped() {
        let (inbox, _waker, inputs) = channel();
        drop(inputs);

        inbox.emit(EngineEvent::IndexReady);
    }
}
