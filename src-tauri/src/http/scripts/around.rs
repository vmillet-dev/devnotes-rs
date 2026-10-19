//! The scripts of one send: which run and in which order, the variables they read, and what
//! their writes do to the levels the request resolves from and to the tables that hold them.

use super::model::{
    LayerScope, LogEntry, Script, ScriptKind, ScriptLayer, ScriptLevel, VariableWrite, WriteScope,
};
use crate::http::environments::{Variable, VariableKind, variable_faults};
use crate::http::model::{HttpItemKind, RequestScripts};
use crate::http::settings::Inherited;
use crate::http::variables::{self, VariableLayer, VariableScope};

/// What one send runs: before it leaves the collection's script, each folder's, then the
/// request's; once it is answered, the other way round. A blank script is not run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Around {
    pub before: Vec<Script>,
    pub after: Vec<Script>,
}

struct Holder<'a> {
    level: ScriptLevel,
    owner: Option<&'a str>,
    name: &'a str,
    scripts: &'a RequestScripts,
}

impl Holder<'_> {
    fn script(&self, kind: ScriptKind) -> Option<Script> {
        let source = match kind {
            ScriptKind::Before => &self.scripts.before,
            ScriptKind::After | ScriptKind::Tests => &self.scripts.after,
        };
        (!source.trim().is_empty()).then(|| Script {
            id: format!("{}:{}:{kind}", self.level, self.owner.unwrap_or("request")),
            level: self.level,
            kind,
            name: self.name.to_string(),
            owner: self.owner.map(str::to_string),
            source: source.clone(),
        })
    }
}

