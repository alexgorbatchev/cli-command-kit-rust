use clap::{Arg, Command};
use cli_command_kit::{HelpOptions, Mode, TreeOptions, render_help, setup};
use unicode_width::UnicodeWidthStr;

fn command() -> Command {
    Command::new("track-auto-cue").subcommand(
        Command::new("separate")
            .arg(Arg::new("input").required(true))
            .args(
                ["output-dir", "medium-model", "high-model", "onnx-runtime"]
                    .map(|name| Arg::new(name).long(name).required(true).value_name("FILE")),
            )
            .arg(Arg::new("日本語").long("日本語").required(true)),
    )
}

fn usage(text: &str) -> &str {
    text.split("Usage:\n")
        .nth(1)
        .unwrap()
        .split("\n\n")
        .next()
        .unwrap()
}

#[test]
fn human_usage_wraps_without_losing_or_splitting_command_tokens() {
    let root = command();
    let options = |width| HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(width),
            ..Default::default()
        },
        ..Default::default()
    };
    let unwrapped = render_help(&root, &["separate"], Mode::Human, &options(0)).unwrap();
    let original: Vec<_> = usage(&unwrapped).split_whitespace().collect();
    assert!(usage(&unwrapped).trim().lines().count() == 1);
    for width in [40, 60, 80, 100] {
        let text = render_help(&root, &["separate"], Mode::Human, &options(width)).unwrap();
        assert!(
            usage(&text).lines().all(|line| line.width() <= width),
            "{text}"
        );
        assert_eq!(
            usage(&text).split_whitespace().collect::<Vec<_>>(),
            original
        );
        let agent = render_help(&root, &["separate"], Mode::Agent, &options(width)).unwrap();
        assert_eq!(
            agent,
            render_help(&root, &["separate"], Mode::Agent, &options(0)).unwrap()
        );
        for args in [
            vec!["track-auto-cue", "separate", "--help"],
            vec!["track-auto-cue", "help", "separate"],
        ] {
            let error = setup(command(), Mode::Human, &options(width))
                .unwrap()
                .try_get_matches_from(args)
                .unwrap_err();
            assert_eq!(usage(&error.to_string()), usage(&text));
        }
    }
}

#[test]
fn unbreakable_tokens_are_preserved_even_on_tiny_terminals() {
    let options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(5),
            ..Default::default()
        },
        ..Default::default()
    };
    let text = render_help(&command(), &["separate"], Mode::Human, &options).unwrap();
    assert!(usage(&text).contains("track-auto-cue"));
    assert!(usage(&text).contains("--medium-model"));
    assert!(!usage(&text).contains("..."));
}

#[test]
fn flags_without_descriptions_remain_visible() {
    let options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(80),
            ..Default::default()
        },
        ..Default::default()
    };
    let text = render_help(&command(), &["separate"], Mode::Human, &options).unwrap();
    let flags = text.split("Flags:\n").nth(1).unwrap();
    for name in [
        "--output-dir",
        "--medium-model",
        "--high-model",
        "--onnx-runtime",
    ] {
        assert!(flags.contains(name), "{text}");
    }
}
