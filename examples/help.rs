use cli_command_kit::clap::Command;
use cli_command_kit::{HelpOptions, Mode, render_help};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Command::new("notes").about("Manage notes").subcommand(
        Command::new("doc")
            .about("Documents")
            .subcommand(Command::new("list").about("List documents")),
    );
    print!(
        "{}",
        render_help(&root, &[], Mode::Human, &HelpOptions::default())?
    );
    Ok(())
}
