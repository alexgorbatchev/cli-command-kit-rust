//! Full command-tree help for clap, with human and deterministic agent output.
//! Renderers build private clones; callers may share their command definitions.

mod agent;
mod completion;
mod human;
mod messages;
mod skill;
mod wrap;
pub use messages::{display_message, fold_message};

pub use clap;
use clap::{Arg, ArgAction, Command};
pub use completion::{completion_command, write_completion};
pub use skill::{SkillSource, skill_omissions, write_skill};
use std::{collections::BTreeMap, fmt, path::Path};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Human,
    Agent,
}

impl Mode {
    pub fn from_value(value: &str) -> Self {
        if ["1", "true", "yes"].contains(&value.trim().to_ascii_lowercase().as_str()) {
            Self::Agent
        } else {
            Self::Human
        }
    }

    pub fn from_env() -> Self {
        Self::from_value(&std::env::var("AGENT").unwrap_or_default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct TreeOptions {
    pub include_root: bool,
    /// Zero selects two cells.
    pub min_padding: usize,
    /// Zero selects twenty cells.
    pub min_label_width: usize,
    /// The width human help wraps to, never cutting text. None detects the terminal;
    /// Some(0) leaves lines unwrapped, as does an unknown width.
    pub terminal_width: Option<usize>,
    pub hide_generated_commands: bool,
}

impl TreeOptions {
    fn padding(&self) -> usize {
        if self.min_padding == 0 {
            2
        } else {
            self.min_padding
        }
    }
    fn label_width(&self) -> usize {
        if self.min_label_width == 0 {
            20
        } else {
            self.min_label_width
        }
    }
    fn width(&self) -> usize {
        self.terminal_width.unwrap_or_else(terminal_width)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Spec {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct QuickstartItem {
    pub command: String,
    pub comment: String,
}

#[derive(Clone, Debug, Default)]
pub struct TechInfo {
    pub summary: String,
    pub description: String,
    pub args: Vec<Spec>,
    pub env: Vec<Spec>,
    pub quickstart: Vec<QuickstartItem>,
    pub mutates_db: bool,
    pub auto_backup: bool,
    pub metadata: BTreeMap<String, String>,
}

pub type TechCatalog = BTreeMap<String, TechInfo>;

#[derive(Clone, Debug, Default)]
pub struct HelpOptions {
    pub catalog: TechCatalog,
    pub tree: TreeOptions,
    pub disable_agent: bool,
    /// The GitHub repository that publishes the CLI's agent skill. When set, the
    /// root gains a `skill` command with instructions and an installer, and every
    /// agent screen opens with an alert naming that command. `None` adds neither.
    pub skill: Option<SkillSource>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    UnknownCommand(String),
    SkillCollision,
    /// The skill repository is not a GitHub `owner/repo`.
    InvalidSkillRepository(String),
    /// The skill source names a repository but no branch.
    MissingSkillBranch {
        repository: String,
    },
    /// The skill branch contains whitespace.
    InvalidSkillBranch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(path) => write!(f, "unknown command path: {path}"),
            Self::SkillCollision => {
                f.write_str("the root already has a command named or aliased skill")
            }
            Self::InvalidSkillRepository(repository) => write!(
                f,
                "skill repository must be a GitHub \"owner/repo\", got {repository:?}"
            ),
            Self::MissingSkillBranch { repository } => {
                write!(
                    f,
                    "skill branch must name the default branch of {repository}"
                )
            }
            Self::InvalidSkillBranch(branch) => {
                write!(
                    f,
                    "skill branch must not contain whitespace, got {branch:?}"
                )
            }
        }
    }
}
impl std::error::Error for Error {}

/// Install clap-native help overrides throughout the real command tree.
/// clap retains responsibility for parsing arguments and choosing output streams.
pub fn setup(mut root: Command, mode: Mode, options: &HelpOptions) -> Result<Command, Error> {
    add_skill(&mut root, options)?;
    root.build();
    let snapshot = root.clone();
    install_help(&mut root, &snapshot, &mut Vec::new(), mode, options)?;
    Ok(root)
}

fn install_help(
    command: &mut Command,
    root: &Command,
    path: &mut Vec<String>,
    mode: Mode,
    options: &HelpOptions,
) -> Result<(), Error> {
    let segments: Vec<_> = path.iter().map(String::as_str).collect();
    let output = render_built(root, &segments, mode, options)?;
    *command = command.clone().override_help(output);
    for child in command.get_subcommands_mut() {
        path.push(child.get_name().to_owned());
        install_help(child, root, path, mode, options)?;
        path.pop();
    }
    Ok(())
}

/// The command an agent runs to learn how to install the skill. The alert names
/// it, so it is fixed rather than configurable.
const SKILL_COMMAND: &str = "skill";

fn add_skill(root: &mut Command, options: &HelpOptions) -> Result<(), Error> {
    if options.skill.is_some() {
        if root.get_subcommands().any(|c| {
            c.get_name() == SKILL_COMMAND || c.get_all_aliases().any(|a| a == SKILL_COMMAND)
        }) {
            return Err(Error::SkillCollision);
        }
        *root = root.clone().subcommand(
            Command::new(SKILL_COMMAND)
                .about("Print instructions or install the agent skill")
                .disable_help_subcommand(true)
                .subcommand(skill::install_command()),
        );
    }
    Ok(())
}

/// Render a command selected by canonical names or aliases relative to root.
pub fn render_help(
    root: &Command,
    path: &[&str],
    mode: Mode,
    options: &HelpOptions,
) -> Result<String, Error> {
    let mut root = root.clone();
    add_skill(&mut root, options)?;
    root.build();
    render_built(&root, path, mode, options)
}

pub fn render_usage(
    root: &Command,
    path: &[&str],
    mode: Mode,
    options: &HelpOptions,
) -> Result<String, Error> {
    let mut root = root.clone();
    add_skill(&mut root, options)?;
    root.build();
    let command = select(&root, path)?;
    let output = if mode == Mode::Agent && !options.disable_agent {
        agent::render(&root, command, options)
    } else {
        human::render(command, options, false)
    };
    Ok(output)
}

fn render_built(
    root: &Command,
    path: &[&str],
    mode: Mode,
    options: &HelpOptions,
) -> Result<String, Error> {
    let command = select(root, path)?;
    if mode == Mode::Agent && !options.disable_agent {
        Ok(agent::render(root, command, options))
    } else {
        Ok(human::render(command, options, true))
    }
}

fn select<'a>(root: &'a Command, path: &[&str]) -> Result<&'a Command, Error> {
    let mut command = root;
    for segment in path {
        command = command
            .find_subcommand(segment)
            .ok_or_else(|| Error::UnknownCommand(path.join(" ")))?;
    }
    Ok(command)
}

fn command_path(command: &Command) -> &str {
    command.get_bin_name().unwrap_or(command.get_name())
}

fn visible(command: &Command, hide_generated: bool, has_skill: bool) -> Vec<&Command> {
    let mut children: Vec<_> = command
        .get_subcommands()
        .filter(|c| {
            // clap itself distinguishes generated help by the parent's setting
            // (Command::_propagate_global_args). Its cloned completion subtree is
            // not another set of executable domain commands.
            let generated_help =
                c.get_name() == "help" && !command.is_disable_help_subcommand_set();
            !c.is_hide_set()
                && !generated_help
                && !(hide_generated && has_skill && c.get_name() == SKILL_COMMAND)
        })
        .collect();
    children.sort_by(|a, b| {
        (a.get_display_order(), a.get_name()).cmp(&(b.get_display_order(), b.get_name()))
    });
    children
}

fn positional_label(arg: &Arg) -> String {
    let names = arg
        .get_value_names()
        .map(|names| names.iter().map(ToString::to_string).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![arg.get_id().to_string()]);
    let label = names.join(" ");
    let suffix = if arg.get_num_args().is_some_and(|n| n.max_values() > 1)
        || matches!(arg.get_action(), ArgAction::Append)
    {
        "..."
    } else {
        ""
    };
    if arg.is_required_set() {
        format!("<{label}>{suffix}")
    } else {
        format!("[{label}]{suffix}")
    }
}

fn command_label(command: &Command) -> String {
    let args = command
        .get_positionals()
        .map(positional_label)
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        command.get_name().to_owned()
    } else {
        format!("{} {args}", command.get_name())
    }
}

fn summary(command: &Command) -> String {
    command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default()
}
fn help(arg: &Arg) -> String {
    arg.get_help().map(ToString::to_string).unwrap_or_default()
}

fn arg_type(arg: &Arg, info: &TechInfo) -> String {
    if let Some(name) = info.metadata.get(&format!("type:{}", arg.get_id())) {
        return name.clone();
    }
    if !arg.get_action().takes_values() {
        return if matches!(arg.get_action(), ArgAction::Count) {
            "uint8"
        } else {
            "bool"
        }
        .into();
    }
    let id = arg.get_value_parser().type_id();
    macro_rules! type_names { ($($ty:ty => $name:literal),* $(,)?) => { $( if id == std::any::TypeId::of::<$ty>() { return $name.into(); } )* }; }
    type_names!(String => "string", std::ffi::OsString => "os-string", std::path::PathBuf => "path", i8 => "int8", i16 => "int16", i32 => "int32", i64 => "int64", isize => "int", u8 => "uint8", u16 => "uint16", u32 => "uint32", u64 => "uint64", usize => "uint", f32 => "float32", f64 => "float64", bool => "bool", std::time::Duration => "duration");
    // clap's type ID intentionally has no stable name for custom parsers.
    "value".into()
}

fn flag_label(arg: &Arg) -> String {
    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(long)) => format!("-{short}, --{long}"),
        (_, Some(long)) => format!("--{long}"),
        (Some(short), _) => format!("-{short}"),
        _ => arg.get_id().to_string(),
    }
}

