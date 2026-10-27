use super::*;
use std::{
    collections::BTreeSet,
    io::{self, Write},
};

/// Handle the skill command after clap parsing, preserving exact bytes and errors.
pub fn write_skill(
    matches: &clap::ArgMatches,
    options: &HelpOptions,
    output: &mut impl Write,
) -> io::Result<bool> {
    if matches.subcommand_name() != Some("skill") {
        return Ok(false);
    }
    if let Some(text) = &options.skill {
        output.write_all(text.as_bytes())?;
        return Ok(true);
    }
    Ok(false)
}

fn mentions(text: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    text.match_indices(word).any(|(start, _)| {
        let end = start + word.len();
        let continues = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
        !text[..start].chars().next_back().is_some_and(continues)
            && !text[end..].chars().next().is_some_and(continues)
    })
}

/// Check a guide against clap's built interface, including generated commands,
/// inherited flags, parser types, defaults, argument labels, and environment.
pub fn skill_omissions(root: &Command, options: &HelpOptions, text: &str) -> Vec<String> {
    let mut root = root.clone();
    // A consumer may check either a bare tree or an already configured tree.
    let mut missing = BTreeSet::new();
    if options.skill.is_some()
        && !root.get_subcommands().any(|c| c.get_name() == "skill")
        && let Err(error) = add_skill(&mut root, options)
    {
        missing.insert(error.to_string());
    }
    root.build();
    visit(&root, options, text, &mut missing);
    missing.into_iter().collect()
}

fn visit(command: &Command, options: &HelpOptions, text: &str, missing: &mut BTreeSet<String>) {
    let path = command_path(command);
    if !mentions(text, path) {
        missing.insert(format!("command {path}"));
    }
    let info = info(command, options);
    for arg in flags(command) {
        let name = arg
            .get_long()
            .map(|v| format!("--{v}"))
            .unwrap_or_else(|| flag_label(arg));
        if let Some(short) = arg.get_short()
            && !mentions(text, &format!("-{short}"))
        {
            missing.insert(format!("flag -{short}, the shorthand of {name}"));
        }
        let lines: Vec<_> = text.lines().filter(|line| mentions(line, &name)).collect();
        if lines.is_empty() {
            missing.insert(format!("flag {name}"));
            continue;
        }
        let type_name = arg_type(arg, &info);
        if !lines
            .iter()
            .any(|line| mentions(&line.to_lowercase(), &type_name.to_lowercase()))
        {
            missing.insert(format!(
                "flag {name}: its type {type_name} is not stated on a line that names it"
            ));
        }
        if let Some(default) = defaults(arg)
            && !lines.iter().any(|line| {
                if default.is_empty() {
                    mentions(&line.to_lowercase(), "empty") || line.contains("\"\"")
                } else {
                    mentions(line, &default)
                }
            })
        {
            missing.insert(format!(
                "flag {name}: its default {default:?} is not stated on a line that names it"
            ));
        }
    }
    for spec in argument_specs(command, &info) {
        if !text.contains(&spec.name) {
            missing.insert(format!("argument {} of {path}", spec.name));
        }
    }
    for spec in environment_specs(command, &info) {
        if !mentions(text, &spec.name) {
            missing.insert(format!("environment variable {}", spec.name));
        }
    }
    for child in command.get_subcommands().filter(|c| !c.is_hide_set()) {
        visit(child, options, text, missing);
    }
}
