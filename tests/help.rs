use clap::{Arg, ArgAction, Command};
use cli_command_kit::*;
use std::collections::BTreeMap;

fn fixture() -> Command {
    Command::new("app")
        .about("Manage documents")
        .version("1.2.3")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .global(true)
                .default_value("cfg.toml")
                .help("Configuration file"),
        )
        .subcommand(
            Command::new("doc")
                .about("Documents")
                .subcommand(
                    Command::new("list").about("List documents").arg(
                        Arg::new("all")
                            .long("all")
                            .action(ArgAction::SetTrue)
                            .help("Include archived documents"),
                    ),
                )
                .subcommand(
                    Command::new("create")
                        .about("Create a document")
                        .arg(Arg::new("name").required(true).help("Document name")),
                ),
        )
        .subcommand(Command::new("secret").hide(true))
}

#[test]
fn human_help_lists_full_subtree_and_shared_columns() {
    let root = fixture();
    let options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(100),
            ..Default::default()
        },
        ..Default::default()
    };
    let output = render_help(&root, &[], Mode::Human, &options).unwrap();
    assert!(output.contains("╰─ doc"));
    assert!(output.contains("   ├─ list"));
    assert!(output.contains("   ╰─ create <name>"), "{output}");
    assert_eq!(output.matches("List documents").count(), 1);
    assert!(!output.contains("secret"));
    assert!(output.contains("--config"));
    let child = render_help(&root, &["doc", "create"], Mode::Human, &options).unwrap();
    assert!(child.contains("Document name"));
    assert!(child.contains("--config"));
    assert!(child.contains("app doc create"));
}

#[test]
fn human_about_wraps_to_terminal_cells_and_preserves_paragraphs() {
    let about = "Keep a fast local copy of your email and calendar for searching offline.\n\n日本語の説明 with enough words to require another wrapped line in the terminal.";
    let root = Command::new("app")
        .long_about(about)
        .subcommand(Command::new("doc").long_about(about));
    let mut options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(40),
            ..Default::default()
        },
        ..Default::default()
    };
    for path in [vec![], vec!["doc"]] {
        assert!(
            render_help(&root, &path, Mode::Agent, &options)
                .unwrap()
                .contains(about)
        );
        let human = render_help(&root, &path, Mode::Human, &options).unwrap();
        let description = human.split("\n\nUsage:").next().unwrap();
        assert!(description.contains("\n\n日本語の説明"), "{human}");
        assert!(
            description
                .lines()
                .all(|line| unicode_width::UnicodeWidthStr::width(line) <= 40),
            "{human}"
        );
    }
    options.tree.terminal_width = Some(0);
    assert!(
        render_help(&root, &[], Mode::Human, &options)
            .unwrap()
            .starts_with(about)
    );
}

#[test]
fn agent_help_is_untruncated_deterministic_and_contains_real_metadata() {
    let summary = "日".repeat(200);
    let mut options = HelpOptions::default();
    options.catalog.insert(
        "app".into(),
        TechInfo {
            summary: summary.clone(),
            metadata: BTreeMap::from([
                ("usage".into(), "quota".into()),
                ("alpha".into(), "one".into()),
            ]),
            ..Default::default()
        },
    );
    let first = render_help(&fixture(), &[], Mode::Agent, &options).unwrap();
    assert!(first.contains(&summary));
    assert!(first.contains("metadata:\n  alpha: one\n  usage: quota\n"));
    assert_eq!(
        first
            .lines()
            .filter(|line| line.starts_with("usage:"))
            .count(),
        1
    );
    assert!(first.contains(
        "  - doc: Documents\n    - list: List documents\n    - create <name>: Create a document"
    ));
    let child = render_help(&fixture(), &["doc", "list"], Mode::Agent, &options).unwrap();
    assert!(child.contains("--all bool: Include archived documents"));
    assert!(child.contains("-c, --config string: Configuration file (default: \"cfg.toml\")"));
    for _ in 0..10 {
        assert_eq!(
            first,
            render_help(&fixture(), &[], Mode::Agent, &options).unwrap()
        );
    }
}

