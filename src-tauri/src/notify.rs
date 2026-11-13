//! Reminders as macOS notifications (ADR 0035). Until the real center is wired (step 20), and always in
//! tests and the debug build, notifications are recorded instead of posted (ADR 0022).

use crate::lock::MutexExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

/// Keeps what would have been posted, newest last.
#[derive(Default)]
pub struct Notifier {
    recorded: std::sync::Mutex<Vec<Notification>>,
}

impl Notifier {
    pub fn post(&self, notification: Notification) {
        log::info!("Notification recorded: {}", notification.title);
        self.recorded.lock_unpoisoned().push(notification);
    }

    pub fn recorded(&self) -> Vec<Notification> {
        self.recorded.lock_unpoisoned().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(title: &str) -> Notification {
        Notification {
            title: title.into(),
            body: format!("{title} body"),
        }
    }

    #[test]
    fn a_new_notifier_has_recorded_nothing() {
        assert_eq!(Notifier::default().recorded(), vec![]);
    }

    #[test]
    fn posts_are_recorded_in_order() {
        let notifier = Notifier::default();
        notifier.post(note("First"));
        notifier.post(note("Second"));
        notifier.post(note("First"));
        assert_eq!(
            notifier.recorded(),
            vec![note("First"), note("Second"), note("First")]
        );
    }
}
