//! What a request takes from the folders and the collection above it, and what its body implies.

use super::transport::{self, Limits, SetBy, Transport, TransportSettings};
use serde::Serialize;
use specta::Type;

use super::model::{
    ContainerSettings, HttpItemKind, KeyValue, RequestAuth, RequestBody, RequestScripts,
};
use super::variables::{VariableLayer, VariableScope};

/// Where an inherited value was set: what the request says it comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HttpOrigin {
    pub kind: HttpItemKind,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InheritedHeader {
    pub header: KeyValue,
    pub from: HttpOrigin,
}

/// A folder's or the collection's scripts, and whose they are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InheritedScripts {
    pub from: HttpOrigin,
    pub scripts: RequestScripts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Inherited {
    /// Never `Inherit`: what a request inheriting gets, `None` when nothing above sets one.
    pub auth: RequestAuth,
    pub auth_from: Option<HttpOrigin>,
    /// Enabled and named, one a name: a folder's overrides its collection's.
    pub headers: Vec<InheritedHeader>,
    /// Resolved from the collection down; a request's own settings go over it.
    pub transport: Transport,
    /// Which level set the timeout and the redirects: Rust's, for the failure that names it.
    #[serde(skip)]
    pub limits: Limits,
    /// A level holds a `User-Agent` row switched off: the application's own is not sent in
    /// its place.
    #[serde(skip)]
    pub user_agent_off: bool,
    /// The folders' variables, the nearest first, then the collection's: the front asks for
    /// them resolved, never as layers.
    #[serde(skip)]
    pub variables: Vec<VariableLayer>,
    /// The collection's, then each folder's down to the request's: the order they run in
    /// before the send, the other way after it.
    pub scripts: Vec<InheritedScripts>,
}

/// `levels` from the collection down to the request's own folder: the nearest auth set wins.
pub fn inherit(levels: &[(HttpOrigin, ContainerSettings)]) -> Inherited {
    let (auth, auth_from) = levels
        .iter()
        .rev()
        .find(|(_, settings)| settings.auth != RequestAuth::Inherit)
        .map_or((RequestAuth::None, None), |(origin, settings)| {
            (settings.auth.clone(), Some(origin.clone()))
        });

    let mut headers: Vec<InheritedHeader> = Vec::new();
    for (origin, settings) in levels {
        for header in settings
            .headers
            .iter()
            .filter(|header| header.enabled && !header.key.trim().is_empty())
        {
            let inherited = InheritedHeader {
                header: header.clone(),
                from: origin.clone(),
            };
            match headers
                .iter_mut()
                .find(|known| known.header.key.eq_ignore_ascii_case(&header.key))
            {
                Some(known) => *known = inherited,
                None => headers.push(inherited),
            }
        }
    }

    let set_by = |sets: fn(&TransportSettings) -> bool| {
        levels
            .iter()
            .rev()
            .find(|(_, settings)| sets(&settings.transport))
            .map_or(SetBy::Default, |(origin, _)| SetBy::Above(origin.clone()))
    };

    Inherited {
        auth,
        auth_from,
        headers,
        transport: transport::resolve(levels.iter().map(|(_, settings)| &settings.transport)),
        limits: Limits {
            timeout: set_by(|transport| transport.timeout_ms.is_some()),
            redirects: set_by(|transport| transport.max_redirects.is_some()),
        },
        user_agent_off: levels
            .iter()
            .any(|(_, settings)| switches_off(&settings.headers, "user-agent")),
        variables: levels
            .iter()
            .rev()
            .map(|(origin, settings)| VariableLayer {
                scope: match origin.kind {
                    HttpItemKind::Collection => VariableScope::Collection,
                    HttpItemKind::Folder | HttpItemKind::Request => VariableScope::Folder,
                },
                from: origin.name.clone(),
                variables: settings.variables.clone(),
            })
            .collect(),
        scripts: levels
            .iter()
            .map(|(origin, settings)| InheritedScripts {
                from: origin.clone(),
                scripts: settings.scripts.clone(),
            })
            .collect(),
    }
}

/// Whether a row of that name is written and switched off.
pub fn switches_off(rows: &[KeyValue], name: &str) -> bool {
    rows.iter()
        .any(|row| !row.enabled && row.key.trim().eq_ignore_ascii_case(name))
}