#[test]
fn setup_uses_clap_for_all_help_paths_and_propagates_skill_alert() {
    let options = HelpOptions {
        skill: Some("exact skill\n".into()),
        ..Default::default()
    };
    for arguments in [
        vec!["app", "--help"],
        vec!["app", "doc", "--help"],
        vec!["app", "help", "doc"],
        vec!["app", "skill", "--help"],
    ] {
        let mut root = setup(fixture(), Mode::Agent, &options).unwrap();
        let error = root.try_get_matches_from_mut(arguments).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let text = error.to_string();
        assert!(
            text.starts_with(
                "ALERT: Agents must read `AGENT=1 app skill` before using this tool.\n"
            ),
            "{text}"
        );
    }
    let mut root = setup(fixture(), Mode::Human, &options).unwrap();
    assert!(
        root.try_get_matches_from_mut(["app", "skill", "unexpected"])
            .is_err()
    );
    let matches = root.try_get_matches_from_mut(["app", "skill"]).unwrap();
    let mut bytes = Vec::new();
    assert!(write_skill(&matches, &options, &mut bytes).unwrap());
    assert_eq!(bytes, b"exact skill\n");
    let mut broken = BrokenWriter;
    assert!(write_skill(&matches, &options, &mut broken).is_err());
}

struct BrokenWriter;
impl std::io::Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn unicode_cells_and_agent_key_structure_survive_clipping() {
    let mut options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(45),
            ..Default::default()
        },
        agent: AgentOptions {
            max_line_width: Some(24),
        },
        ..Default::default()
    };
    options.catalog.insert(
        "app".into(),
        TechInfo {
            summary: "日".repeat(100),
            metadata: BTreeMap::from([
                ("long_key_that_cannot_fit".into(), "v".repeat(80)),
                ("x".into(), "日".repeat(80)),
            ]),
            ..Default::default()
        },
    );
    let text = render_help(&fixture(), &[], Mode::Agent, &options).unwrap();
    assert!(text.contains("metadata:\n"));
    assert!(text.contains(&format!("long_key_that_cannot_fit: {}", "v".repeat(80))));
    let clipped = text.lines().find(|line| line.starts_with("  x:")).unwrap();
    assert!(clipped.ends_with("..."));
    assert!(unicode_width::UnicodeWidthStr::width(clipped) <= 24);
    let root = Command::new("app").subcommand(Command::new("文書").about("日".repeat(60)));
    let human = render_help(&root, &[], Mode::Human, &options).unwrap();
    let row = human
        .lines()
        .find(|line| line.contains("╰─ 文書"))
        .unwrap_or_else(|| panic!("{human}"));
    assert!(row.ends_with("..."));
    assert!(unicode_width::UnicodeWidthStr::width(row) <= 45);
}

#[test]
fn skill_coverage_checks_boundaries_types_defaults_and_environment() {
    let root = Command::new("app")
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .default_value("cfg.toml")
                .env("APP_CONFIG"),
        );
    let bad = skill_omissions(
        &root,
        &HelpOptions::default(),
        "app --configuration -config APP_CONFIG_EXTRA",
    );
    assert!(bad.iter().any(|x| x == "flag --config"));
    assert!(bad.iter().any(|x| x.contains("shorthand")));
    assert!(bad.iter().any(|x| x.contains("APP_CONFIG")));
    let missing = skill_omissions(&root, &HelpOptions::default(), "app --config -c APP_CONFIG");
    assert!(missing.iter().any(|x| x.contains("type string")));
    assert!(missing.iter().any(|x| x.contains("cfg.toml")));
    assert!(
        skill_omissions(
            &root,
            &HelpOptions::default(),
            "app\n--config -c string cfg.toml APP_CONFIG"
        )
        .is_empty()
    );
}

#[test]
fn skill_coverage_recognizes_explicit_empty_string_defaults() {
    let root = Command::new("app")
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .arg(Arg::new("model").long("model").default_value(""));
    for description in [
        "app\n--model string default empty",
        "app\n--model string default \"\"",
    ] {
        assert!(skill_omissions(&root, &HelpOptions::default(), description).is_empty());
    }
    assert!(
        skill_omissions(
            &root,
            &HelpOptions::default(),
            "app\n--model string default auto"
        )
        .iter()
        .any(|omission| omission.contains("default"))
    );
}

#[test]
fn rejects_unknown_command_paths_and_skill_collisions() {
    assert!(
        render_help(
            &fixture(),
            &["missing"],
            Mode::Human,
            &HelpOptions::default()
        )
        .is_err()
    );
    let options = HelpOptions {
        skill: Some("skill".into()),
        ..Default::default()
    };
    assert!(
        setup(
            fixture().subcommand(Command::new("x").alias("skill")),
            Mode::Human,
            &options
        )
        .is_err()
    );
}

