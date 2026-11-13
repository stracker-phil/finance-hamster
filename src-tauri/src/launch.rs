//! What a launch leaves for the page: queued until the page listens and takes it.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::events::LAUNCH_EVENT;
use crate::ipc::IpcMessage;
use crate::lock::MutexExt;
use crate::state::AppState;
use crate::window;

/// Something the page acts on once it listens: from the launch that started the app or a later one.
#[derive(Serialize, Debug, Clone, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchEvent {
    Error { message: String },
    Activate,
}

/// `LaunchEvent`s waiting for the page to take them (`take_launch_events`).
#[derive(Default)]
pub struct LaunchQueue {
    events: Mutex<Vec<LaunchEvent>>,
}

impl LaunchQueue {
    /// Returns true when the message produced an event for the page.
    pub fn push(&self, message: IpcMessage) -> bool {
        let event = match message {
            IpcMessage::Error { message } => LaunchEvent::Error { message },
            IpcMessage::Activate { hidden: false } | IpcMessage::Show { .. } => {
                LaunchEvent::Activate
            }
            _ => return false,
        };
        self.events.lock_unpoisoned().push(event);
        true
    }

    pub fn take_events(&self) -> Vec<LaunchEvent> {
        std::mem::take(&mut *self.events.lock_unpoisoned())
    }
}

/// Queues the message for the page and shows the window, except for a hidden activation.
pub fn queue(handle: &AppHandle, message: IpcMessage) {
    let show = !matches!(message, IpcMessage::Activate { hidden: true });
    if handle.state::<AppState>().launches.push(message)
        && let Err(e) = handle.emit(LAUNCH_EVENT, ())
    {
        log::warn!("Couldn't notify the page of a launch: {e}");
    }
    if show {
        window::show_main(handle);
    }
}

#[tauri::command]
pub fn take_launch_events(state: State<'_, AppState>) -> Vec<LaunchEvent> {
    state.launches.take_events()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_message_becomes_an_error_event() {
        let queue = LaunchQueue::default();
        assert!(queue.push(IpcMessage::Error {
            message: "boom".into()
        }));
        assert_eq!(
            queue.take_events(),
            vec![LaunchEvent::Error {
                message: "boom".into()
            }]
        );
    }

    #[test]
    fn a_visible_activation_or_show_becomes_an_activate_event() {
        for message in [
            IpcMessage::Activate { hidden: false },
            IpcMessage::Show { focus: false },
            IpcMessage::Show { focus: true },
        ] {
            let queue = LaunchQueue::default();
            assert!(queue.push(message.clone()), "{message:?}");
            assert_eq!(
                queue.take_events(),
                vec![LaunchEvent::Activate],
                "{message:?}"
            );
        }
    }

    #[test]
    fn a_hidden_activation_and_other_messages_leave_no_event() {
        for message in [
            IpcMessage::Activate { hidden: true },
            IpcMessage::Hide,
            IpcMessage::Quit,
            IpcMessage::Status,
        ] {
            let queue = LaunchQueue::default();
            assert!(!queue.push(message.clone()), "{message:?}");
            assert_eq!(queue.take_events(), vec![], "{message:?}");
        }
    }

    #[test]
    fn taking_events_drains_them_in_order() {
        let queue = LaunchQueue::default();
        queue.push(IpcMessage::Error {
            message: "first".into(),
        });
        queue.push(IpcMessage::Activate { hidden: false });
        queue.push(IpcMessage::Error {
            message: "second".into(),
        });

        assert_eq!(
            queue.take_events(),
            vec![
                LaunchEvent::Error {
                    message: "first".into()
                },
                LaunchEvent::Activate,
                LaunchEvent::Error {
                    message: "second".into()
                },
            ]
        );
        assert_eq!(queue.take_events(), vec![]);
    }

    #[test]
    fn events_have_their_wire_shape() {
        assert_eq!(
            serde_json::to_value(LaunchEvent::Error {
                message: "boom".into()
            })
            .unwrap(),
            serde_json::json!({ "type": "error", "message": "boom" })
        );
        assert_eq!(
            serde_json::to_value(LaunchEvent::Activate).unwrap(),
            serde_json::json!({ "type": "activate" })
        );
    }
}
