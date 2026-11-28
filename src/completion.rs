use clap::builder::{EnumValueParser, TypedValueParser};
use clap::{Arg, ArgMatches, Command};
use clap_complete::Shell;
use std::any::TypeId;
use std::io::{self, Write};

const COMPLETION_COMMAND: &str = "completion";
const SHELL_ARG: &str = "__cli_command_kit_completion_shell";

// The private parser output distinguishes our command from consumer definitions.
#[derive(Clone)]
struct CompletionShell(Shell);

/// Define the built-in `completion <shell>` command for manual registration.
///
/// [crate::setup] registers it by default. The available shell names come
/// from the upstream completion generators.
pub fn completion_command() -> Command {
    Command::new(COMPLETION_COMMAND)
        .about("Generate a shell completion script")
        .arg(
            Arg::new(SHELL_ARG)
                .value_name("shell")
                .value_parser(EnumValueParser::<Shell>::new().map(CompletionShell))
                .required(true)
                .help("Shell to generate a completion script for"),
        )
}

pub(super) fn is_builtin(command: &Command) -> bool {
    command.get_name() == COMPLETION_COMMAND
        && command.get_arguments().any(|arg| {
            arg.get_id() == SHELL_ARG
                && arg.get_value_parser().type_id() == TypeId::of::<CompletionShell>()
        })
}

pub(super) fn add(root: &mut Command) -> Result<(), crate::Error> {
    match root.find_subcommand(COMPLETION_COMMAND) {
        Some(command) if is_builtin(command) => {}
        Some(_) => return Err(crate::Error::CompletionCollision),
        None => *root = root.clone().subcommand(completion_command()),
    }
    Ok(())
}

/// Write the script selected by matches from [completion_command].
///
/// Pass the complete root command so the script includes its arguments and
/// subcommands. Generation builds a private clone of the root. Returns
/// `false` without writing if the matches do not select the built-in command.
///
/// # Errors
///
/// Returns an error if completion matches lack a valid shell or writing fails.
pub fn write_completion(
    root: &Command,
    matches: &ArgMatches,
    output: &mut impl Write,
) -> io::Result<bool> {
    let Some((COMPLETION_COMMAND, completion)) = matches.subcommand() else {
        return Ok(false);
    };
    if !root
        .find_subcommand(COMPLETION_COMMAND)
        .is_some_and(is_builtin)
    {
        return Ok(false);
    }
    let CompletionShell(shell) = completion
        .try_get_one::<CompletionShell>(SHELL_ARG)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing completion shell"))?;
    let mut command = root.clone();
    let mut script = Vec::new();
    // Some generators panic on writer errors, so only give them a memory buffer.
    clap_complete::generate(*shell, &mut command, crate::command_path(root), &mut script);
    output.write_all(&script)?;
    Ok(true)
}
