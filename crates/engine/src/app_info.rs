//! What Cargo has no field for comes from `[package.metadata.devnotes]` and
//! `rust-toolchain.toml`, read by `build.rs`: Cargo does not pass `[package.metadata]` to the
//! crate. No version here: the front asks the running binary, which cannot go stale.

use serde::Serialize;

/// A constant rather than a command: the titlebar reads the name synchronously.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadata {
    /// What the user is shown, which is not the crate name (`devnotes`).
    pub name: &'static str,
    /// Opened by the about card, as an `https` link.
    pub repository: &'static str,
    pub author: &'static str,
    pub author_handle: &'static str,
    /// The toolchain the project pins, for the about card.
    pub rust_version: &'static str,
}

pub const METADATA: AppMetadata = AppMetadata {
    name: env!("DEVNOTES_DISPLAY_NAME"),
    repository: env!("CARGO_PKG_REPOSITORY"),
    author: env!("DEVNOTES_AUTHOR"),
    author_handle: env!("DEVNOTES_AUTHOR_HANDLE"),
    rust_version: env!("DEVNOTES_RUST_VERSION"),
};