/// The `Content-Type` a body implies, sent unless the request sets one by hand. A multipart one
/// gets its boundary when it is sent.
pub fn implied_content_type(body: &RequestBody) -> Option<&'static str> {
    match body {
        RequestBody::None => None,
        RequestBody::Json { .. } => Some("application/json"),
        RequestBody::Text { .. } => Some("text/plain"),
        RequestBody::Form { .. } => Some("application/x-www-form-urlencoded"),
        RequestBody::Multipart { .. } => Some("multipart/form-data"),
        RequestBody::Binary { .. } => Some("application/octet-stream"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(kind: HttpItemKind, name: &str) -> HttpOrigin {
        HttpOrigin {
            kind,
            id: name.to_lowercase(),
            name: name.to_string(),
        }
    }

    fn header(key: &str, value: &str, enabled: bool) -> KeyValue {
        KeyValue {
            enabled,
            key: key.to_string(),
            value: value.to_string(),
            description: String::new(),
        }
    }

    #[test]
    fn the_nearest_auth_set_wins_and_none_set_is_no_auth() {
        let bearer = RequestAuth::Bearer {
            token: "{{accessToken}}".to_string(),
        };
        let levels = vec![
            (
                origin(HttpItemKind::Collection, "API Paiements"),
                ContainerSettings {
                    auth: bearer.clone(),
                    headers: Vec::new(),
                    ..ContainerSettings::default()
                },
            ),
            (
                origin(HttpItemKind::Folder, "Factures"),
                ContainerSettings::default(),
            ),
        ];

        let inherited = inherit(&levels);
        assert_eq!(inherited.auth, bearer);
        assert_eq!(
            inherited.auth_from.map(|from| from.name),
            Some("API Paiements".to_string())
        );

        let nothing = inherit(&[(
            origin(HttpItemKind::Collection, "Vide"),
            ContainerSettings::default(),
        )]);
        assert_eq!((nothing.auth, nothing.auth_from), (RequestAuth::None, None));
    }

    #[test]
    fn a_folder_header_overrides_its_collection_one_whatever_its_case() {
        let levels = vec![
            (
                origin(HttpItemKind::Collection, "API"),
                ContainerSettings {
                    auth: RequestAuth::Inherit,
                    headers: vec![
                        header("Accept", "application/json", true),
                        header("X-Trace", "1", false),
                        header("X-Client", "web", true),
                    ],
                    ..ContainerSettings::default()
                },
            ),
            (
                origin(HttpItemKind::Folder, "Factures"),
                ContainerSettings {
                    auth: RequestAuth::Inherit,
                    headers: vec![header("accept", "text/csv", true)],
                    ..ContainerSettings::default()
                },
            ),
        ];

        let inherited = inherit(&levels);
        let seen: Vec<(&str, &str, &str)> = inherited
            .headers
            .iter()
            .map(|known| {
                (
                    known.header.key.as_str(),
                    known.header.value.as_str(),
                    known.from.name.as_str(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                ("accept", "text/csv", "Factures"),
                ("X-Client", "web", "API")
            ]
        );
    }

    /// What a timeout's sentence names: the nearest level that set the value, not the first.
    #[test]
    fn a_limit_is_set_by_the_nearest_level_that_sets_it_or_by_nobody() {
        let sets = |timeout_ms, max_redirects| ContainerSettings {
            transport: TransportSettings {
                timeout_ms,
                max_redirects,
                ..TransportSettings::default()
            },
            ..ContainerSettings::default()
        };
        let api = origin(HttpItemKind::Collection, "API");
        let auth = origin(HttpItemKind::Folder, "Auth");
        let levels = vec![
            (api.clone(), sets(Some(5_000), Some(3))),
            (auth.clone(), sets(Some(2_000), None)),
        ];

        let inherited = inherit(&levels);

        assert_eq!(inherited.limits.timeout, SetBy::Above(auth));
        assert_eq!(inherited.limits.redirects, SetBy::Above(api));
        assert_eq!(inherit(&[]).limits, Limits::default());
    }

    /// Off anywhere above, and no level sets another: none is sent, not the application's.
    #[test]
    fn a_user_agent_row_switched_off_at_any_level_is_remembered() {
        let holds = |rows: Vec<KeyValue>| ContainerSettings {
            headers: rows,
            ..ContainerSettings::default()
        };
        let api = origin(HttpItemKind::Collection, "API");
        let auth = origin(HttpItemKind::Folder, "Auth");

        let off = inherit(&[
            (api.clone(), holds(vec![header(" user-agent ", "x", false)])),
            (auth.clone(), holds(Vec::new())),
        ]);
        assert!(off.user_agent_off);
        assert!(off.headers.is_empty());

        let on = inherit(&[
            (api, holds(vec![header("User-Agent", "Équipe/1", true)])),
            (auth, holds(vec![header("X-Debug", "1", false)])),
        ]);
        assert!(!on.user_agent_off);
    }

    #[test]
    fn a_body_implies_its_content_type() {
        assert_eq!(implied_content_type(&RequestBody::None), None);
        assert_eq!(
            implied_content_type(&RequestBody::Json {
                text: "{}".to_string()
            }),
            Some("application/json")
        );
        assert_eq!(
            implied_content_type(&RequestBody::Form { fields: Vec::new() }),
            Some("application/x-www-form-urlencoded")
        );
    }
}
