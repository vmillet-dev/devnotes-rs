//! How a request travels: its timeout, its redirects, whether TLS is checked and cookies kept.
//! A collection and a folder set defaults; a request overrides them, field by field.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Each field unset inherits: from the folder above, the collection, then `Transport::default`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TransportSettings {
    pub timeout_ms: Option<u32>,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<u32>,
    pub verify_tls: Option<bool>,
    /// The library's jar: sent from, and filled by the answer.
    pub use_cookies: Option<bool>,
}

/// The settings resolved, every field decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Transport {
    pub timeout_ms: u32,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub verify_tls: bool,
    pub use_cookies: bool,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            follow_redirects: true,
            max_redirects: 10,
            verify_tls: true,
            use_cookies: true,
        }
    }
}

/// A timeout below this is a slip of the keyboard, not a choice.
const SHORTEST_TIMEOUT_MS: u32 = 100;

impl Transport {
    /// `settings` set over this, field by field.
    #[must_use]
    pub fn over(self, settings: &TransportSettings) -> Self {
        Self {
            timeout_ms: settings
                .timeout_ms
                .map_or(self.timeout_ms, |timeout| timeout.max(SHORTEST_TIMEOUT_MS)),
            follow_redirects: settings.follow_redirects.unwrap_or(self.follow_redirects),
            max_redirects: settings.max_redirects.unwrap_or(self.max_redirects),
            verify_tls: settings.verify_tls.unwrap_or(self.verify_tls),
            use_cookies: settings.use_cookies.unwrap_or(self.use_cookies),
        }
    }
}

/// From the collection down: the nearest set wins.
pub fn resolve<'a>(levels: impl IntoIterator<Item = &'a TransportSettings>) -> Transport {
    levels
        .into_iter()
        .fold(Transport::default(), |transport, settings| {
            transport.over(settings)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_setting_wins_field_by_field_over_the_defaults() {
        let collection = TransportSettings {
            timeout_ms: Some(5_000),
            verify_tls: Some(false),
            ..TransportSettings::default()
        };
        let folder = TransportSettings {
            timeout_ms: Some(2_000),
            max_redirects: Some(3),
            ..TransportSettings::default()
        };
        let request = TransportSettings {
            use_cookies: Some(false),
            timeout_ms: Some(10),
            ..TransportSettings::default()
        };

        assert_eq!(
            resolve([&collection, &folder, &request]),
            Transport {
                timeout_ms: 100,
                follow_redirects: true,
                max_redirects: 3,
                verify_tls: false,
                use_cookies: false,
            }
        );
        assert_eq!(resolve([]), Transport::default());
    }
}
