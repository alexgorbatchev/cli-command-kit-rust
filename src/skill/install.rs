use super::SkillSource;
use crate::display_path;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn skill(source: &SkillSource, cli: &str, target: &Path) -> io::Result<PathBuf> {
    if !safe_segment(cli) || !source.repository().split('/').all(safe_segment) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "skill name and GitHub repository must use valid path segments",
        ));
    }

    let destination = target.join(cli);
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err(destination_exists(&destination)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::create_dir_all(target)?;
    let staging = tempfile::Builder::new()
        .prefix(".skill-install-")
        .tempdir_in(target)
        .map_err(|error| io::Error::new(error.kind(), "cannot create a skill staging directory"))?;
    let relative = format!("skills/{cli}");
    let installed = staging.path().join(&relative);
    let url = format!("https://github.com/{}.git", source.repository());
    git(
        Command::new("git")
            .args(["clone", "--depth=1", "--filter=blob:none", "--no-checkout"])
            .arg(format!("--branch={}", source.branch()))
            .args(["--", &url])
            .arg(staging.path()),
        "fetch the configured skill repository and branch",
    )?;
    git(
        Command::new("git").arg("-C").arg(staging.path()).args([
            "--literal-pathspecs",
            "checkout",
            "HEAD",
            "--",
            &relative,
        ]),
        "check out the skill directory",
    )?;
    if !fs::symlink_metadata(&installed).is_ok_and(|metadata| metadata.is_dir())
        || !fs::symlink_metadata(installed.join("SKILL.md"))
            .is_ok_and(|metadata| metadata.is_file())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the configured GitHub branch does not contain a skill directory with a regular SKILL.md",
        ));
    }

    let mut entries = fs::read_dir(&installed)?;
    // Reserve the destination exclusively. A directory rename could replace an
    // existing empty destination on Unix, including a concurrent install's.
    fs::create_dir(&destination).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            destination_exists(&destination)
        } else {
            error
        }
    })?;
    if let Err(error) = entries.try_for_each(|entry| {
        let entry = entry?;
        fs::rename(entry.path(), destination.join(entry.file_name()))
    }) {
        fs::remove_dir_all(&destination)?;
        return Err(error);
    }
    Ok(destination)
}

fn safe_segment(segment: &str) -> bool {
    !matches!(segment, "" | "." | "..")
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn destination_exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("skill destination already exists: {}", display_path(path)),
    )
}

fn git(command: &mut Command, action: &str) -> io::Result<()> {
    let status = command
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| io::Error::new(error.kind(), format!("cannot run Git: {error}")))?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "Git could not {action} ({status})"
        )));
    }
    Ok(())
}