#[test]
fn display_paths_fold_home_on_segment_boundaries() {
    for (path, expected) in [
        ("/home/dev", "~"),
        ("/home/dev/code", "~/code"),
        ("/home/developer/code", "/home/developer/code"),
        ("/opt/code", "/opt/code"),
    ] {
        assert_eq!(fold_home(path.as_ref(), "/home/dev".as_ref()), expected);
    }
    for value in ["1", "true", "YES", " true "] {
        assert_eq!(Mode::from_value(value), Mode::Agent);
    }
    for value in ["0", "", "false", "not true"] {
        assert_eq!(Mode::from_value(value), Mode::Human);
    }
}

#[test]
fn catalogs_augment_native_values_and_human_usage_omits_description() {
    let root = Command::new("app")
        .about("Short summary")
        .long_about("Detailed explanation")
        .after_long_help("Additional examples")
        .arg(Arg::new("choice").required(true).value_parser([
            clap::builder::PossibleValue::new("read").help("Read only"),
            clap::builder::PossibleValue::new("write").help("Modify data"),
            clap::builder::PossibleValue::new("hidden").hide(true),
        ]))
        .arg(
            Arg::new("limit")
                .long("limit")
                .value_parser(clap::value_parser!(i64))
                .default_value("12"),
        )
        .arg(Arg::new("verbosity").short('v').action(ArgAction::Count))
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("FORMAT")
                .value_parser(["json", "text"])
                .env("APP_FORMAT")
                .help("Output format"),
        )
        .arg(Arg::new("private").long("private").hide(true))
        .arg(
            Arg::new("quiet-default")
                .long("quiet-default")
                .default_value("secret-value")
                .hide_default_value(true),
        )
        .subcommand(Command::new("edit").alias("e").about("Edit an entry"));
    let info = TechInfo {
        summary: "Catalog summary".into(),
        description: "Catalog details".into(),
        args: vec![Spec {
            name: "<choice>".into(),
            description: "Select an action".into(),
        }],
        env: vec![Spec {
            name: "APP_FORMAT".into(),
            description: "Preferred format".into(),
        }],
        quickstart: vec![
            QuickstartItem {
                command: "app read".into(),
                comment: " # Safe example ".into(),
            },
            QuickstartItem {
                command: "app write".into(),
                comment: String::new(),
            },
        ],
        mutates_db: true,
        auto_backup: true,
        ..Default::default()
    };
    let options = HelpOptions {
        catalog: BTreeMap::from([("app".into(), info)]),
        tree: TreeOptions {
            include_root: true,
            terminal_width: Some(0),
            min_padding: 3,
            min_label_width: 10,
            ..Default::default()
        },
        ..Default::default()
    };
    let human = render_help(&root, &[], Mode::Human, &options).unwrap();
    assert!(human.starts_with("Detailed explanation\n\n"));
    for text in [
        "Select an action",
        "Preferred format",
        "Quickstart:",
        "# Safe example",
        "Additional examples",
        "[possible: json, text]",
        "--format <FORMAT>",
    ] {
        assert!(human.contains(text), "{human}");
    }
    assert!(!human.contains("--private"));
    assert!(!human.contains("secret-value"));
    let usage = render_usage(&root, &[], Mode::Human, &options).unwrap();
    assert!(usage.starts_with("Usage:"));
    assert!(!usage.contains("Detailed explanation"));
    let agent = render_help(&root, &[], Mode::Agent, &options).unwrap();
    for text in [
        "summary: Catalog summary",
        "description: Catalog details",
        "valid_args:\n  - read: Read only\n  - write: Modify data",
        "--limit int64:",
        "-v uint8:",
        "mutates_db: true",
        "auto_backup: true",
        "(values: json,text)",
        "quickstart:\n  - app read: Safe example",
    ] {
        assert!(agent.contains(text), "{agent}");
    }
    assert!(!agent.contains("  - hidden"));
    assert!(
        render_help(&root, &["e"], Mode::Agent, &options)
            .unwrap()
            .contains("command: app edit")
    );
    assert_eq!(
        agent,
        render_usage(&root, &[], Mode::Agent, &options).unwrap()
    );
    assert!(render_usage(&root, &["absent"], Mode::Human, &options).is_err());
}