impl Around {
    #[must_use]
    pub fn of(own: &RequestScripts, request: &str, inherited: &Inherited) -> Self {
        let mut levels: Vec<Holder<'_>> = inherited
            .scripts
            .iter()
            .map(|above| Holder {
                level: match above.from.kind {
                    HttpItemKind::Collection => ScriptLevel::Collection,
                    HttpItemKind::Folder | HttpItemKind::Request => ScriptLevel::Folder,
                },
                owner: Some(above.from.id.as_str()),
                name: above.from.name.as_str(),
                scripts: &above.scripts,
            })
            .collect();
        levels.push(Holder {
            level: ScriptLevel::Request,
            owner: None,
            name: request,
            scripts: own,
        });
        Self {
            before: levels
                .iter()
                .filter_map(|level| level.script(ScriptKind::Before))
                .collect(),
            after: levels
                .iter()
                .rev()
                .filter_map(|level| level.script(ScriptKind::After))
                .collect(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.after.is_empty()
    }
}

/// The levels a script reads, as the send resolves them: a row switched off or one its table
/// marks is left out, the first of a name wins.
#[must_use]
pub fn layers_of(layers: &[VariableLayer]) -> Vec<ScriptLayer> {
    layers
        .iter()
        .filter_map(|layer| {
            let scope = match layer.scope {
                VariableScope::Request => LayerScope::Request,
                VariableScope::Folder => LayerScope::Folder,
                VariableScope::Collection => LayerScope::Collection,
                VariableScope::Environment => LayerScope::Environment,
                VariableScope::Globals => LayerScope::Globals,
                VariableScope::Dynamic => return None,
            };
            let mut values = std::collections::BTreeMap::new();
            for (variable, fault) in layer
                .variables
                .iter()
                .zip(variable_faults(&layer.variables))
            {
                let name = variable.name.trim();
                if variable.enabled && !name.is_empty() && fault.is_none() {
                    values
                        .entry(name.to_string())
                        .or_insert_with(|| variable.value.clone());
                }
            }
            Some(ScriptLayer {
                scope,
                from: layer.from.clone(),
                values,
            })
        })
        .collect()
}

/// A table after the writes made to it, in their order: a set changes the value of the row of
/// that name — a secret stays one, and `env.secret` makes one — or adds a row; an unset removes
/// the rows of that name.
#[must_use]
pub fn written<'a>(
    table: &[Variable],
    writes: impl IntoIterator<Item = &'a VariableWrite>,
) -> Vec<Variable> {
    let mut table = table.to_vec();
    for write in writes {
        let name = write.name.trim();
        let Some(value) = &write.value else {
            table.retain(|variable| variable.name.trim() != name);
            continue;
        };
        match table
            .iter_mut()
            .find(|variable| variable.name.trim() == name)
        {
            Some(variable) => {
                variable.value.clone_from(value);
                if write.secret {
                    variable.kind = VariableKind::Secret;
                }
            }
            None => table.push(Variable {
                enabled: true,
                name: name.to_string(),
                value: value.clone(),
                kind: if write.secret {
                    VariableKind::Secret
                } else {
                    VariableKind::Text
                },
            }),
        }
    }
    table
}

/// The writes a phase made to one table.
pub fn writes_to(
    writes: &[VariableWrite],
    scope: WriteScope,
) -> impl Iterator<Item = &VariableWrite> {
    writes.iter().filter(move |write| write.scope == scope)
}

/// The levels in memory as the phase left them: the send that follows resolves from them.
pub fn apply(layers: &mut [VariableLayer], writes: &[VariableWrite]) {
    for (scope, level) in [
        (WriteScope::Environment, VariableScope::Environment),
        (WriteScope::Globals, VariableScope::Globals),
        (WriteScope::Collection, VariableScope::Collection),
    ] {
        if let Some(layer) = layers.iter_mut().find(|layer| layer.scope == level) {
            layer.variables = written(&layer.variables, writes_to(writes, scope));
        }
    }
}

/// `pm.variables` as the phase before left them: nearer than the request's own rows.
#[must_use]
pub fn locals_layer(locals: &std::collections::BTreeMap<String, String>) -> VariableLayer {
    VariableLayer {
        scope: VariableScope::Request,
        from: String::new(),
        variables: locals
            .iter()
            .map(|(name, value)| Variable {
                enabled: true,
                name: name.clone(),
                value: value.clone(),
                kind: VariableKind::Text,
            })
            .collect(),
    }
}

/// What a script logged, each secret in force written as its `{{name}}`.
#[must_use]
pub fn masked(logs: Vec<LogEntry>, secrets: &[(String, String)]) -> Vec<LogEntry> {
    logs.into_iter()
        .map(|entry| LogEntry {
            text: variables::hide(&entry.text, secrets),
            ..entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::model::ContainerSettings;
    use crate::http::settings::{self, HttpOrigin};

    fn scripts(before: &str, after: &str) -> RequestScripts {
        RequestScripts {
            before: before.to_string(),
            after: after.to_string(),
        }
    }

    fn origin(kind: HttpItemKind, id: &str, name: &str) -> HttpOrigin {
        HttpOrigin {
            kind,
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    fn set(scope: WriteScope, name: &str, value: Option<&str>, secret: bool) -> VariableWrite {
        VariableWrite {
            scope,
            name: name.to_string(),
            value: value.map(str::to_string),
            secret,
        }
    }

    #[test]
    fn the_scripts_run_from_the_collection_down_before_and_back_up_after() {
        let inherited = settings::inherit(&[
            (
                origin(HttpItemKind::Collection, "c1", "API Paiements"),
                ContainerSettings {
                    scripts: scripts("collection()", "collection()"),
                    ..ContainerSettings::default()
                },
            ),
            (
                origin(HttpItemKind::Folder, "f1", "Factures"),
                ContainerSettings {
                    scripts: scripts("  \n", "folder()"),
                    ..ContainerSettings::default()
                },
            ),
        ]);
        let around = Around::of(&scripts("request()", ""), "Payer", &inherited);
        let order = |list: &[Script]| -> Vec<String> {
            list.iter()
                .map(|script| format!("{} {}", script.id, script.name))
                .collect()
        };
        assert_eq!(
            order(&around.before),
            [
                "collection:c1:before API Paiements",
                "request:request:before Payer"
            ]
        );
        assert_eq!(
            order(&around.after),
            [
                "folder:f1:after Factures",
                "collection:c1:after API Paiements"
            ]
        );
        assert_eq!(around.before[0].owner.as_deref(), Some("c1"));
        assert!(Around::of(&RequestScripts::default(), "x", &settings::inherit(&[])).is_empty());
    }

    #[test]
    fn a_write_changes_its_row_keeps_a_secret_secret_adds_or_removes() {
        let table = vec![
            Variable {
                name: "token".to_string(),
                value: "old".to_string(),
                kind: VariableKind::Secret,
                ..Variable::default()
            },
            Variable {
                name: "gone".to_string(),
                value: "1".to_string(),
                ..Variable::default()
            },
        ];
        let writes = [
            set(WriteScope::Environment, "token", Some("new"), false),
            set(WriteScope::Environment, "gone", None, false),
            set(WriteScope::Environment, "key", Some("k"), true),
            set(WriteScope::Environment, "page", Some("2"), false),
        ];
        let table = written(&table, &writes);
        let rows: Vec<(&str, &str, VariableKind)> = table
            .iter()
            .map(|variable| {
                (
                    variable.name.as_str(),
                    variable.value.as_str(),
                    variable.kind,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("token", "new", VariableKind::Secret),
                ("key", "k", VariableKind::Secret),
                ("page", "2", VariableKind::Text),
            ]
        );
    }

    #[test]
    fn what_a_phase_wrote_is_what_the_send_resolves_from() {
        let mut layers = vec![
            VariableLayer {
                scope: VariableScope::Environment,
                from: "Staging".to_string(),
                variables: Vec::new(),
            },
            VariableLayer {
                scope: VariableScope::Globals,
                from: String::new(),
                variables: Vec::new(),
            },
        ];
        apply(
            &mut layers,
            &[
                set(WriteScope::Environment, "token", Some("abc"), true),
                set(WriteScope::Globals, "tenant", Some("acme"), false),
                set(WriteScope::Collection, "nowhere", Some("1"), false),
            ],
        );
        let resolver = variables::Resolver::new(&layers);
        assert_eq!(resolver.fill("{{token}}/{{tenant}}"), "abc/acme");
        assert_eq!(
            resolver.secrets(),
            [("token".to_string(), "abc".to_string())]
        );

        let scripted = layers_of(&layers);
        assert_eq!(scripted[0].scope, LayerScope::Environment);
        assert_eq!(scripted[0].values["token"], "abc");
    }

    #[test]
    fn a_log_never_shows_a_secret_in_force() {
        let logs = masked(
            vec![LogEntry {
                text: "Bearer s3cr3t-value".to_string(),
                ..LogEntry::default()
            }],
            &[("token".to_string(), "s3cr3t-value".to_string())],
        );
        assert_eq!(logs[0].text, "Bearer {{token}}");
    }
}
