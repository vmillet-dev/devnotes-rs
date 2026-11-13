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

/// Opens nothing, until the shell hands the system's: a test never starts a program.
pub struct NoOpener;

impl Opener for NoOpener {
    fn open(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::other("no desktop here"))
    }
}
