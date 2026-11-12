//! The `devnotes` binary started again as a script host, as the application starts it.

use std::path::Path;
use std::time::Duration;

use engine::http::scripts::model::{Job, Phase, Script, ScriptKind, ScriptLevel};
use engine::http::scripts::network::Network;
use engine::http::scripts::process::ScriptProcess;
use engine::http::transport::Transport;

#[test]
fn the_binary_relaunched_as_a_script_host_runs_a_script() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let mut network = Network::new(runtime.handle().clone(), Transport::default(), None, true);
    let mut host = ScriptProcess::start_at(Path::new(env!("CARGO_BIN_EXE_devnotes"))).unwrap();
    let job = Job {
        phase: Phase::Before,
        scripts: vec![Script {
            id: "request:script".to_string(),
            level: ScriptLevel::Request,
            kind: ScriptKind::Before,
            name: "Greeting".to_string(),
            owner: None,
            source: "pm.variables.set('greeting', 'bonjour');\
                     env.set('answered', pm.variables.get('greeting'));"
                .to_string(),
        }],
        environment: Some("Staging".to_string()),
        ..Job::default()
    };

    let outcome = host.run(&job, Duration::from_secs(10), &mut network);

    assert_eq!(outcome.failure, None);
    let written: Vec<(&str, Option<&str>)> = outcome
        .writes
        .iter()
        .map(|write| (write.name.as_str(), write.value.as_deref()))
        .collect();
    assert_eq!(written, [("answered", Some("bonjour"))]);
}
