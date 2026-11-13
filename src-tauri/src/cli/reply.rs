//! What the command prints for the app's reply, one fact per line so scripts can grep it. `--json` prints the
//! reply itself instead.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::CommandInfo;
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

/// One command per line: id, shortcut and title, tab-separated; `--json` prints the list as JSON.
pub fn commands_text(commands: &[CommandInfo], json: bool) -> String {
    if json {
        return serde_json::to_string(commands).unwrap_or_default() + "\n";
    }
    commands
        .iter()
        .map(|c| {
            format!(
                "{}\t{}\t{}\n",
                c.id,
                c.shortcut.as_deref().unwrap_or(""),
                c.title
            )
        })
        .collect()
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
            IpcMessage::Run {
                id: "go.today".into(),
            },
        ] {
            assert_eq!(
                render(&message, &serde_json::Value::Null),
                Ok(None),
                "{message:?}"
            );
        }
    }

    fn info(id: &str, shortcut: Option<&str>, title: &str) -> CommandInfo {
        CommandInfo {
            id: id.into(),
            title: title.into(),
            shortcut: shortcut.map(Into::into),
            custom: false,
        }
    }

    #[test]
    fn commands_text_lists_id_shortcut_and_title_tab_separated_per_line() {
        let commands = [
            info("go.today", Some("⌘1"), "Today"),
            info("demo.noshortcut", None, "No Shortcut"),
        ];
        assert_eq!(
            commands_text(&commands, false),
            "go.today\t⌘1\tToday\ndemo.noshortcut\t\tNo Shortcut\n"
        );
    }

    #[test]
    fn commands_text_as_json_is_an_array_of_id_title_shortcut_and_custom() {
        let commands = [
            info("go.today", Some("⌘1"), "Today"),
            info("demo.noshortcut", None, "No Shortcut"),
        ];
        let text = commands_text(&commands, true);
        assert!(text.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            json!([
                { "id": "go.today", "title": "Today", "shortcut": "⌘1", "custom": false },
                { "id": "demo.noshortcut", "title": "No Shortcut", "shortcut": null, "custom": false }
            ])
        );
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
