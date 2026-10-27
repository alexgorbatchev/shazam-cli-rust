#[test]
fn guide_covers_the_live_native_command_tree() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = [
        "skills/shazam-cli/SKILL.md",
        "skills/shazam-cli/references/commands.md",
        "skills/shazam-cli/references/setup.md",
    ]
    .into_iter()
    .map(|p| std::fs::read_to_string(root.join(p)).unwrap())
    .collect::<Vec<_>>()
    .join("\n");
    let command = shazam_cli::cli::command(cli_command_kit::Mode::Agent).unwrap();
    let missing =
        cli_command_kit::skill_omissions(&command, &shazam_cli::cli::help_options(), &text);
    assert!(missing.is_empty(), "undocumented interface: {missing:?}");
    let skill = std::fs::read_to_string(root.join("skills/shazam-cli/SKILL.md")).unwrap();
    assert!(skill.contains("name: shazam-cli\n"));
}
