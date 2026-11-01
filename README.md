`cli-command-kit` gives Rust CLI authors aligned command-tree help, deterministic agent output, and a `skill` command that tells AI agents how to install the CLI's skill. It describes the real command interface, including inherited flags and nested commands.

# What It Does

- **Complete trees**: Shows every visible descendant of a command group.
- **Two output modes**: Produces human-readable columns or compact agent metadata.
- **Agent skills**: Prints how to install the CLI's skill from its GitHub repository with the Skills CLI, and checks the skill for interface omissions.
- **Native metadata**: Includes argument requirements, environment variables, possible values, parser types, and defaults.
- **Terminal-aware layout**: Measures Unicode display cells and wraps help between words, with continuation lines aligned to their column. Text is never cut.

# How It Works

1. Supply your command definition and optional descriptions, examples, and the GitHub repository that publishes your agent skill.
2. Select human or agent output, explicitly or from `AGENT`.
3. Install the help screens before parsing arguments, or render individual screens into strings.
4. Handle a configured `skill` command with `write_skill`, which prints the install instructions and returns write failures.

# How it Really Works

- Rendering works on private copies of command definitions. It performs no network requests or file writes.
- Human output detects terminal width from `COLUMNS`, then the terminal, unless you specify a width. Every part of the screen wraps between words and keeps its indentation and paragraph breaks. Descriptions stay in their column, tree rows keep their branch lines, and a description moves under its label when its column is narrower than 20 cells. A word wider than the terminal stays whole on its own line. `Some(0)` disables wrapping, as does an unknown width.
- Agent output is always complete, and neither `COLUMNS` nor the terminal size changes it. Every line is a key, a list item, or a continuation indented beneath one: a multi-line value drops its blank lines, its tabs become spaces, and each further line is indented.
- The `skill` command prints the same text in both modes, where `<cli>` is the root command's name. The binary carries no copy of the skill, so the one an agent installs is always the one the repository holds at `skills/<cli>/SKILL.md`:

  ```text
  Install the <cli> skill using the Skills CLI:

    npx skills add <owner>/<repo> --skill <cli>

  Skill:

    https://github.com/<owner>/<repo>/tree/<branch>/skills/<cli>
  ```

  Every agent screen opens with ``ALERT: Agents must read `AGENT=1 <cli> skill` before using this tool.``; human screens do not. A root that already has a command named or aliased `skill` returns an error.
- Environment values are read when constructing the native command definition. `Mode::from_env()` reads `AGENT` when called; `1`, `true`, and `yes` select agent mode, ignoring case and surrounding whitespace.
- Hidden commands and flags are omitted. Generated help routes are not repeated as copies of the domain-command tree. Skill coverage includes generated help routes because the parser accepts them.
- A custom parser's type is reported as `value` unless the catalog supplies `type:<argument-id>` metadata. Explicit native defaults are included, including boolean defaults.
- `setup` configures requested help. Parse errors retain native argument diagnostics and compact usage; `render_usage` provides the complete usage screen for applications that print it themselves.
- Automatic-backup metadata advertises an override only when the command actually accepts `--no-backup`.

# Installation

From a sibling Rust project:

```sh
cargo add --path ../cli-command-kit
```

# Quick Start

```rust
use cli_command_kit::{render_help, HelpOptions, Mode};
use cli_command_kit::clap::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Command::new("notes")
        .about("Manage notes")
        .subcommand(Command::new("doc")
            .about("Documents")
            .subcommand(Command::new("list").about("List documents")));
    print!("{}", render_help(&root, &[], Mode::Human, &HelpOptions::default())?);
    Ok(())
}
```

Publish the skill at `skills/notes/SKILL.md` in the CLI's GitHub repository, then add the `skill` command:

```rust
use cli_command_kit::{setup, write_skill, HelpOptions, Mode, SkillSource};
use cli_command_kit::clap::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = HelpOptions {
        skill: Some(SkillSource::new("example/notes", "main")?),
        ..Default::default()
    };
    let mut root = setup(Command::new("notes"), Mode::from_env(), &options)?;
    let matches = root.get_matches_mut();
    write_skill(&root, &matches, &options, &mut std::io::stdout())?;
    Ok(())
}
```

Keep the skill complete with a test that reads it from the repository. `skill_omissions` lists every command, flag, shorthand, type, default, argument, and environment variable it does not mention; paths are relative to the package root:

```rust
use cli_command_kit::{skill_omissions, HelpOptions, SkillSource};
use cli_command_kit::clap::Command;

#[test]
fn skill_mentions_the_whole_interface() -> Result<(), Box<dyn std::error::Error>> {
    let options = HelpOptions {
        skill: Some(SkillSource::new("example/notes", "main")?),
        ..Default::default()
    };
    let skill = std::fs::read_to_string("skills/notes/SKILL.md")?;
    let missing = skill_omissions(&Command::new("notes"), &options, &skill);
    assert!(missing.is_empty(), "{missing:?}");
    Ok(())
}
```

# API

| Export | Purpose |
| :--- | :--- |
| `setup` | Installs help screens and an optional `skill` command; returns a command definition or a skill-collision error. |
| `render_help`, `render_usage` | Renders a root or descendant selected by canonical names or aliases; unknown paths return an error. |
| `write_skill` | Prints the install instructions for the root command when parsed matches select `skill`; returns whether it handled the command. |
| `SkillSource` | Names the GitHub `owner/repo` and branch that publish the skill; `SkillSource::new` rejects a malformed repository or branch. |
| `skill_omissions` | Lists missing commands, flags, shorthands, types, defaults, arguments, and environment variables in sorted order. |
| `Mode` | Selects human or agent rendering. |
| `HelpOptions`, `TechCatalog`, `TechInfo` | Supplies formatting, command metadata, examples, and the optional skill source. |
| `display_path`, `fold_home` | Folds a home-directory prefix on complete path-segment boundaries for display. |
| `display_message`, `fold_message` | Folds complete home paths inside diagnostics, preserving sibling prefixes and quoted or bracketed path boundaries. |
| `terminal_width` | Detects terminal width, returning zero when unavailable. |

# Configuration

| Option | Default | Effect |
| :--- | :--- | :--- |
| `tree.include_root` | `false` | Includes the selected command as a tree row. |
| `tree.min_padding` | `0` → `2` | Minimum cells between labels and descriptions. |
| `tree.min_label_width` | `0` → `20` | Minimum label-column width. |
| `tree.terminal_width` | `None` | Detects terminal width; `Some(0)` keeps descriptions complete. |
| `tree.hide_generated_commands` | `false` | Hides the library-provided skill command from human listings. |
| `disable_agent` | `false` | Uses human rendering regardless of the supplied mode. |
| `skill` | `None` | A `SkillSource`; adds a `skill` command that prints how to install the skill, and an agent-mode alert naming that command. |

# License

[MIT License](LICENSE), copyright 2026 Alex Gorbatchev.
