use clap::{Arg, Command};
use cli_command_kit::*;

const ALERT: &str = "ALERT: Agents must read `AGENT=1 app skill` before using this tool.\n";

/// The branch is deliberately not "main", so output that assumes one fails.
const INSTRUCTIONS: &str = "Install the app skill using the Skills CLI:\n\
    \n  npx skills add acme/app-cli --skill app\n\
    \nSkill:\n\
    \n  https://github.com/acme/app-cli/tree/trunk/skills/app\n";

fn root() -> Command {
    Command::new("app").about("Manage documents").subcommand(
        Command::new("doc")
            .about("Documents")
            .arg(Arg::new("name").help("Document name")),
    )
}

fn options() -> HelpOptions {
    HelpOptions {
        skill: Some(SkillSource::new("acme/app-cli", "trunk").unwrap()),
        ..Default::default()
    }
}

fn run_skill(root: Command, options: &HelpOptions) -> Vec<u8> {
    let mut configured = setup(root, Mode::Human, options).unwrap();
    let matches = configured
        .try_get_matches_from_mut(["app", "skill"])
        .unwrap();
    let mut bytes = Vec::new();
    assert!(write_skill(&configured, &matches, options, &mut bytes).unwrap());
    bytes
}

#[test]
fn skill_command_prints_the_install_instructions_identically_in_both_modes() {
    let options = options();
    for mode in [Mode::Human, Mode::Agent] {
        let mut configured = setup(root(), mode, &options).unwrap();
        let matches = configured
            .try_get_matches_from_mut(["app", "skill"])
            .unwrap();
        let mut bytes = Vec::new();
        assert!(write_skill(&configured, &matches, &options, &mut bytes).unwrap());
        assert_eq!(String::from_utf8(bytes).unwrap(), INSTRUCTIONS, "{mode:?}");
    }
}

#[test]
fn skill_command_names_the_root_command() {
    let options = HelpOptions {
        skill: Some(SkillSource::new("acme/tools", "main").unwrap()),
        ..Default::default()
    };
    let mut configured = setup(Command::new("other-tool"), Mode::Human, &options).unwrap();
    let matches = configured
        .try_get_matches_from_mut(["other-tool", "skill"])
        .unwrap();
    let mut bytes = Vec::new();
    assert!(write_skill(&configured, &matches, &options, &mut bytes).unwrap());
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "Install the other-tool skill using the Skills CLI:\n\n  \
         npx skills add acme/tools --skill other-tool\n\nSkill:\n\n  \
         https://github.com/acme/tools/tree/main/skills/other-tool\n"
    );
}

#[test]
fn skill_command_rejects_arguments_and_describes_itself() {
    let mut configured = setup(root(), Mode::Human, &options()).unwrap();
    assert!(
        configured
            .try_get_matches_from_mut(["app", "skill", "unexpected"])
            .is_err()
    );
    let skill = configured.find_subcommand("skill").unwrap();
    assert_eq!(
        skill.get_about().unwrap().to_string(),
        "Print how to install the agent skill"
    );
    assert_eq!(
        skill.get_arguments().filter(|a| !a.is_positional()).count(),
        1
    );
}

