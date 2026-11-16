//! What the command prints for the app's reply, one fact per line so scripts can grep it. `--json` prints the
//! reply itself instead.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::CommandInfo;
use crate::ipc::IpcMessage;
use crate::profiles::cli::ProfileRequest;
use crate::profiles::{Profile, Profiles};

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
            &StatusReport::deserialize(result).map_err(unexpected)?,
        )),
        IpcMessage::Profile { request } => Some(profile_text(request, result)?),
        _ => None,
    })
}

fn unexpected(e: serde_json::Error) -> String {
    format!("The running Hamster sent an unexpected reply: {e}")
}

/// `list`: one profile per line, `*` before the active one; the others: what changed.
fn profile_text(request: &ProfileRequest, result: &Value) -> Result<String, String> {
    if *request == ProfileRequest::List {
        let profiles = Profiles::deserialize(result).map_err(unexpected)?;
        return Ok(profiles
            .profiles
            .iter()
            .map(|p| {
                format!(
                    "{} {}",
                    if p.id == profiles.active { "*" } else { " " },
                    p.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let profile = Profile::deserialize(result).map_err(unexpected)?;
    Ok(match request {
        ProfileRequest::Switch { .. } | ProfileRequest::Add { switch: true, .. } => {
            format!("Active profile: {}", profile.name)
        }
        ProfileRequest::Add { .. } => format!("Added profile {}", profile.name),
        ProfileRequest::Rename { .. } => format!("Renamed to {}", profile.name),
        ProfileRequest::List => unreachable!("answered above"),
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
                id: "go:today".into(),
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
            info("go:today", Some("⌘1"), "Today"),
            info("demo.noshortcut", None, "No Shortcut"),
        ];
        assert_eq!(
            commands_text(&commands, false),
            "go:today\t⌘1\tToday\ndemo.noshortcut\t\tNo Shortcut\n"
        );
    }

    #[test]
    fn commands_text_as_json_is_an_array_of_id_title_shortcut_and_custom() {
        let commands = [
            info("go:today", Some("⌘1"), "Today"),
            info("demo.noshortcut", None, "No Shortcut"),
        ];
        let text = commands_text(&commands, true);
        assert!(text.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            json!([
                { "id": "go:today", "title": "Today", "shortcut": "⌘1", "custom": false },
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

    fn profile_message(request: ProfileRequest) -> IpcMessage {
        IpcMessage::Profile { request }
    }

    #[test]
    fn profile_list_marks_the_active_profile_with_a_star() {
        let result = json!({
            "active": "a1",
            "profiles": [{ "id": "a1", "name": "Demo" }, { "id": "b2", "name": "Alex" }]
        });
        assert_eq!(
            render(&profile_message(ProfileRequest::List), &result),
            Ok(Some("* Demo\n  Alex".to_string()))
        );
    }

    #[test]
    fn profile_changes_print_what_happened() {
        let result = json!({ "id": "c3", "name": "Casey" });
        for (request, text) in [
            (
                ProfileRequest::Switch {
                    name: "casey".into(),
                },
                "Active profile: Casey",
            ),
            (
                ProfileRequest::Add {
                    name: "Casey".into(),
                    switch: true,
                },
                "Active profile: Casey",
            ),
            (
                ProfileRequest::Add {
                    name: "Casey".into(),
                    switch: false,
                },
                "Added profile Casey",
            ),
            (
                ProfileRequest::Rename {
                    name: "Alex".into(),
                    to: "Casey".into(),
                },
                "Renamed to Casey",
            ),
        ] {
            assert_eq!(
                render(&profile_message(request.clone()), &result),
                Ok(Some(text.to_string())),
                "{request:?}"
            );
        }
    }

    #[test]
    fn a_malformed_profile_reply_is_an_error() {
        let list = json!({ "active": "a1", "profiles": [] });
        let switch = ProfileRequest::Switch {
            name: "Casey".into(),
        };
        for (request, result) in [
            (ProfileRequest::List, json!("Demo")),
            (ProfileRequest::List, json!({ "profiles": "none" })),
            (switch.clone(), json!(null)),
            (switch, json!({ "id": "c3" })),
            (
                ProfileRequest::Add {
                    name: "Casey".into(),
                    switch: false,
                },
                list,
            ),
            (
                ProfileRequest::Rename {
                    name: "Alex".into(),
                    to: "Casey".into(),
                },
                json!([]),
            ),
        ] {
            let err = render(&profile_message(request.clone()), &result).unwrap_err();
            assert!(
                err.starts_with("The running Hamster sent an unexpected reply: "),
                "{request:?} {result}: {err}"
            );
        }
    }

    #[test]
    fn profile_replies_use_the_shapes_the_cli_prints() {
        let profiles = Profiles {
            active: "a1".into(),
            profiles: vec![Profile {
                id: "a1".into(),
                name: "Demo".into(),
            }],
        };
        assert_eq!(
            serde_json::to_value(&profiles).unwrap(),
            json!({ "active": "a1", "profiles": [{ "id": "a1", "name": "Demo" }] })
        );
        assert_eq!(
            serde_json::to_value(&profiles.profiles[0]).unwrap(),
            json!({ "id": "a1", "name": "Demo" })
        );
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
