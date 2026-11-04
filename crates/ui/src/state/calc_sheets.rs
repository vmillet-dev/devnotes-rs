//! The sheet open, computed off the frame. The last answer stays until the next one replaces it:
//! nothing jumps while typing.

use chrono::NaiveDate;
use engine::notes::calc::CalcSheet;
use engine::notes::model::DisplayNote;
use i18n::keys;

use crate::api;
use crate::backend::{Backend, Pending};
use crate::services::errors::ErrorNotifier;

#[derive(Default)]
pub struct CalcSheets {
    asked: Option<(u64, Pending<CalcSheet>)>,
    answer: Option<(u64, CalcSheet)>,
    copying: Vec<(DisplayNote, Pending<CalcSheet>)>,
}

impl CalcSheets {
    /// The sheet of the opening `session`, as last computed.
    pub fn sheet(&self, session: u64) -> Option<&CalcSheet> {
        let (of, sheet) = self.answer.as_ref()?;
        (*of == session).then_some(sheet)
    }

    /// A newer text takes the place of the one on its way.
    pub fn ask(&mut self, backend: &Backend, session: u64, text: String, today: NaiveDate) {
        self.asked = Some((session, api::notes::evaluate_calc(backend, text, today)));
    }

    /// A copy carries the results: read whole, the note is computed before it reaches the clipboard.
    pub fn copy(&mut self, backend: &Backend, note: DisplayNote, today: NaiveDate) {
        let sheet = api::notes::evaluate_calc(backend, note.content.clone(), today);
        self.copying.push((note, sheet));
    }

    /// The notes computed for a copy, each with its sheet.
    pub fn update(&mut self, errors: &mut ErrorNotifier) -> Vec<(DisplayNote, CalcSheet)> {
        self.adopt(errors);
        let mut copied = Vec::new();
        let copying = std::mem::take(&mut self.copying);
        for (note, mut pending) in copying {
            match pending.poll() {
                None => self.copying.push((note, pending)),
                Some(Ok(sheet)) => copied.push((note, sheet)),
                Some(Err(error)) => errors.report(keys::errors::CALC_FAILED, &error),
            }
        }
        copied
    }

    fn adopt(&mut self, errors: &mut ErrorNotifier) {
        let Some((session, pending)) = &mut self.asked else {
            return;
        };
        let session = *session;
        let Some(answer) = pending.poll() else {
            return;
        };
        self.asked = None;
        match answer {
            Ok(sheet) => self.answer = Some((session, sheet)),
            Err(error) => errors.report(keys::errors::CALC_FAILED, &error),
        }
    }
}
