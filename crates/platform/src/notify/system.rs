//! The toast itself, which only a desktop can show: the OS checklist's (plan § 8.5).

use std::sync::Arc;

use notify_rust::Notification;

use super::Notifier;

struct System {
    name: String,
    #[cfg_attr(
        not(windows),
        expect(dead_code, reason = "only Windows names its sender")
    )]
    identifier: String,
}

impl Notifier for System {
    fn notify(&self, title: &str, body: &str) {
        let mut toast = Notification::new();
        toast.appname(&self.name).summary(title).body(body);
        #[cfg(windows)]
        toast.app_id(&self.identifier);
        if let Err(error) = toast.show() {
            log::warn!("The desktop took no notification: {error}");
        }
    }
}

pub fn system(name: &str, identifier: &str) -> Arc<dyn Notifier + Send + Sync> {
    Arc::new(System {
        name: name.to_owned(),
        identifier: identifier.to_owned(),
    })
}
