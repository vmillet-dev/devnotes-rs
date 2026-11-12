//! « Présenter »: the draft on screen cut into slides by the core, then shown over everything.
//! The window is made full screen while it shows, and given back only if it was taken.

use egui::{Context, ViewportCommand};
use engine::notes::model::{MapNode, Presentation};
use i18n::keys;
use richtext::Document;

use super::ErrorNotifier;
use crate::api;
use crate::backend::{Backend, Pending};

/// A Note being presented: its title, each slide read, and the map of its headings.
pub struct Presenting {
    pub title: String,
    pub slides: Vec<Document>,
    pub map: Vec<MapNode>,
    /// Counts the presentations: the view starts over on a new one.
    pub session: u64,
}

#[derive(Default)]
pub struct PresentationState {
    asked: Option<(String, Pending<Presentation>)>,
    shown: Option<Presenting>,
    took_screen: bool,
    sessions: u64,
}

impl PresentationState {
    pub fn ask(&mut self, backend: &Backend, title: String, markdown: String) {
        self.asked = Some((title, api::notes::present_note(backend, markdown)));
    }

    pub fn shown(&self) -> Option<&Presenting> {
        self.shown.as_ref()
    }

    /// Once a frame: the slides cut are shown, full screen.
    pub fn update(&mut self, ctx: &Context, errors: &mut ErrorNotifier) {
        let Some((_, pending)) = &mut self.asked else {
            return;
        };
        let Some(answer) = pending.poll() else {
            return;
        };
        let Some((title, _)) = self.asked.take() else {
            return;
        };
        match answer {
            Ok(presentation) => self.show(ctx, title, presentation),
            Err(error) => errors.report(keys::errors::PRESENT_FAILED, &error),
        }
    }

    fn show(&mut self, ctx: &Context, title: String, presentation: Presentation) {
        if self.shown.is_none() {
            let full = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
            self.took_screen = !full;
            if self.took_screen {
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
            }
        }
        self.sessions += 1;
        self.shown = Some(Presenting {
            title,
            slides: presentation
                .slides
                .iter()
                .map(|slide| richtext::read(slide))
                .collect(),
            map: presentation.map,
            session: self.sessions,
        });
    }

    /// Left: the screen goes back as it was.
    pub fn close(&mut self, ctx: &Context) {
        if self.shown.take().is_some() && std::mem::take(&mut self.took_screen) {
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        }
        self.asked = None;
    }
}

#[cfg(test)]
mod tests {
    use egui::{ViewportId, ViewportInfo};

    use super::*;

    fn presentation() -> Presentation {
        Presentation {
            slides: vec!["## One".to_owned()],
            map: Vec::new(),
        }
    }

    /// The commands a frame sent the window, its full screen as `full`.
    fn sent(full: bool, mut frame: impl FnMut(&Context)) -> Vec<ViewportCommand> {
        let mut harness = crate::testing::harness().build_ui(|_| {});
        harness.input_mut().viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                fullscreen: Some(full),
                ..ViewportInfo::default()
            },
        );
        harness.step();
        frame(&harness.ctx);
        harness.step();
        harness.output().viewport_output[&ViewportId::ROOT]
            .commands
            .clone()
    }

    #[test]
    fn full_screen_is_taken_then_given_back() {
        let mut state = PresentationState::default();

        let taken = sent(false, |ctx| state.show(ctx, "T".to_owned(), presentation()));
        let given = sent(true, |ctx| state.close(ctx));

        assert_eq!(taken, [ViewportCommand::Fullscreen(true)]);
        assert_eq!(given, [ViewportCommand::Fullscreen(false)]);
        assert!(state.shown().is_none());
    }

    #[test]
    fn a_window_already_full_screen_stays_so() {
        let mut state = PresentationState::default();

        let taken = sent(true, |ctx| state.show(ctx, "T".to_owned(), presentation()));
        let given = sent(true, |ctx| state.close(ctx));

        assert!(taken.is_empty() && given.is_empty());
    }
}
