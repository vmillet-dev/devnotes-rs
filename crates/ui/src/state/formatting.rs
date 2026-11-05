//! Prettier's answers, computed off the frame: the one the editor asked for, which it applies
//! through its own history, and the bodies formatted on their way to the core.

use text::format::{Answer, Request};

use crate::backend::{Backend, Pending};

/// An answer for the opening `session`; `seq` tells the editor it has not applied it yet.
pub struct Answered {
    pub session: u64,
    pub seq: u64,
    pub answer: Answer,
    /// Formatted on save: no notice, and nothing to undo but the field's own step.
    pub quiet: bool,
}

/// A body held back until it is formatted: it is written once, not typed then formatted.
struct Saving {
    session: u64,
    id: String,
    typed: String,
    pending: Pending<Answer>,
}

/// What a body formatted on save comes back as.
pub struct Saved {
    pub session: u64,
    pub id: String,
    pub body: String,
}

#[derive(Default)]
pub struct FormattingState {
    asked: Option<(u64, Pending<Answer>)>,
    answered: Option<Answered>,
    seq: u64,
    saving: Vec<Saving>,
}

impl FormattingState {
    pub fn ask(&mut self, backend: &Backend, session: u64, request: Request) {
        self.asked = Some((session, format(backend, request)));
    }

    pub fn busy(&self, session: u64) -> bool {
        self.asked
            .as_ref()
            .is_some_and(|(asked, _)| *asked == session)
    }

    pub fn answered(&self, session: u64) -> Option<&Answered> {
        self.answered
            .as_ref()
            .filter(|each| each.session == session)
    }

    /// A body to format before it is written, by `id` if the editor is closed by then.
    pub fn save(&mut self, backend: &Backend, session: u64, id: String, request: Request) {
        self.saving.push(Saving {
            session,
            id,
            typed: request.text.clone(),
            pending: format(backend, request),
        });
    }

    /// Once a frame: the answer asked for, and the bodies formatted on save, to be written.
    pub fn update(&mut self) -> Vec<Saved> {
        if let Some((session, pending)) = &mut self.asked
            && let Some(answer) = pending.poll()
        {
            let session = *session;
            self.asked = None;
            self.answer(session, answer.unwrap_or(Answer::Failed), false);
        }
        let mut saved = Vec::new();
        for mut saving in std::mem::take(&mut self.saving) {
            match saving.pending.poll() {
                None => self.saving.push(saving),
                Some(answer) => {
                    let body = match answer {
                        Ok(Answer::Formatted {
                            text,
                            cursor,
                            changed_lines,
                        }) => {
                            let answer = Answer::Formatted {
                                text: text.clone(),
                                cursor,
                                changed_lines,
                            };
                            self.answer(saving.session, answer, true);
                            text
                        }
                        // Refused or unchanged, the body is written as it was typed.
                        _ => saving.typed,
                    };
                    saved.push(Saved {
                        session: saving.session,
                        id: saving.id,
                        body,
                    });
                }
            }
        }
        saved
    }

    fn answer(&mut self, session: u64, answer: Answer, quiet: bool) {
        self.seq += 1;
        self.answered = Some(Answered {
            session,
            seq: self.seq,
            answer,
            quiet,
        });
    }
}

fn format(backend: &Backend, request: Request) -> Pending<Answer> {
    backend.compute(move |_| Ok(text::format::format(&request)))
}
