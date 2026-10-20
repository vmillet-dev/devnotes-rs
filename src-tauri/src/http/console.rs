//! The console under a request: what left and what came back — a script's sends included —,
//! what the scripts logged, the tests that failed and what stopped. Each line crosses once, as
//! an event, a secret in force written as its `{{name}}` and an auth header masked; nothing of
//! it is kept.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::history;
use super::model::KeyValue;
use super::scripts::model::{
    LogEntry, LogLevel, Outcome, Script, ScriptKind, ScriptLevel, ScriptStop, TestResult,
};
use super::scripts::network::ScriptSend;
use super::send::{Exchange, SentResponse};
use super::variables;
use crate::error::{AppError, ErrorCode};

pub const HTTP_CONSOLE_EVENT: &str = "http-console";

/// The script a line comes from: its kind, and the request, folder or collection that holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleOrigin {
    pub kind: ScriptKind,
    pub level: ScriptLevel,
    pub name: String,
}

impl ConsoleOrigin {
    #[must_use]
    pub fn of(id: Option<&str>, scripts: &[Script]) -> Option<Self> {
        let script = scripts
            .iter()
            .find(|script| Some(script.id.as_str()) == id)?;
        Some(Self {
            kind: script.kind,
            level: script.level,
            name: script.name.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConsoleLine {
    /// A request leaving: the send's own, or one a script sent (`by`).
    Out {
        method: String,
        url: String,
        headers: Vec<KeyValue>,
        by: Option<ConsoleOrigin>,
    },
    In {
        status: u16,
        reason: String,
        millis: u32,
        size: u32,
        by: Option<ConsoleOrigin>,
    },
    /// What came back in place of an answer, as an error's code and the values it states.
    Failed {
        code: ErrorCode,
        params: BTreeMap<String, String>,
        by: Option<ConsoleOrigin>,
    },
    Log {
        level: LogLevel,
        text: String,
        by: Option<ConsoleOrigin>,
    },
    /// A test that failed.
    Test {
        name: String,
        expected: Option<String>,
        actual: Option<String>,
        message: Option<String>,
        by: Option<ConsoleOrigin>,
    },
    Stopped {
        stop: ScriptStop,
    },
    /// An extraction whose path led nowhere: its variable was left as it was.
    Missed {
        path: String,
        variable: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleEvent {
    /// The send it belongs to, under the id the front sent it with.
    pub id: String,
    pub at: DateTime<Utc>,
    pub line: ConsoleLine,
}

/// A request as the console shows it: each secret in force written as its name, an auth or a
/// cookie header masked as the history masks it.
#[must_use]
pub fn out(
    exchange: &Exchange,
    secrets: &[(String, String)],
    by: Option<ConsoleOrigin>,
) -> ConsoleLine {
    let shown = history::shown(exchange, secrets);
    ConsoleLine::Out {
        method: shown.method.to_string(),
        url: shown.url,
        headers: shown.headers,
        by,
    }
}

#[must_use]
pub fn answered(response: &SentResponse, by: Option<ConsoleOrigin>) -> ConsoleLine {
    ConsoleLine::In {
        status: response.status,
        reason: response.reason.clone(),
        millis: response.millis,
        size: response.size,
        by,
    }
}

#[must_use]
pub fn failed(error: AppError, by: Option<ConsoleOrigin>) -> ConsoleLine {
    ConsoleLine::Failed {
        code: error.code,
        params: error.params,
        by,
    }
}

/// What a phase leaves in the console, in the order it happened: its logs and its sends, then
/// the tests that failed and what stopped it.
#[must_use]
pub fn phase(
    logs: Vec<LogEntry>,
    sends: Vec<ScriptSend>,
    outcome: &Outcome,
    scripts: &[Script],
    secrets: &[(String, String)],
) -> Vec<(DateTime<Utc>, ConsoleLine)> {
    let now = Utc::now();
    let mut lines: Vec<(DateTime<Utc>, ConsoleLine)> = logs
        .into_iter()
        .map(|entry| {
            (
                entry.at.unwrap_or(now),
                ConsoleLine::Log {
                    level: entry.level,
                    text: variables::hide(&entry.text, secrets),
                    by: ConsoleOrigin::of(entry.script.as_deref(), scripts),
                },
            )
        })
        .collect();
    for send in sends {
        let by = ConsoleOrigin::of(send.script.as_deref(), scripts);
        lines.push((send.started, out(&send.exchange, secrets, by.clone())));
        lines.push((
            send.ended,
            match send.answer {
                Ok(response) => answered(&response, by),
                Err(error) => failed(error.into(), by),
            },
        ));
    }
    // Stable: what was said at the same instant keeps its order.
    lines.sort_by_key(|(at, _)| *at);
    lines.extend(
        outcome
            .tests
            .iter()
            .filter(|test| !test.passed && !test.skipped)
            .map(|test| (now, tested(test, scripts, secrets))),
    );
    if let Some(failure) = &outcome.failure {
        lines.push((
            now,
            ConsoleLine::Stopped {
                stop: ScriptStop::of(failure, scripts),
            },
        ));
    }
    lines
}

fn tested(test: &TestResult, scripts: &[Script], secrets: &[(String, String)]) -> ConsoleLine {
    let hide = |text: &Option<String>| text.as_deref().map(|text| variables::hide(text, secrets));
    ConsoleLine::Test {
        name: variables::hide(&test.name, secrets),
        expected: hide(&test.expected),
        actual: hide(&test.actual),
        message: hide(&test.message),
        by: ConsoleOrigin::of(test.script.as_deref(), scripts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::model::HttpMethod;
    use crate::http::scripts::model::{FailureKind, ScriptFailure};
    use crate::http::send::{SendError, SentBody};

    fn script(id: &str, kind: ScriptKind, level: ScriptLevel, name: &str) -> Script {
        Script {
            id: id.to_string(),
            kind,
            level,
            name: name.to_string(),
            ..Script::default()
        }
    }

    fn exchange(url: &str, headers: &[(&str, &str)]) -> Exchange {
        Exchange {
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(key, value)| KeyValue {
                    key: (*key).to_string(),
                    value: (*value).to_string(),
                    ..KeyValue::default()
                })
                .collect(),
            body: SentBody::None,
        }
    }

    #[test]
    fn a_request_leaves_with_its_secrets_written_as_their_names_and_its_auth_masked() {
        let secrets = [("token".to_string(), "s3cr3t".to_string())];
        let ConsoleLine::Out { url, headers, .. } = out(
            &exchange(
                "https://api.exemple.fr/me?key=s3cr3t",
                &[("Authorization", "Bearer s3cr3t"), ("X-Trace", "s3cr3t-1")],
            ),
            &secrets,
            None,
        ) else {
            panic!("not a request");
        };
        assert_eq!(url, "https://api.exemple.fr/me?key={{token}}");
        assert_eq!(headers[0].value, "Bearer ••••••••");
        assert_eq!(headers[1].value, "{{token}}-1");
    }

    #[test]
    fn a_phase_says_its_logs_and_sends_in_order_then_its_failed_tests_and_its_stop() {
        let scripts = [
            script("s1", ScriptKind::Before, ScriptLevel::Collection, "API"),
            script("s2", ScriptKind::Tests, ScriptLevel::Request, "Payer"),
        ];
        let at = |seconds: i64| DateTime::from_timestamp(1_760_000_000 + seconds, 0).unwrap();
        let logs = vec![LogEntry {
            level: LogLevel::Log,
            text: "token s3cr3t".to_string(),
            script: Some("s1".to_string()),
            at: Some(at(2)),
        }];
        let sends = vec![ScriptSend {
            script: Some("s1".to_string()),
            exchange: exchange("https://auth/login", &[]),
            started: at(1),
            ended: at(3),
            answer: Err(SendError::Refused("down".to_string())),
        }];
        let outcome = Outcome {
            tests: vec![
                TestResult {
                    name: "passes".to_string(),
                    passed: true,
                    ..TestResult::default()
                },
                TestResult {
                    name: "currency".to_string(),
                    expected: Some("\"eur\"".to_string()),
                    actual: Some("\"usd\"".to_string()),
                    script: Some("s2".to_string()),
                    ..TestResult::default()
                },
            ],
            failure: Some(ScriptFailure {
                kind: FailureKind::Error,
                script: Some("s1".to_string()),
                line: Some(4),
                message: "boom".to_string(),
            }),
            ..Outcome::default()
        };
        let secrets = [("token".to_string(), "s3cr3t".to_string())];

        let lines = phase(logs, sends, &outcome, &scripts, &secrets);

        let kinds: Vec<&str> = lines
            .iter()
            .map(|(_, line)| match line {
                ConsoleLine::Out { .. } => "out",
                ConsoleLine::In { .. } => "in",
                ConsoleLine::Failed { .. } => "failed",
                ConsoleLine::Log { .. } => "log",
                ConsoleLine::Test { .. } => "test",
                ConsoleLine::Stopped { .. } => "stopped",
                ConsoleLine::Missed { .. } => "missed",
            })
            .collect();
        assert_eq!(kinds, ["out", "log", "failed", "test", "stopped"]);
        let ConsoleLine::Log { text, by, .. } = &lines[1].1 else {
            panic!("not a log");
        };
        assert_eq!(text, "token {{token}}");
        assert_eq!(
            by.as_ref().map(|by| by.level),
            Some(ScriptLevel::Collection)
        );
        let ConsoleLine::Failed { code, .. } = &lines[2].1 else {
            panic!("not a failure");
        };
        assert_eq!(*code, ErrorCode::HttpRefused);
        let ConsoleLine::Test { by, .. } = &lines[3].1 else {
            panic!("not a test");
        };
        assert_eq!(by.as_ref().map(|by| by.kind), Some(ScriptKind::Tests));
    }
}
