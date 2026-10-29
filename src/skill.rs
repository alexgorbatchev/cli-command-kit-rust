use super::*;
use std::{
    collections::BTreeSet,
    io::{self, Write},
};

/// The GitHub repository that publishes a CLI's agent skill.
///
/// The skill lives at `skills/<cli>/SKILL.md` on the branch, where `<cli>` is the
/// root command's name, and its frontmatter `name` is that same `<cli>`. The
/// binary never carries a copy, so the installed skill is always the one the
/// repository holds. Construct it with [`SkillSource::new`], which rejects a
/// source that cannot locate a skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillSource {
    repository: String,
    branch: String,
}

impl SkillSource {
    /// Name a GitHub `owner/repo` and its default branch.
    ///
    /// The repository must be exactly two non-empty segments without whitespace,
    /// and the branch must be non-empty without whitespace, which Git rules out of
    /// branch names anyway. The repository is checked first, so a branch without
    /// a repository is reported as an invalid repository.
    pub fn new(repository: impl Into<String>, branch: impl Into<String>) -> Result<Self, Error> {
        let (repository, branch) = (repository.into(), branch.into());
        if !is_github_repository(&repository) {
            return Err(Error::InvalidSkillRepository(repository));
        }
        if branch.is_empty() {
            return Err(Error::MissingSkillBranch { repository });
        }
        if branch.chars().any(char::is_whitespace) {
            return Err(Error::InvalidSkillBranch(branch));
        }
        Ok(Self { repository, branch })
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// The text the skill command prints for the CLI named `cli`, ending in
    /// exactly one newline.
    fn instructions(&self, cli: &str) -> String {
        let Self { repository, branch } = self;
        format!(
            "Install the {cli} skill using the Skills CLI:\n\n  npx skills add {repository} --skill {cli}\n\nSkill:\n\n  https://github.com/{repository}/tree/{branch}/skills/{cli}\n"
        )
    }
}

/// Whether `repository` has the shape GitHub gives a repository: exactly two
/// non-empty segments and no whitespace.
fn is_github_repository(repository: &str) -> bool {
    repository.split_once('/').is_some_and(|(owner, name)| {
        !owner.is_empty()
            && !name.is_empty()
            && !name.contains('/')
            && !repository.chars().any(char::is_whitespace)
    })
}

/// Handle the skill command after clap parsing: print how to install the skill
/// that `options.skill` publishes for the CLI named by `root`, identically in
/// both modes, and propagate write failures.
///
/// Returns `false` without writing when the matches select another command or no
/// skill source is configured.
pub fn write_skill(
    root: &Command,
    matches: &clap::ArgMatches,
    options: &HelpOptions,
    output: &mut impl Write,
) -> io::Result<bool> {
    if matches.subcommand_name() != Some(SKILL_COMMAND) {
        return Ok(false);
    }
    let Some(source) = &options.skill else {
        return Ok(false);
    };
    output.write_all(source.instructions(root.get_name()).as_bytes())?;
    Ok(true)
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

/// Check a skill against clap's built interface, including generated commands,
/// inherited flags, parser types, defaults, argument labels, and environment.
/// A CLI's tests pass the `SKILL.md` read from its repository, the file the
/// skill command sends agents to install.
pub fn skill_omissions(root: &Command, options: &HelpOptions, text: &str) -> Vec<String> {
    let mut root = root.clone();
    // A consumer may check either a bare tree or an already configured tree.
    let mut missing = BTreeSet::new();
    if options.skill.is_some()
        && !root
            .get_subcommands()
            .any(|c| c.get_name() == SKILL_COMMAND)
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