fn flags(command: &Command) -> Vec<&Arg> {
    let mut args: Vec<_> = command
        .get_arguments()
        .filter(|a| !a.is_positional() && !a.is_hide_set())
        .collect();
    args.sort_by_key(|a| {
        (
            a.get_display_order(),
            a.get_long()
                .map(str::to_owned)
                .unwrap_or_else(|| a.get_id().to_string()),
        )
    });
    args
}

fn defaults(arg: &Arg) -> Option<String> {
    if arg.is_hide_default_value_set() {
        return None;
    }
    let values: Vec<_> = arg
        .get_default_values()
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values.join(","))
    }
}

fn info(command: &Command, options: &HelpOptions) -> TechInfo {
    options
        .catalog
        .get(command_path(command))
        .cloned()
        .unwrap_or_default()
}

fn argument_specs(
    command: &Command,
    info: &TechInfo,
    description: fn(&Arg) -> String,
) -> Vec<Spec> {
    if !info.args.is_empty() {
        return info.args.clone();
    }
    command
        .get_positionals()
        .filter(|a| !a.is_hide_set())
        .map(|arg| Spec {
            name: positional_label(arg),
            description: description(arg),
        })
        .collect()
}

fn environment_specs(command: &Command, info: &TechInfo) -> Vec<Spec> {
    let mut entries: BTreeMap<_, _> = command
        .get_arguments()
        .filter(|a| !a.is_hide_set())
        .filter_map(|a| {
            a.get_env()
                .map(|name| (name.to_string_lossy().into_owned(), help(a)))
        })
        .collect();
    entries.extend(
        info.env
            .iter()
            .map(|s| (s.name.clone(), s.description.clone())),
    );
    entries
        .into_iter()
        .map(|(name, description)| Spec { name, description })
        .collect()
}

fn usage(command: &Command) -> String {
    let mut clone = command.clone();
    clone
        .render_usage()
        .to_string()
        .trim_start_matches("Usage: ")
        .trim()
        .to_owned()
}

pub fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .or_else(|| terminal_size::terminal_size().map(|(w, _)| usize::from(w.0)))
        .unwrap_or(0)
}

/// Display-only home folding using whole path segments.
pub fn fold_home(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(tail) if tail.as_os_str().is_empty() => "~".into(),
        Ok(tail) => format!("~{}{}", std::path::MAIN_SEPARATOR, tail.display()),
        Err(_) => path.display().to_string(),
    }
}

pub fn display_path(path: &Path) -> String {
    dirs::home_dir().map_or_else(|| path.display().to_string(), |home| fold_home(path, &home))
}
