//! The URL analyser: what a URL is made of, read the way a browser reads it (WHATWG).

use percent_encoding::percent_decode_str;
use serde::Serialize;
use specta::Type;
use url::{ParseError, Url};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueryParameter {
    pub name: String,
    pub value: String,
}

/// The user and the password decoded, the path as written, the query both ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlParts {
    pub scheme: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub host: Option<String>,
    /// The port written, or the scheme's own when none is.
    pub port: Option<u16>,
    pub port_is_default: bool,
    pub path: String,
    pub query: Option<String>,
    pub parameters: Vec<QueryParameter>,
    pub fragment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UrlProblem {
    Empty,
    /// "example.com/path": a URL starts with its scheme.
    NoScheme,
    EmptyHost,
    InvalidPort,
    InvalidAddress,
    InvalidDomain,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UrlAnswer {
    Parsed { parts: UrlParts },
    Invalid { problem: UrlProblem },
}

fn decoded(text: &str) -> String {
    percent_decode_str(text).decode_utf8_lossy().into_owned()
}

fn problem(error: ParseError) -> UrlProblem {
    match error {
        ParseError::RelativeUrlWithoutBase => UrlProblem::NoScheme,
        ParseError::EmptyHost => UrlProblem::EmptyHost,
        ParseError::InvalidPort => UrlProblem::InvalidPort,
        ParseError::InvalidIpv4Address | ParseError::InvalidIpv6Address => {
            UrlProblem::InvalidAddress
        }
        ParseError::InvalidDomainCharacter | ParseError::IdnaError => UrlProblem::InvalidDomain,
        _ => UrlProblem::Other,
    }
}

pub fn parse_url(text: &str) -> UrlAnswer {
    let text = text.trim();
    if text.is_empty() {
        return UrlAnswer::Invalid {
            problem: UrlProblem::Empty,
        };
    }

    match Url::parse(text) {
        Ok(url) => UrlAnswer::Parsed {
            parts: UrlParts {
                scheme: url.scheme().to_owned(),
                username: Some(url.username())
                    .filter(|name| !name.is_empty())
                    .map(decoded),
                password: url.password().map(decoded),
                host: url.host_str().map(str::to_owned),
                port: url.port_or_known_default(),
                port_is_default: url.port().is_none(),
                path: url.path().to_owned(),
                query: url.query().map(str::to_owned),
                parameters: url
                    .query_pairs()
                    .map(|(name, value)| QueryParameter {
                        name: name.into_owned(),
                        value: value.into_owned(),
                    })
                    .collect(),
                fragment: url.fragment().map(decoded),
            },
        },
        Err(error) => UrlAnswer::Invalid {
            problem: problem(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(text: &str) -> UrlParts {
        match parse_url(text) {
            UrlAnswer::Parsed { parts } => parts,
            UrlAnswer::Invalid { problem } => panic!("{text} was refused: {problem:?}"),
        }
    }

    fn refused(text: &str) -> UrlProblem {
        match parse_url(text) {
            UrlAnswer::Invalid { problem } => problem,
            UrlAnswer::Parsed { .. } => panic!("{text} was accepted"),
        }
    }

    #[test]
    fn a_url_is_taken_apart() {
        let parts = parts(
            "https://ana:s%C3%A9cret@api.exemple.fr:8443/v1/factures?statut=pay%C3%A9e&page=2#bas",
        );

        assert_eq!(parts.scheme, "https");
        assert_eq!(parts.username.as_deref(), Some("ana"));
        assert_eq!(parts.password.as_deref(), Some("sécret"));
        assert_eq!(parts.host.as_deref(), Some("api.exemple.fr"));
        assert_eq!((parts.port, parts.port_is_default), (Some(8443), false));
        assert_eq!(parts.path, "/v1/factures");
        assert_eq!(parts.query.as_deref(), Some("statut=pay%C3%A9e&page=2"));
        assert_eq!(parts.fragment.as_deref(), Some("bas"));
    }

    #[test]
    fn the_parameters_are_decoded_in_order() {
        let parts = parts("https://exemple.fr/?q=caf%C3%A9+cr%C3%A8me&tag=a&tag=b&vide");

        let pairs: Vec<(&str, &str)> = parts
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("q", "café crème"),
                ("tag", "a"),
                ("tag", "b"),
                ("vide", "")
            ]
        );
    }

    #[test]
    fn a_port_left_out_is_the_scheme_own() {
        let parts = parts("http://localhost/");

        assert_eq!((parts.port, parts.port_is_default), (Some(80), true));
        assert_eq!(parts.username, None);
    }

    #[test]
    fn a_refused_url_says_why() {
        assert_eq!(refused(""), UrlProblem::Empty);
        assert_eq!(refused("exemple.fr/chemin"), UrlProblem::NoScheme);
        assert_eq!(
            refused("https://exemple.fr:99999/"),
            UrlProblem::InvalidPort
        );
        assert_eq!(refused("http://[::1"), UrlProblem::InvalidAddress);
        assert_eq!(refused("https://"), UrlProblem::EmptyHost);
    }

    #[test]
    fn an_answer_says_its_kind() {
        let json = serde_json::to_value(parse_url("nope")).unwrap();

        assert_eq!(
            json,
            serde_json::json!({ "kind": "invalid", "problem": "noScheme" })
        );
    }
}
