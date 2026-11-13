//! Reminders as macOS notifications (ADR 0042). The release build hands them to the notification center; the
//! debug build and tests only record them (ADR 0022).

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

    /// Records it, and in the release build shows it in the notification center.
    pub fn deliver(&self, app: &tauri::AppHandle, notification: Notification) {
        #[cfg(not(debug_assertions))]
        {
            use tauri_plugin_notification::NotificationExt;
            let shown = app
                .notification()
                .builder()
                .title(&notification.title)
                .body(&notification.body)
                .show();
            if let Err(e) = shown {
                log::warn!("Couldn't show a notification: {e}");
            }
        }
        #[cfg(debug_assertions)]
        let _ = app;
        self.post(notification);
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
