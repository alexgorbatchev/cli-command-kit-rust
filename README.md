`cli-command-kit` gives Rust CLI authors aligned command-tree help, deterministic agent output, and embedded usage guides. It describes the real command interface, including inherited flags and nested commands.

# What It Does

- **Complete trees**: Shows every visible descendant of a command group.
- **Two output modes**: Produces human-readable columns or compact agent metadata.
- **Usage guides**: Prints supplied guide text verbatim and checks it for interface omissions.
- **Native metadata**: Includes argument requirements, environment variables, possible values, parser types, and defaults.
- **Terminal-aware layout**: Measures Unicode display cells and clips descriptions without cutting command labels.

# How It Works

1. Supply your command definition and optional descriptions, examples, and usage guide.
2. Select human or agent output, explicitly or from `AGENT`.
3. Install the help screens before parsing arguments, or render individual screens into strings.
4. Handle a configured `skill` command with `write_skill`, which writes the exact guide bytes and returns write failures.

# How it Really Works

- Rendering works on private copies of command definitions. It performs no network requests or file writes.
- Human output detects terminal width from `COLUMNS`, then the terminal, unless you specify a width. Introductory descriptions wrap to that width while retaining paragraph breaks. `Some(0)` disables wrapping and clipping. Oversized labels and very narrow fields remain complete so commands and metadata keys stay identifiable.
- Agent output is untruncated by default. Optional clipping shortens values while preserving keys, and the guide-reading alert always remains complete.
- Environment values are read when constructing the native command definition. `Mode::from_env()` reads `AGENT` when called; `1`, `true`, and `yes` select agent mode, ignoring case and surrounding whitespace.
- Hidden commands and flags are omitted. Generated help routes are not repeated as copies of the domain-command tree. Guide coverage includes generated help routes because the parser accepts them.
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

# API

| Export | Purpose |
| :--- | :--- |
| `setup` | Installs help screens and an optional `skill` command; returns a command definition or a skill-collision error. |
| `render_help`, `render_usage` | Renders a root or descendant selected by canonical names or aliases; unknown paths return an error. |
| `write_skill` | Writes the configured guide when parsed matches select `skill`; returns whether it handled the command. |
| `skill_omissions` | Lists missing commands, flags, shorthands, types, defaults, arguments, and environment variables in sorted order. |
| `Mode` | Selects human or agent rendering. |
| `HelpOptions`, `TechCatalog`, `TechInfo` | Supplies formatting, command metadata, examples, and optional guide text. |
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
| `agent.max_line_width` | `None` | Keeps agent values complete; a positive value enables clipping. |
| `disable_agent` | `false` | Uses human rendering regardless of the supplied mode. |
| `skill` | `None` | Adds a guide command and an agent-mode guide-reading alert. |

# License

[MIT License](LICENSE), copyright 2026 Alex Gorbatchev.
