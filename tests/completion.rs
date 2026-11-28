use cli_command_kit::{
    Error, HelpOptions, Mode, SkillSource, TreeOptions,
    clap::{Arg, Command, error::ErrorKind},
    completion_command, render_help, render_usage, setup, skill_omissions, write_completion,
};
use std::io::{self, Write};

const SHELLS: &[&str] = &["bash", "elvish", "fish", "powershell", "zsh"];

fn root() -> Command {
    Command::new("notes").bin_name("work-notes").subcommand(
        Command::new("list").about("List notes").arg(
            Arg::new("format")
                .long("format")
                .value_parser(["text", "json"]),
        ),
    )
}

#[test]
fn completion_validates_shell_names_and_uses_native_help() {
    let options = HelpOptions {
        skill: Some(SkillSource::new("example/notes", "main").unwrap()),
        ..Default::default()
    };
    for mode in [Mode::Human, Mode::Agent] {
        let mut command = setup(root(), mode, &options).unwrap();
        for args in [
            ["work-notes", "completion", "--help"],
            ["work-notes", "help", "completion"],
        ] {
            let help = command.try_get_matches_from_mut(args).unwrap_err();
            assert_eq!(help.kind(), ErrorKind::DisplayHelp);
            let help = help.to_string();
            assert!(help.contains("work-notes completion <shell>"), "{help}");
            assert_eq!(help.starts_with("ALERT:"), mode == Mode::Agent, "{help}");
            for shell in SHELLS {
                assert!(help.contains(shell), "{help}");
            }
        }
        for (args, kind) in [
            (
                &["work-notes", "completion"][..],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                &["work-notes", "completion", "tcsh"],
                ErrorKind::InvalidValue,
            ),
        ] {
            assert_eq!(
                command.try_get_matches_from_mut(args).unwrap_err().kind(),
                kind
            );
        }
    }
    let omissions = skill_omissions(&root(), &options, "work-notes");
    assert!(
        omissions.contains(&"command work-notes completion".into()),
        "{omissions:?}"
    );
    assert!(
        omissions.contains(&"argument <shell> of work-notes completion".into()),
        "{omissions:?}"
    );
}

#[test]
fn completion_generates_each_script_from_the_configured_tree() {
    for shell in SHELLS {
        let scripts = [Mode::Human, Mode::Agent].map(|mode| {
            let mut command = setup(root(), mode, &HelpOptions::default()).unwrap();
            let matches = command
                .try_get_matches_from_mut(["work-notes", "completion", shell])
                .unwrap();
            let mut output = Vec::new();
            assert!(write_completion(&command, &matches, &mut output).unwrap());
            String::from_utf8(output).unwrap()
        });
        assert_eq!(scripts[0], scripts[1], "{shell}");
        for name in ["work-notes", "list", "format", "completion"] {
            assert!(scripts[0].contains(name), "{shell}: {name}");
        }
    }
}

#[test]
fn completion_rejects_matches_from_an_incompatible_command() {
    let root = setup(root(), Mode::Human, &HelpOptions::default()).unwrap();
    let mut command =
        Command::new("notes").subcommand(Command::new("completion").arg(Arg::new("shell")));
    for args in [
        &["notes", "completion"][..],
        &["notes", "completion", "bash"],
    ] {
        let matches = command.try_get_matches_from_mut(args).unwrap();
        let mut output = Vec::new();
        let error = write_completion(&root, &matches, &mut output).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(output.is_empty());
    }
}

#[test]
fn completion_propagates_write_failures_only_for_its_route() {
    struct BrokenWriter;
    impl Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut command = setup(root(), Mode::Human, &HelpOptions::default()).unwrap();
    let args = command
        .try_get_matches_from_mut(["work-notes", "list"])
        .unwrap();
    assert!(!write_completion(&command, &args, &mut BrokenWriter).unwrap());
    for shell in SHELLS {
        let args = command
            .try_get_matches_from_mut(["work-notes", "completion", shell])
            .unwrap();
        let error = write_completion(&command, &args, &mut BrokenWriter).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe, "{shell}");
    }
}

#[test]
fn completion_opt_out_applies_to_parsing_rendering_and_skill_coverage() {
    let options = HelpOptions {
        disable_completion: true,
        ..Default::default()
    };
    let mut configured = setup(root(), Mode::Human, &options).unwrap();
    assert_eq!(
        configured
            .try_get_matches_from_mut(["work-notes", "completion", "bash"])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidSubcommand
    );
    for mode in [Mode::Human, Mode::Agent] {
        for render in [render_help, render_usage] {
            assert!(
                !render(&root(), &[], mode, &options)
                    .unwrap()
                    .contains("completion")
            );
            assert_eq!(
                render(&configured, &["completion"], mode, &options),
                Err(Error::UnknownCommand("completion".into()))
            );
        }
    }
    assert!(
        skill_omissions(&root(), &options, "")
            .iter()
            .all(|item| !item.contains("completion"))
    );
}

#[test]
fn completion_collision_respects_custom_commands_when_disabled() {
    let options = HelpOptions {
        disable_completion: true,
        ..Default::default()
    };
    for custom in [
        Command::new("completion").arg(Arg::new("shell")),
        Command::new("shell")
            .alias("completion")
            .arg(Arg::new("shell")),
    ] {
        let root = root().subcommand(custom);
        assert_eq!(
            setup(root.clone(), Mode::Human, &HelpOptions::default()).unwrap_err(),
            Error::CompletionCollision
        );
        assert!(
            skill_omissions(&root, &HelpOptions::default(), "")
                .contains(&Error::CompletionCollision.to_string())
        );
        let mut command = setup(root, Mode::Human, &options).unwrap();
        let matches = command
            .try_get_matches_from_mut(["work-notes", "completion", "custom"])
            .unwrap();
        let mut output = Vec::new();
        assert!(!write_completion(&command, &matches, &mut output).unwrap());
        assert!(output.is_empty());
    }
}

#[test]
fn manual_completion_and_global_shell_flag_can_coexist() {
    let root = root()
        .subcommand(completion_command())
        .arg(Arg::new("shell").long("shell").global(true));
    let options = HelpOptions {
        tree: TreeOptions {
            hide_generated_commands: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut command = setup(root, Mode::Human, &options).unwrap();
    assert!(
        !render_help(&command, &[], Mode::Human, &options)
            .unwrap()
            .contains("completion")
    );
    assert!(
        render_usage(&command, &[], Mode::Agent, &options)
            .unwrap()
            .contains("completion")
    );
    let matches = command
        .try_get_matches_from_mut(["work-notes", "completion", "--shell", "custom", "bash"])
        .unwrap();
    assert_eq!(matches.get_one::<String>("shell").unwrap(), "custom");
    let mut output = Vec::new();
    assert!(write_completion(&command, &matches, &mut output).unwrap());
    assert!(String::from_utf8(output).unwrap().contains("complete -F"));
}
