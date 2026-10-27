use std::path::Path;

#[test]
fn diagnostics_fold_only_complete_home_path_segments() {
    let home = Path::new("/home/alex");
    assert_eq!(
        cli_command_kit::fold_message("[/home/alex]; /home/alex;", home),
        "[~]; ~;"
    );
    assert_eq!(cli_command_kit::fold_message("/home/alex", home), "~");
    assert_eq!(
        cli_command_kit::fold_message("failed '/home/alex/music/a' then /home/alex/b", home),
        "failed '~/music/a' then ~/b"
    );
    assert_eq!(
        cli_command_kit::fold_message("/home/alexander/a /var/tmp/a x/home/alex/a", home),
        "/home/alexander/a /var/tmp/a x/home/alex/a"
    );
    assert_eq!(
        cli_command_kit::fold_message("path=/home/alex, (/home/alex/a)", home),
        "path=~, (~/a)"
    );
    assert_eq!(
        cli_command_kit::fold_message("literal", Path::new("")),
        "literal"
    );
    if let Some(home) = dirs::home_dir() {
        assert_eq!(
            cli_command_kit::display_message(&format!(
                "failed {}",
                home.join("audio.flac").display()
            )),
            "failed ~/audio.flac"
        );
    }
}
