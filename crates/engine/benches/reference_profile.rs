//! A reference library (`reference.rs`) written into a real profile, behind a real passphrase:
//! what the interface's bench and a measured process open.
//!
//! ```
//! DEVNOTES_PROFILE=<empty dir> DEVNOTES_CORPUS=realistic cargo bench -p engine --bench reference_profile
//! ```

mod reference;

use std::path::PathBuf;
use std::sync::Arc;

use engine::db::run_here;
use engine::vault::{create_vault, secret};
use engine::{Engine, Paths, RecordingSink};

use reference::Size;

/// A test value, the one the end-to-end driver types.
const PASSPHRASE: &str = "correct horse battery staple";

fn main() {
    let Some(profile) = std::env::var_os("DEVNOTES_PROFILE").map(PathBuf::from) else {
        eprintln!("DEVNOTES_PROFILE names the profile to write");
        return;
    };
    let size = match std::env::var("DEVNOTES_CORPUS").as_deref() {
        Ok("stress") => Size::Stress,
        _ => Size::Realistic,
    };
    std::fs::create_dir_all(&profile).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let paths = Paths {
        resources: profile.clone(),
        profile,
    };
    let engine = Arc::new(Engine::new(
        paths,
        Arc::new(RecordingSink::default()),
        runtime.handle().clone(),
    ));
    run_here(&engine, |e| create_vault(e, secret(PASSPHRASE.to_owned()))).unwrap();
    let (_, note_ids) = run_here(&engine, |e| {
        let mut library = e.db().lock().unwrap();
        reference::seed(library.as_mut().unwrap(), size)
    });
    reference::attach(&engine, &note_ids);
    println!("{} notes written", note_ids.len());
}
