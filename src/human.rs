use super::*;

type Row = (String, String);

fn rows(command: &Command, prefix: &str, options: &HelpOptions, output: &mut Vec<Row>) {
    let children = visible(
        command,
        options.tree.hide_generated_commands,
        options.skill.is_some(),
    );
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        let branch = if last { "╰─ " } else { "├─ " };
        output.push((
            format!("{prefix}{branch}{}", command_label(child)),
            summary(child),
        ));
        rows(
            child,
            &format!("{prefix}{}", if last { "   " } else { "│  " }),
            options,
            output,
        );
    }
}

fn specs(specs: &[Spec]) -> Vec<Row> {
    specs
        .iter()
        .map(|s| (format!("  {}", s.name), s.description.clone()))
        .collect()
}

fn write_rows(output: &mut String, rows: &[Row], column: usize, options: &TreeOptions) {
    for (label, description) in rows {
        // If the label itself exceeds the screen, retaining the complete command
        // remains more useful than a prefix that cannot be invoked.
        output.push_str(label);
        if !description.is_empty() {
            let padding = column
                .saturating_sub(UnicodeWidthStr::width(label.as_str()))
                .max(options.padding());
            output.push_str(&" ".repeat(padding));
            let budget = options.width().saturating_sub(column);
            output.push_str(&truncate(&description.replace(['\n', '\r'], " "), budget));
        }
        output.push('\n');
    }
}

fn append_block(
    output: &mut String,
    heading: &str,
    rows: &[Row],
    column: usize,
    options: &TreeOptions,
) {
    if !rows.is_empty() {
        output.push_str(&format!("\n{heading}:\n"));
        write_rows(output, rows, column, options);
    }
}

pub(super) fn render(command: &Command, options: &HelpOptions, description: bool) -> String {
    let mut output = String::new();
    let mut tree_options = options.tree.clone();
    tree_options.terminal_width = Some(tree_options.width());
    let info = info(command, options);
    if description {
        let about = command
            .get_long_about()
            .or_else(|| command.get_about())
            .map(ToString::to_string)
            .unwrap_or_default();
        if !about.is_empty() {
            if tree_options.width() > 0 {
                output.push_str(&textwrap::fill(
                    about.trim(),
                    textwrap::Options::new(tree_options.width()).break_words(false),
                ));
            } else {
                output.push_str(about.trim());
            }
            output.push_str("\n\n");
        }
    }
    output.push_str(&format!("Usage:\n  {}\n", usage(command)));
    let args = specs(&argument_specs(command, &info));
    let envs = specs(&environment_specs(command, &info));
    let mut tree = Vec::new();
    if tree_options.include_root {
        tree.push((command_label(command), summary(command)));
    }
    rows(command, "", options, &mut tree);
    let column = args
        .iter()
        .chain(&envs)
        .chain(&tree)
        .map(|(label, _)| UnicodeWidthStr::width(label.as_str()))
        .max()
        .unwrap_or(0)
        .max(tree_options.label_width())
        + tree_options.padding();
    append_block(&mut output, "Arguments", &args, column, &tree_options);
    append_block(
        &mut output,
        "Available Commands",
        &tree,
        column,
        &tree_options,
    );
    append_block(
        &mut output,
        "Environment Variables",
        &envs,
        column,
        &tree_options,
    );
    let (mut local, mut global) = (Vec::new(), Vec::new());
    for arg in flags(command) {
        let mut label = format!("  {}", flag_label(arg));
        if arg.get_action().takes_values() {
            let name = arg
                .get_value_names()
                .map(|names| {
                    names
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_else(|| arg_type(arg, &info));
            label.push_str(&format!(" <{name}>"));
        }
        let mut description = help(arg);
        if let Some(default) = defaults(arg) {
            description.push_str(&format!(" (default: {default:?})"));
        }
        let values: Vec<_> = arg
            .get_possible_values()
            .into_iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_owned())
            .collect();
        if !values.is_empty() {
            description.push_str(&format!(" [possible: {}]", values.join(", ")));
        }
        if arg.is_global_set() {
            global.push((label, description));
        } else {
            local.push((label, description));
        }
    }
    let flag_column = local
        .iter()
        .chain(&global)
        .map(|(label, _)| UnicodeWidthStr::width(label.as_str()))
        .max()
        .unwrap_or(0)
        + tree_options.padding();
    for (title, block) in [("Flags", local), ("Global Flags", global)] {
        if block.is_empty() {
            continue;
        }
        output.push_str(&format!("\n{title}:\n"));
        for (label, text) in block {
            let padding = flag_column.saturating_sub(UnicodeWidthStr::width(label.as_str()));
            let initial = format!("{label}{}", " ".repeat(padding));
            let width = tree_options.width();
            if width > flag_column {
                let wrapped = textwrap::fill(
                    &text,
                    textwrap::Options::new(width)
                        .initial_indent(&initial)
                        .subsequent_indent(&" ".repeat(flag_column))
                        .break_words(false),
                );
                output.push_str(&wrapped);
            } else {
                output.push_str(&initial);
                output.push_str(&text);
            }
            output.push('\n');
        }
    }
    let quickstart: Vec<_> = info
        .quickstart
        .iter()
        .map(|q| {
            let comment = q.comment.trim().trim_start_matches('#').trim();
            (
                format!("  {}", q.command),
                if comment.is_empty() {
                    String::new()
                } else {
                    format!("# {comment}")
                },
            )
        })
        .collect();
    let quick_column = quickstart
        .iter()
        .map(|(label, _)| UnicodeWidthStr::width(label.as_str()))
        .max()
        .unwrap_or(0)
        + tree_options.padding();
    append_block(
        &mut output,
        "Quickstart",
        &quickstart,
        quick_column,
        &tree_options,
    );
    if let Some(after) = command
        .get_after_long_help()
        .or_else(|| command.get_after_help())
    {
        output.push('\n');
        output.push_str(&after.to_string());
        output.push('\n');
    }
    if !tree.is_empty() {
        let hint = format!(
            "Use \"{} [command] --help\" for more information about a command.",
            command_path(command)
        );
        output.push('\n');
        if tree_options.width() > 0 {
            output.push_str(&textwrap::fill(
                &hint,
                textwrap::Options::new(tree_options.width()).break_words(false),
            ));
        } else {
            output.push_str(&hint);
        }
        output.push('\n');
    }
    output
}
