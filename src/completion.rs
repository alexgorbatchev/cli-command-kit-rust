use clap::{Arg, ArgMatches, Command, value_parser};
use clap_complete::Shell;
use std::io::{self, Write};

const COMPLETION_COMMAND: &str = "completion";
const SHELL_ARG: &str = "shell";

/// Define an opt-in `completion <shell>` command for a clap root.
///
/// Add it to the root before [crate::setup] to include it in help. The
/// available shell names come from the upstream completion generators.
pub fn completion_command() -> Command {
    Command::new(COMPLETION_COMMAND)
        .about("Generate a shell completion script")
        .arg(
            Arg::new(SHELL_ARG)
                .value_name(SHELL_ARG)
                .value_parser(value_parser!(Shell))
                .required(true)
                .help("Shell to generate a completion script for"),
        )
}

/// Write the script selected by matches from [completion_command].
///
/// Pass the complete root command so the script includes its arguments and
/// subcommands. Generation builds a private clone of the root. Returns
/// `false` without writing if the matches select another command.
///
/// # Errors
///
/// Returns an error if completion matches lack a valid shell or writing fails.
pub fn write_completion(
    root: &Command,
    matches: &ArgMatches,
    output: &mut impl Write,
) -> io::Result<bool> {
    let Some(completion) = matches.subcommand_matches(COMPLETION_COMMAND) else {
        return Ok(false);
    };
    let shell = completion
        .try_get_one::<Shell>(SHELL_ARG)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing completion shell"))?;
    let mut command = root.clone();
    let mut script = Vec::new();
    // Some generators panic on writer errors, so only give them a memory buffer.
    clap_complete::generate(*shell, &mut command, crate::command_path(root), &mut script);
    output.write_all(&script)?;
    Ok(true)
}
