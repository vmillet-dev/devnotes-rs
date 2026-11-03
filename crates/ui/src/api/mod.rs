//! The one place that names the core's functions (plan § 3.3.1, rule 3). Each function says once
//! which queue its service runs on; in doubt, `db`. The rest of the interface calls these, and
//! uses the core's model types as they are.
//!
//! A feature's file arrives with the first screen that calls it.

pub mod libraries;
pub mod preferences;
pub mod vault;
