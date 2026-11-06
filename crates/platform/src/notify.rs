//! A toast on the desktop, for what the window cannot say because it has just gone. Never
//! load-bearing: what it says has already happened, so a desktop that drops it changes nothing.

mod system;

pub use system::system;

pub trait Notifier {
    /// ⚠️ May block on the desktop's bus: never on the window's thread.
    fn notify(&self, title: &str, body: &str);
}

pub struct NoNotifier;

impl Notifier for NoNotifier {
    fn notify(&self, _title: &str, _body: &str) {}
}
