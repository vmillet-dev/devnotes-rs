//! Handing a file to the program the desktop picks for it.

use std::io;
use std::path::Path;
use std::sync::Arc;

pub trait Opener {
    fn open(&self, path: &Path) -> io::Result<()>;
}

/// The desktop's own: what makes a PDF or an archive usable.
pub struct System;

impl Opener for System {
    fn open(&self, path: &Path) -> io::Result<()> {
        open::that_detached(path)
    }
}

pub fn system() -> Arc<dyn Opener + Send + Sync> {
    Arc::new(System)
}
