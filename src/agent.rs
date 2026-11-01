use super::*;

const INDENT: &str = "  ";
/// The continuation indent of a value that already sits one level in.
const NESTED_INDENT: &str = "    ";

/// Make text safe to place after a key: blank lines go, tabs become spaces, and each
/// further line is indented by `cont`, so it reads as a continuation rather than as
/// a key of its own. Every line of agent help stays a key, a list item, or a
/// continuation beneath one.
fn value(text: &str, cont: &str) -> String {
    text.lines()
        .map(|line| line.replace('\t', " "))
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(&format!("\n{cont}"))
}

fn item(output: &mut String, depth: usize, name: &str, description: &str) {
    let indent = INDENT.repeat(depth + 1);
    output.push_str(&format!("{indent}- {}", name.trim()));
    let description = value(description, &format!("{indent}{INDENT}"));
    if !description.is_empty() {
        output.push_str(&format!(": {description}"));
    }
    output.push('\n');
}

fn subtree(command: &Command, depth: usize, options: &HelpOptions, output: &mut String) {
    for child in visible(command, false, options.skill.is_some()) {
        let info = info(child, options);
        let summary = if info.summary.is_empty() {
            summary(child)
        } else {
            info.summary
        };
        item(output, depth, &command_label(child), &summary);
        subtree(child, depth + 1, options, output);
    }
}

pub(super) fn render(root: &Command, command: &Command, options: &HelpOptions) -> String {
    let info = info(command, options);
    let mut output = format!("command: {}\n", command_path(command));
    let summary = if info.summary.is_empty() {
        summary(command)
    } else {
        info.summary.clone()
    };
    if !summary.is_empty() {
        output.push_str(&format!("summary: {}\n", value(&summary, INDENT)));
    }
    let description = if info.description.is_empty() {
        command
            .get_long_about()
            .map(ToString::to_string)
            .unwrap_or_default()
    } else {
        info.description.clone()
    };
    if !description.is_empty() && description.trim() != summary.trim() {
        output.push_str(&format!("description: {}\n", value(&description, INDENT)));
    }
    output.push_str(&format!("usage: {}\n", usage(command)));
    for (title, specs) in [
        ("args", argument_specs(command, &info)),
        ("env", environment_specs(command, &info)),
    ] {
        if !specs.is_empty() {
            output.push_str(&format!("{title}:\n"));
            for spec in specs {
                item(&mut output, 0, &spec.name, &spec.description);
            }
        }
    }
    let possible: Vec<_> = command
        .get_positionals()
        .flat_map(|a| a.get_possible_values())
        .filter(|v| !v.is_hide_set())
        .collect();
    if !possible.is_empty() {
        output.push_str("valid_args:\n");
        for value in possible {
            item(
                &mut output,
                0,
                value.get_name(),
                &value
                    .get_help()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            );
        }
    }
    if info.mutates_db {
        output.push_str("mutates_db: true\n");
    }
    if info.auto_backup {
        output.push_str("auto_backup: true");
        if command
            .get_arguments()
            .any(|arg| arg.get_long() == Some("no-backup"))
        {
            output.push_str(" (override with --no-backup)");
        }
        output.push('\n');
    }
    if !info.metadata.is_empty() {
        output.push_str("metadata:\n");
        for (key, value) in &info.metadata {
            output.push_str(&format!("  {key}: {}\n", self::value(value, NESTED_INDENT)));
        }
    }
    if !visible(command, false, options.skill.is_some()).is_empty() {
        output.push_str("subcommands:\n");
        subtree(command, 0, options, &mut output);
    }
    let flags = flags(command);
    if !flags.is_empty() {
        output.push_str("flags:\n");
        for arg in flags {
            output.push_str(&format!(
                "  {} {}: {}",
                flag_label(arg),
                arg_type(arg, &info),
                value(&help(arg), NESTED_INDENT)
            ));
            if let Some(default) = defaults(arg) {
                output.push_str(&format!(" (default: {default:?})"));
            }
            let possible: Vec<_> = arg
                .get_possible_values()
                .into_iter()
                .filter(|v| !v.is_hide_set())
                .map(|v| v.get_name().to_owned())
                .collect();
            if !possible.is_empty() {
                output.push_str(&format!(" (values: {})", possible.join(",")));
            }
            output.push('\n');
        }
    }
    if !info.quickstart.is_empty() {
        output.push_str("quickstart:\n");
        for q in &info.quickstart {
            item(
                &mut output,
                0,
                &q.command,
                q.comment.trim().trim_start_matches('#').trim(),
            );
        }
    }
    if options.skill.is_some() {
        output.insert_str(
            0,
            &format!(
                "ALERT: Agents must read `AGENT=1 {} skill` before using this tool.\n",
                command_path(root)
            ),
        );
    }
    output
}
