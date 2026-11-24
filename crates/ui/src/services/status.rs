//! What was done. Every operation on a library reports, even when nothing changed.

use i18n::Message;

use super::notices::Notices;

#[derive(Debug, Default)]
pub struct StatusNotifier {
    notices: Notices,
}

impl StatusNotifier {
    pub fn report(&mut self, message: Message) {
        self.notices.push(message);
    }

    /// The last report and its number.
    pub fn latest(&self) -> Option<(u64, &Message)> {
        self.shown().next()
    }

    /// Those still shown, the latest first.
    pub fn shown(&self) -> impl Iterator<Item = (u64, &Message)> {
        self.notices.shown()
    }

    pub fn dismiss(&mut self, number: u64) {
        self.notices.dismiss(number);
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
        assert_eq!(status.shown().count(), 2);
    }
}
