//! What the status toast says: the last thing that happened. Every operation on a library
//! reports, even when nothing changed.

use i18n::Message;

/// The toast's message, numbered: the same words said twice are two reports.
#[derive(Debug, Default)]
pub struct StatusNotifier {
    latest: Option<(u64, Message)>,
    said: u64,
}

impl StatusNotifier {
    pub fn report(&mut self, message: Message) {
        self.said += 1;
        self.latest = Some((self.said, message));
    }

    /// The last report and its number.
    pub fn latest(&self) -> Option<(u64, &Message)> {
        self.latest.as_ref().map(|(said, message)| (*said, message))
    }

    pub fn dismiss(&mut self) {
        self.latest = None;
    }
}

#[cfg(test)]
mod tests {
    use i18n::keys;

    use super::*;

    #[test]
    fn every_report_counts_even_the_same_words() {
        let mut status = StatusNotifier::default();
        let saved = Message::new(keys::errors::RETRY);

        status.report(saved.clone());
        let first = status.latest().unwrap().0;
        status.report(saved);

        assert_ne!(status.latest().unwrap().0, first);
    }
}
