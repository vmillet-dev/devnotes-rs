//! The clock, injected, and the labels that age with it: a relative label asks to be drawn
//! again at the exact moment it changes, never every thirty seconds (plan § 3.4).

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use i18n::Message;

/// What the interface reads the time from: the system's, or a test's.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// « il y a 3 min » for `at`, and how long it says so: the delay until the next frame that has
/// to draw it again.
pub fn relative(at: DateTime<Utc>, clock: &dyn Clock) -> (Message, Duration) {
    let now = clock.now();
    let label = i18n::time::relative(at, now);
    (label.clone(), until_change(&label, at, now))
}

/// The label rounds minutes; it can change only when the rounded count does, half a minute
/// before each whole one. The next of those moments where it reads otherwise, a day at most.
/// An age read later never reads as an earlier one, so the moment is found by halves: a dozen
/// labels written rather than up to a day's minutes, for every card on screen at every frame.
fn until_change(label: &Message, at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    let elapsed = (now - at).num_milliseconds().max(0);
    let minute = 60_000;
    let first = (elapsed + minute / 2) / minute * minute + minute / 2;
    let changed = |step: i64| {
        let then = at + TimeDelta::milliseconds(first + step * minute);
        i18n::time::relative(at, then) != *label
    };
    let (mut same, mut other) = (-1, 24 * 60);
    if !changed(other - 1) {
        return Duration::from_hours(24);
    }
    other -= 1;
    while other - same > 1 {
        let middle = same + (other - same) / 2;
        if changed(middle) {
            other = middle;
        } else {
            same = middle;
        }
    }
    let wait = u64::try_from(first + other * minute - elapsed).unwrap_or(0);
    Duration::from_millis(wait)
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::Mutex;

    use super::*;

    /// A clock a test moves by hand.
    pub(crate) struct FakeClock(Mutex<DateTime<Utc>>);

    impl FakeClock {
        pub(crate) fn at(time: &str) -> Self {
            Self(Mutex::new(time.parse().unwrap()))
        }

        pub(crate) fn advance(&self, by: Duration) {
            let mut now = self.0.lock().unwrap();
            *now += TimeDelta::from_std(by).unwrap();
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.0.lock().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeClock;
    use super::*;

    const AT: &str = "2026-07-25T09:00:00Z";

    #[test]
    fn a_label_is_drawn_again_when_it_changes_and_not_before() {
        let clock = FakeClock::at(AT);
        let at: DateTime<Utc> = AT.parse().unwrap();

        let (label, wait) = relative(at, &clock);
        assert_eq!(label.key, i18n::keys::time::JUST_NOW);
        assert_eq!(wait, Duration::from_secs(30));

        clock.advance(wait);
        let (label, wait) = relative(at, &clock);
        assert_eq!(
            label,
            Message::new(i18n::keys::time::MINUTES_AGO).with("count", 1.0)
        );
        assert_eq!(wait, Duration::from_mins(1));
    }

    #[test]
    fn hours_wait_for_the_hour_that_changes_them() {
        let clock = FakeClock::at("2026-07-25T12:00:00Z");
        let at: DateTime<Utc> = "2026-07-25T09:00:00Z".parse().unwrap();

        let (label, wait) = relative(at, &clock);

        assert_eq!(
            label,
            Message::new(i18n::keys::time::HOURS_AGO).with("count", 3.0)
        );
        // 3 h rounds to 4 h from 3 h 29 min 30 s on.
        assert_eq!(wait, Duration::from_secs(29 * 60 + 30));
    }
}

#[cfg(test)]
mod halves {
    use super::*;

    /// The minutes walked one by one, as before: the answer the halves must give.
    fn walked(label: &Message, at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
        let elapsed = (now - at).num_milliseconds().max(0);
        let minute = 60_000;
        let mut boundary = (elapsed + minute / 2) / minute * minute + minute / 2;
        for _ in 0..24 * 60 {
            let then = at + TimeDelta::milliseconds(boundary);
            if i18n::time::relative(at, then) != *label {
                return Duration::from_millis(u64::try_from(boundary - elapsed).unwrap_or(0));
            }
            boundary += minute;
        }
        Duration::from_hours(24)
    }

    #[test]
    fn the_halves_find_the_moment_the_minutes_did() {
        let now: DateTime<Utc> = "2026-07-25T09:00:00Z".parse().unwrap();
        for seconds in (0..90 * 24 * 3600).step_by(1_337) {
            let at = now - TimeDelta::seconds(seconds);
            let label = i18n::time::relative(at, now);
            assert_eq!(
                until_change(&label, at, now),
                walked(&label, at, now),
                "{seconds} s ago"
            );
        }
    }
}
