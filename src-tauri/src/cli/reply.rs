//! What the command prints for the app's reply, one fact per line so scripts can grep it. `--json` prints the
//! reply itself instead.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ipc::IpcMessage;

/// The reply to `status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub version: String,
    pub window_visible: bool,
}

/// The text for `result`, the reply to `message`; `None` when the subcommand prints nothing.
pub fn render(message: &IpcMessage, result: &Value) -> Result<Option<String>, String> {
    Ok(match message {
        IpcMessage::Status => Some(status_text(
            &StatusReport::deserialize(result)
                .map_err(|e| format!("The running Hamster sent an unexpected reply: {e}"))?,
        )),
        _ => None,
    })
}

fn status_text(report: &StatusReport) -> String {
    let window = if report.window_visible {
        "visible"
    } else {
        "hidden"
    };
    format!("Hamster {}\nwindow: {window}", report.version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_prints_the_version_and_whether_the_window_is_visible() {
        for (visible, window) in [(true, "visible"), (false, "hidden")] {
            let result = json!({ "version": "1.2.3", "windowVisible": visible });
            assert_eq!(
                render(&IpcMessage::Status, &result),
                Ok(Some(format!("Hamster 1.2.3\nwindow: {window}"))),
                "{visible}"
            );
        }
    }

    #[test]
    fn subcommands_without_output_print_nothing() {
        for message in [
            IpcMessage::Hide,
            IpcMessage::Quit,
            IpcMessage::Show { focus: false },
            IpcMessage::Activate { hidden: false },
        ] {
            assert_eq!(
                render(&message, &serde_json::Value::Null),
                Ok(None),
                "{message:?}"
            );
        }
    }

    #[test]
    fn a_malformed_status_reply_is_an_error() {
        for result in [json!(null), json!({ "version": "1.0.0" }), json!("visible")] {
            let err = render(&IpcMessage::Status, &result).unwrap_err();
            assert!(
                err.starts_with("The running Hamster sent an unexpected reply: "),
                "{result}: {err}"
            );
        }
    }

    #[test]
    fn status_report_uses_camel_case_on_the_wire() {
        let report = StatusReport {
            version: "0.1.0".into(),
            window_visible: true,
        };
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            json!({ "version": "0.1.0", "windowVisible": true })
        );
    }
}
