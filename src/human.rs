use super::wrap::{cells, expand_tabs, split_indent, trim_trailing_space, wrap_prose, wrap_words};
use super::*;

/// The narrowest description column worth keeping. Below it a description reads as a
/// stack of single words, so it moves under its label.
const MIN_DESCRIPTION_CELLS: usize = 20;
const STACKED_INDENT: &str = "    ";
/// Indents the continuation of a label wider than the terminal.
const LABEL_HANG_INDENT: &str = "  ";
/// Flag labels start at most this far in, so a stacked description sits beneath them.
const FLAG_RAIL: &str = "    ";
const QUICKSTART_MARKER: &str = "# ";

#[derive(Default)]
struct Row {
    label: String,
    description: String,
    /// Starts every continuation line, so a tree's branch lines stay unbroken beside
    /// a description that wraps.
    rail: String,
    /// Opens every description line, so a wrapped comment stays a comment.
    marker: &'static str,
}

impl Row {
    /// The description stays in its column while the column is wide enough to read,
    /// and otherwise moves under the label. A label that wraps also moves it, since
    /// no single line then ends where the column starts.
    fn render(&self, column: usize, padding: usize, width: usize) -> String {
        let (indent, body) = split_indent(&self.label);
        let hang = format!("{}{LABEL_HANG_INDENT}", self.rail);
        let labels = wrap_words(indent, body, &hang, width);
        if self.description.is_empty() {
            return join_lines(&labels);
        }
        if width > 0 && (width.saturating_sub(column) < MIN_DESCRIPTION_CELLS || labels.len() > 1) {
            let prefix = format!("{}{STACKED_INDENT}{}", self.rail, self.marker);
            return join_lines(&labels) + &join_lines(&self.wrap(&prefix, &prefix, width));
        }
        let label = &labels[labels.len() - 1];
        let pad = column.saturating_sub(cells(label)).max(padding);
        let rail_pad = column.saturating_sub(cells(&self.rail));
        let first = format!("{label}{}{}", " ".repeat(pad), self.marker);
        let cont = format!("{}{}{}", self.rail, " ".repeat(rail_pad), self.marker);
        join_lines(&self.wrap(&first, &cont, width))
    }

    /// Wrap line by line, keeping the line breaks the author wrote. Only the first
    /// line of the first paragraph starts with `first`.
    fn wrap(&self, first: &str, cont: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut start = first;
        for line in self.description.lines() {
            let line = if width > 0 {
                expand_tabs(line)
            } else {
                line.to_owned()
            };
            lines.extend(wrap_words(start, line.trim_end(), cont, width));
            start = cont;
        }
        lines
    }
}

/// End each line with a newline. Trimming here keeps a line that holds only a rail
/// or padding from ending in spaces.
fn join_lines(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("{}\n", line.trim_end_matches(' ')))
        .collect()
}

fn rows(command: &Command, prefix: &str, options: &HelpOptions, output: &mut Vec<Row>) {
    let children = visible(
        command,
        options.tree.hide_generated_commands,
        options.skill.is_some(),
    );
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        let branch = if last { "╰─ " } else { "├─ " };
        let rail = format!("{prefix}{}", if last { "   " } else { "│  " });
        output.push(Row {
            label: format!("{prefix}{branch}{}", command_label(child)),
            description: summary(child),
            rail: rail.clone(),
            ..Row::default()
        });
        rows(child, &rail, options, output);
    }
}

fn specs(specs: &[Spec]) -> Vec<Row> {
    specs
        .iter()
        .map(|s| Row {
            label: format!("  {}", s.name),
            description: s.description.clone(),
            ..Row::default()
        })
        .collect()
}

fn append_possible_values(description: &mut String, arg: &Arg) {
    let values: Vec<_> = arg
        .get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect();
    if !values.is_empty() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(&format!("[possible: {}]", values.join(", ")));
    }
}

fn write_rows(output: &mut String, rows: &[Row], column: usize, options: &TreeOptions) {
    for row in rows {
        output.push_str(&row.render(column, options.padding(), options.width()));
    }
}

fn widest(rows: &[&Row]) -> usize {
    rows.iter().map(|row| cells(&row.label)).max().unwrap_or(0)
}

fn append_heading(output: &mut String, heading: &str, width: usize) {
    output.push('\n');
    output.push_str(&wrap_prose(&format!("{heading}:"), width));
    output.push('\n');
}

fn append_block(
    output: &mut String,
    heading: &str,
    rows: &[Row],
    column: usize,
    options: &TreeOptions,
) {
    if !rows.is_empty() {
        append_heading(output, heading, options.width());
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
            output.push_str(&wrap_prose(about.trim(), tree_options.width()));
            output.push_str("\n\n");
        }
    }
    output.push_str("Usage:\n");
    output.push_str(&wrap_prose(
        &format!("  {}", usage(command)),
        tree_options.width(),
    ));
    output.push('\n');
    let args = specs(&argument_specs(command, &info, |arg| {
        let mut description = help(arg);
        append_possible_values(&mut description, arg);
        description
    }));
    let envs = specs(&environment_specs(command, &info));
    let mut tree = Vec::new();
    if tree_options.include_root {
        tree.push(Row {
            label: command_label(command),
            description: summary(command),
            ..Row::default()
        });
    }
    rows(command, "", options, &mut tree);
    let column = widest(&args.iter().chain(&envs).chain(&tree).collect::<Vec<_>>())
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
        append_possible_values(&mut description, arg);
        let row = Row {
            label,
            description,
            rail: FLAG_RAIL.to_owned(),
            ..Row::default()
        };
        if arg.is_global_set() {
            global.push(row);
        } else {
            local.push(row);
        }
    }
    let flag_column =
        widest(&local.iter().chain(&global).collect::<Vec<_>>()) + tree_options.padding();
    for (title, block) in [("Flags", local), ("Global Flags", global)] {
        append_block(&mut output, title, &block, flag_column, &tree_options);
    }
    let quickstart: Vec<_> = info
        .quickstart
        .iter()
        .map(|q| Row {
            label: format!("  {}", q.command),
            description: q.comment.trim().trim_start_matches('#').trim().to_owned(),
            marker: QUICKSTART_MARKER,
            ..Row::default()
        })
        .collect();
    let quick_column = widest(&quickstart.iter().collect::<Vec<_>>()) + tree_options.padding();
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
        output.push_str(&wrap_prose(&after.to_string(), tree_options.width()));
        output.push('\n');
    }
    if !tree.is_empty() {
        let hint = format!(
            "Use \"{} [command] --help\" for more information about a command.",
            command_path(command)
        );
        output.push('\n');
        output.push_str(&wrap_prose(&hint, tree_options.width()));
        output.push('\n');
    }
    trim_trailing_space(&output)
}
