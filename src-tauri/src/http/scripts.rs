//! JavaScript run around a request — before it leaves, once it is answered — by `QuickJS` in a
//! script host: the application started again, holding no library key and no note, killed at
//! its deadline. A script reaches the network only by asking this process, which sends it with
//! Rust's client.

pub mod child;
pub mod engine;
pub(crate) mod libraries;
pub mod model;
pub mod network;
pub mod process;

/// How long one phase of one request may take, its `pm.sendRequest`s included.
pub const PHASE_TIME: std::time::Duration = std::time::Duration::from_secs(30);
