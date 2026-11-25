//! What the toasts of one kind say: the last things, each until it is dismissed.

use std::collections::VecDeque;

use i18n::Message;

/// What stands at once: the oldest makes room for the next.
pub const KEPT: usize = 4;

/// Numbered: the same words said twice are two.
#[derive(Debug, Default)]
pub struct Notices {
    shown: VecDeque<(u64, Message)>,
    said: u64,
}

impl Notices {
    pub fn push(&mut self, message: Message) {
        self.said += 1;
        self.shown.push_front((self.said, message));
        self.shown.truncate(KEPT);
    }

    /// In the place of the same words, already there.
    pub fn push_once(&mut self, message: Message) {
        self.shown.retain(|(_, shown)| *shown != message);
        self.push(message);
    }

    /// Those still shown, the latest first.
    pub fn shown(&self) -> impl Iterator<Item = (u64, &Message)> {
        self.shown.iter().map(|(said, message)| (*said, message))
    }

    pub fn dismiss(&mut self, number: u64) {
        self.shown.retain(|(said, _)| *said != number);
    }
}

#[cfg(test)]
mod tests {
    use i18n::keys;

    use super::*;

    fn numbers(notices: &Notices) -> Vec<u64> {
        notices.shown().map(|(said, _)| said).collect()
    }

    #[test]
    fn the_latest_is_first_the_oldest_makes_room_and_one_is_dismissed_alone() {
        let mut notices = Notices::default();
        for _ in 0..KEPT + 2 {
            notices.push(Message::new(keys::errors::RETRY));
        }
        assert_eq!(numbers(&notices), [6, 5, 4, 3]);
        notices.dismiss(5);
        assert_eq!(numbers(&notices), [6, 4, 3]);
        notices.dismiss(5);
        assert_eq!(numbers(&notices), [6, 4, 3]);
    }

    #[test]
    fn the_same_words_pushed_once_take_the_place_of_the_first() {
        let mut notices = Notices::default();
        notices.push_once(Message::new(keys::errors::RETRY));
        notices.push_once(Message::new(keys::errors::DISMISS));
        notices.push_once(Message::new(keys::errors::RETRY));
        assert_eq!(numbers(&notices), [3, 2]);
    }
}
