//! The scripts of one send in one host: the phase before, then the phase after, with what the
//! first left to the second — `pm.variables`, the jar's copy and the console.

use super::PHASE_TIME;
use super::model::{Job, LogEntry, Outcome};
use super::network::Network;
use super::process::{Killer, ScriptProcess};

pub struct Session {
    host: ScriptProcess,
    pub network: Network,
    pub logs: Vec<LogEntry>,
}

impl Session {
    #[must_use]
    pub fn new(host: ScriptProcess, network: Network) -> Self {
        Self {
            host,
            network,
            logs: Vec::new(),
        }
    }

    #[must_use]
    pub fn killer(&self) -> Killer {
        self.host.killer()
    }

    /// Blocks until the phase ends: on a thread of its own, never the runtime's.
    pub fn run(&mut self, job: &Job) -> Outcome {
        let outcome = self.host.run(job, PHASE_TIME, &mut self.network);
        self.logs.append(&mut self.network.logs);
        outcome
    }
}
