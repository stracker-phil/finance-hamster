use super::*;

fn run(args: &[&str]) -> CliOutcome {
    parse(std::iter::once("hamster").chain(args.iter().copied()))
}

fn request(message: IpcMessage, json: bool) -> CliRequest {
    CliRequest { message, json }
}

#[test]
fn no_arguments_start_the_app_visible_with_the_default_log_level() {
    assert_eq!(
        run(&[]),
        CliOutcome::Run {
            hidden: false,
            log_level: None
        }
    );
}

#[test]
fn hidden_flag_starts_the_app_hidden() {
    assert_eq!(
        run(&["--hidden"]),
        CliOutcome::Run {
            hidden: true,
            log_level: None
        }
    );
}

#[test]
fn log_level_flag_sets_the_log_level() {
    for (value, level) in [
        ("error", LogLevel::Error),
        ("warn", LogLevel::Warn),
        ("info", LogLevel::Info),
        ("debug", LogLevel::Debug),
        ("trace", LogLevel::Trace),
    ] {
        assert_eq!(
            run(&["--log-level", value]),
            CliOutcome::Run {
                hidden: false,
                log_level: Some(level)
            },
            "{value}"
        );
    }
}

#[test]
fn an_unknown_log_level_is_a_usage_error() {
    assert!(matches!(
        run(&["--log-level", "loud"]),
        CliOutcome::Exit { code: 2, .. }
    ));
}

#[test]
fn log_levels_map_to_log_filters() {
    for (level, filter) in [
        (LogLevel::Error, log::LevelFilter::Error),
        (LogLevel::Warn, log::LevelFilter::Warn),
        (LogLevel::Info, log::LevelFilter::Info),
        (LogLevel::Debug, log::LevelFilter::Debug),
        (LogLevel::Trace, log::LevelFilter::Trace),
    ] {
        assert_eq!(log::LevelFilter::from(level), filter, "{level:?}");
    }
    assert_eq!(LogLevel::default(), LogLevel::Info);
}

#[test]
fn status_hide_and_quit_only_ask_the_running_app() {
    for (subcommand, message) in [
        ("status", IpcMessage::Status),
        ("hide", IpcMessage::Hide),
        ("quit", IpcMessage::Quit),
    ] {
        assert_eq!(
            run(&[subcommand]),
            CliOutcome::Ask(request(message, false)),
            "{subcommand}"
        );
    }
}

#[test]
fn run_asks_the_running_app_to_run_the_command_id() {
    assert_eq!(
        run(&["run", "go.today"]),
        CliOutcome::Ask(request(
            IpcMessage::Run {
                id: "go.today".into()
            },
            false
        ))
    );
}

#[test]
fn run_without_an_id_is_a_usage_error() {
    assert!(matches!(run(&["run"]), CliOutcome::Exit { code: 2, .. }));
}

#[test]
fn commands_prints_every_command_one_per_line_without_the_app() {
    let listed = crate::commands::list();
    match run(&["commands"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            let lines: Vec<&str> = message.lines().collect();
            assert_eq!(lines.len(), listed.len());
            for (line, info) in lines.iter().zip(&listed) {
                let fields: Vec<&str> = line.split('\t').collect();
                assert_eq!(
                    fields,
                    vec![
                        info.id.as_str(),
                        info.shortcut.as_deref().unwrap_or(""),
                        info.title.as_str()
                    ]
                );
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn commands_with_json_prints_the_list_as_a_json_array() {
    let listed = crate::commands::list();
    for args in [["commands", "--json"], ["--json", "commands"]] {
        match run(&args) {
            CliOutcome::Exit { message, code } => {
                assert_eq!(code, 0, "{args:?}");
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&message).unwrap(),
                    serde_json::to_value(&listed).unwrap(),
                    "{args:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn show_and_focus_may_start_the_app() {
    assert_eq!(
        run(&["show"]),
        CliOutcome::Remote(request(IpcMessage::Show { focus: false }, false))
    );
    assert_eq!(
        run(&["focus"]),
        CliOutcome::Remote(request(IpcMessage::Activate { hidden: false }, false))
    );
}

#[test]
fn json_flag_is_accepted_before_and_after_the_subcommand() {
    for args in [["--json", "status"], ["status", "--json"]] {
        assert_eq!(
            run(&args),
            CliOutcome::Ask(request(IpcMessage::Status, true)),
            "{args:?}"
        );
    }
    assert_eq!(
        run(&["focus", "--json"]),
        CliOutcome::Remote(request(IpcMessage::Activate { hidden: false }, true))
    );
}

#[test]
fn an_unknown_subcommand_exits_with_code_2_and_a_message() {
    match run(&["frobnicate"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 2);
            assert!(!message.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn dev_seed_with_yes_resets_the_demo_data() {
    assert_eq!(run(&["dev", "seed", "--yes"]), CliOutcome::ResetDemo);
}

#[test]
fn dev_seed_without_yes_is_refused_with_code_2() {
    match run(&["dev", "seed"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 2);
            assert!(!message.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn version_prints_the_version_and_exits_with_code_0() {
    assert_eq!(
        run(&["--version"]),
        CliOutcome::Exit {
            message: format!("hamster {}\n", env!("CARGO_PKG_VERSION")),
            code: 0
        }
    );
}

#[test]
fn help_exits_with_code_0_and_lists_the_subcommands() {
    match run(&["--help"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            for subcommand in ["status", "show", "focus", "hide", "quit"] {
                assert!(message.contains(subcommand), "{subcommand}");
            }
        }
        other => panic!("{other:?}"),
    }
}