#[test]
fn skill_command_returns_the_write_failure() {
    struct BrokenWriter;
    impl std::io::Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let options = options();
    let mut configured = setup(root(), Mode::Human, &options).unwrap();
    let matches = configured
        .try_get_matches_from_mut(["app", "skill"])
        .unwrap();
    let error = write_skill(&configured, &matches, &options, &mut BrokenWriter).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
fn skill_source_accepts_only_a_github_repository_and_branch() {
    let source = SkillSource::new("acme/app-cli", "release/v3").unwrap();
    assert_eq!(
        (source.repository(), source.branch()),
        ("acme/app-cli", "release/v3")
    );
    let invalid_repository =
        |repository: &str| Err::<SkillSource, _>(Error::InvalidSkillRepository(repository.into()));
    for repository in [
        "",
        "app-cli",
        "/app-cli",
        "acme/",
        "/",
        "acme/app-cli/skills",
        "acme//app-cli",
        "https://github.com/acme/app-cli",
        "acme/app cli",
        "acme/app-cli\n",
        "\tacme/app-cli",
    ] {
        assert_eq!(
            SkillSource::new(repository, "main"),
            invalid_repository(repository)
        );
    }
    // The repository is checked first, so a branch alone names the repository.
    assert_eq!(SkillSource::new("", "main"), invalid_repository(""));
    assert_eq!(
        SkillSource::new("acme/app-cli", ""),
        Err(Error::MissingSkillBranch {
            repository: "acme/app-cli".into()
        })
    );
    assert_eq!(
        SkillSource::new("acme/app-cli", "my branch"),
        Err(Error::InvalidSkillBranch("my branch".into()))
    );
    for (error, field) in [
        (
            SkillSource::new("acme", "main").unwrap_err(),
            "skill repository",
        ),
        (
            SkillSource::new("acme/app-cli", "").unwrap_err(),
            "skill branch",
        ),
        (
            SkillSource::new("acme/app-cli", "a\tb").unwrap_err(),
            "skill branch",
        ),
    ] {
        assert!(error.to_string().starts_with(field), "{error}");
    }
}

#[test]
fn no_skill_source_adds_no_command_and_no_alert() {
    let configured = setup(root(), Mode::Agent, &HelpOptions::default()).unwrap();
    assert!(configured.find_subcommand("skill").is_none());
    let agent = render_help(&root(), &[], Mode::Agent, &HelpOptions::default()).unwrap();
    assert!(!agent.contains("ALERT"), "{agent}");
}

#[test]
fn alert_follows_the_generated_command_not_a_command_named_skill() {
    let own = root().subcommand(Command::new("skill").about("Manage skills"));
    let defaults = HelpOptions::default();
    let agent = render_help(&own, &[], Mode::Agent, &defaults).unwrap();
    assert!(!agent.contains("ALERT"), "{agent}");
    let mut configured = setup(own, Mode::Agent, &defaults).unwrap();
    let matches = configured
        .try_get_matches_from_mut(["app", "skill"])
        .unwrap();
    assert!(!write_skill(&configured, &matches, &defaults, &mut Vec::new()).unwrap());

    let options = options();
    for path in [&[][..], &["doc"], &["skill"]] {
        let agent = render_help(&root(), path, Mode::Agent, &options).unwrap();
        assert!(agent.starts_with(ALERT), "{agent}");
        assert_eq!(agent.matches("ALERT").count(), 1, "{agent}");
    }
    let human = render_help(&root(), &[], Mode::Human, &options).unwrap();
    assert!(!human.contains("ALERT"), "{human}");
}

#[test]
fn a_root_that_already_has_a_skill_command_is_rejected() {
    for duplicate in [
        Command::new("skill"),
        Command::new("guide").alias("skill"),
        Command::new("guide").visible_alias("skill"),
    ] {
        let tree = root().subcommand(duplicate);
        assert_eq!(
            setup(tree.clone(), Mode::Human, &options()).unwrap_err(),
            Error::SkillCollision
        );
        assert_eq!(
            render_help(&tree, &[], Mode::Agent, &options()).unwrap_err(),
            Error::SkillCollision
        );
    }
}

#[test]
fn hidden_generated_skill_command_still_prints_the_instructions() {
    let options = HelpOptions {
        tree: TreeOptions {
            hide_generated_commands: true,
            terminal_width: Some(0),
            ..Default::default()
        },
        ..options()
    };
    let human = render_help(&root(), &[], Mode::Human, &options).unwrap();
    assert!(!human.contains("skill"), "{human}");
    assert_eq!(
        String::from_utf8(run_skill(root(), &options)).unwrap(),
        INSTRUCTIONS
    );
}

#[test]
fn skill_omissions_include_the_generated_skill_command() {
    let text = "app\napp doc\n[name]\napp help\napp help doc\napp help help\n-h, --help bool";
    let bare = skill_omissions(&root(), &HelpOptions::default(), text);
    assert!(!bare.iter().any(|m| m.contains("skill")), "{bare:?}");
    let configured = skill_omissions(&root(), &options(), text);
    assert!(
        configured.contains(&"command app skill".into()),
        "{configured:?}"
    );
}