#[test]
fn generated_skill_hiding_changes_presentation_and_disabled_agent_stays_human() {
    let options = HelpOptions {
        skill: Some("read this\n".into()),
        tree: TreeOptions {
            hide_generated_commands: true,
            terminal_width: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };
    let human = render_help(&fixture(), &[], Mode::Human, &options).unwrap();
    assert!(!human.contains("Print the usage guide"));
    let agent = render_help(&fixture(), &[], Mode::Agent, &options).unwrap();
    assert!(agent.contains("skill: Print the usage guide"));
    let disabled = HelpOptions {
        disable_agent: true,
        ..options.clone()
    };
    assert_eq!(
        human,
        render_help(&fixture(), &[], Mode::Agent, &disabled).unwrap()
    );
    let mut root = setup(fixture(), Mode::Human, &options).unwrap();
    let matches = root
        .try_get_matches_from_mut(["app", "doc", "list"])
        .unwrap();
    assert!(!write_skill(&matches, &options, &mut Vec::new()).unwrap());
    let omissions = skill_omissions(&fixture(), &options, "");
    assert!(omissions.contains(&"command app skill".into()));
    let configured = setup(fixture(), Mode::Human, &options).unwrap();
    assert_eq!(omissions, skill_omissions(&configured, &options, ""));
    let collision = fixture().subcommand(Command::new("guide").alias("skill"));
    assert!(
        skill_omissions(&collision, &options, "")
            .iter()
            .any(|message| message.contains("already has"))
    );
    assert!(
        setup(collision, Mode::Human, &options)
            .unwrap_err()
            .to_string()
            .contains("already has")
    );
    assert!(
        render_help(&fixture(), &["absent"], Mode::Human, &options)
            .unwrap_err()
            .to_string()
            .contains("unknown command path")
    );
}

#[test]
fn leaf_help_handles_empty_and_custom_values_without_guessing_types() {
    #[derive(Clone)]
    struct Custom;
    let root = Command::new("app")
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("custom")
                .long("custom")
                .value_parser(clap::builder::ValueParser::new(|_: &str| {
                    Ok::<_, String>(Custom)
                })),
        )
        .arg(Arg::new("paths").num_args(1..).action(ArgAction::Append));
    let options = HelpOptions {
        tree: TreeOptions {
            terminal_width: Some(5),
            ..Default::default()
        },
        ..Default::default()
    };
    let output = render_help(&root, &[], Mode::Agent, &options).unwrap();
    assert!(output.contains("--custom value:"));
    assert!(output.contains("[paths]..."));
    let mut custom_options = options.clone();
    custom_options.catalog.insert(
        "app".into(),
        TechInfo {
            metadata: BTreeMap::from([("type:custom".into(), "record".into())]),
            ..Default::default()
        },
    );
    assert!(
        render_help(&root, &[], Mode::Agent, &custom_options)
            .unwrap()
            .contains("--custom record:")
    );
    let output = render_help(&root, &[], Mode::Human, &options).unwrap();
    assert!(output.contains("--custom <value>"));
    assert!(!output.contains("Available Commands:"));
    let mut skill_root = setup(
        Command::new("app"),
        Mode::Human,
        &HelpOptions {
            skill: Some("x".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let matches = skill_root
        .try_get_matches_from_mut(["app", "skill"])
        .unwrap();
    assert!(!write_skill(&matches, &HelpOptions::default(), &mut Vec::new()).unwrap());
    assert!(display_path(std::path::Path::new("relative/path")).ends_with("relative/path"));
    assert_eq!(
        Mode::from_env(),
        Mode::from_value(&std::env::var("AGENT").unwrap_or_default())
    );
}

#[test]
fn automatic_backup_help_only_advertises_an_override_that_exists() {
    let mut options = HelpOptions::default();
    options.catalog.insert(
        "app".into(),
        TechInfo {
            auto_backup: true,
            ..Default::default()
        },
    );
    let root = Command::new("app");
    assert!(
        !render_help(&root, &[], Mode::Agent, &options)
            .unwrap()
            .contains("override with")
    );
    let root = root.arg(
        Arg::new("no-backup")
            .long("no-backup")
            .action(ArgAction::SetTrue),
    );
    assert!(
        render_help(&root, &[], Mode::Agent, &options)
            .unwrap()
            .contains("override with --no-backup")
    );
}
