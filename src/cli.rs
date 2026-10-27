use crate::{
    Error, Result, browser,
    history::{ENDPOINT, HistoryClient},
    output::{self, Format},
    storage::{Status, Store},
};
use clap::{Arg, ArgAction, ArgMatches, Command};
use cli_command_kit::{HelpOptions, Mode};
use std::{io::Write, path::PathBuf};
use tokio_util::sync::CancellationToken;

pub const SKILL: &str = "Install the shazam-cli skill using the Skills CLI:\n\n  npx skills add alexgorbatchev/shazam-cli --skill shazam-cli\n\nSkill:\n\n  https://github.com/alexgorbatchev/shazam-cli/tree/main/skills/shazam-cli\n";
pub fn help_options() -> HelpOptions {
    HelpOptions {
        skill: Some(SKILL.into()),
        ..Default::default()
    }
}
pub fn command(mode: Mode) -> Result<Command> {
    let mut root = Command::new("shazam-cli")
        .about("Read and export an existing Shazam library session")
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .long("version")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Print the raw version"),
        )
        .arg(
            Arg::new("cookie")
                .long("cookie")
                .short('c')
                .env("SHAZAM_COOKIE")
                .hide_env_values(true)
                .global(true)
                .help("Use this existing session cookie in memory"),
        )
        .arg(
            Arg::new("browser")
                .long("browser")
                .short('b')
                .global(true)
                .help("Read one browser's session cookie"),
        )
        .arg(
            Arg::new("profile")
                .long("profile")
                .short('p')
                .default_value("Default")
                .global(true)
                .help("Browser profile selector"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .default_value("table")
                .value_parser(clap::value_parser!(Format))
                .global(true)
                .help("Console format: table,list,csv,query"),
        )
        .arg(
            Arg::new("suffix")
                .long("suffix")
                .default_value("")
                .global(true)
                .help("Append text to query output"),
        )
        .subcommand(
            Command::new("library")
                .about("Read or export library tracks")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List library tracks"))
                .subcommand(
                    Command::new("export")
                        .about("Export tracks as JSON or CSV")
                        .arg(
                            Arg::new("file")
                                .value_name("FILE")
                                .required(true)
                                .value_parser(clap::value_parser!(PathBuf)),
                        ),
                ),
        )
        .subcommand(
            Command::new("cache")
                .about("Manage saved library metadata")
                .subcommand_required(true)
                .subcommand(Command::new("status").about("Inspect cache state and size"))
                .subcommand(Command::new("clear").about("Remove the saved library metadata")),
        );
    for (name, help) in [
        ("no-cache", "Fetch fresh tracks without reading the cache"),
        ("no-save", "Keep fetched tracks out of the cache"),
    ] {
        root = root.arg(
            Arg::new(name)
                .long(name)
                .action(ArgAction::SetTrue)
                .global(true)
                .help(help),
        );
    }
    cli_command_kit::setup(root, mode, &help_options()).map_err(|e| Error::Invalid(e.to_string()))
}

pub async fn execute(matches: &ArgMatches, mode: Mode, cancel: &CancellationToken) -> Result<()> {
    let store = Store::standard()?;
    let client = HistoryClient::new(ENDPOINT)?;
    execute_with(
        matches,
        mode,
        cancel,
        &store,
        &client,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
    .await
}

pub async fn execute_with(
    matches: &ArgMatches,
    mode: Mode,
    cancel: &CancellationToken,
    store: &Store,
    client: &HistoryClient,
    out: &mut dyn Write,
    diagnostics: &mut dyn Write,
) -> Result<()> {
    crate::check_cancel(cancel)?;
    if matches.get_flag("version") {
        writeln!(out, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(());
    }
    if let Some(("skill", _)) = matches.subcommand() {
        out.write_all(SKILL.as_bytes())?;
        return Ok(());
    }
    if let Some(("cache", sub)) = matches.subcommand() {
        writeln!(out, "cache: {}", crate::display(store.path()))?;
        match sub.subcommand_name() {
            Some("clear") => {
                store.clear()?;
                writeln!(out, "status: cleared")?;
            }
            _ => match store.status()? {
                Status::Empty => writeln!(out, "status: empty")?,
                Status::Corrupt { bytes } => writeln!(out, "status: corrupt\nbytes: {bytes}")?,
                Status::Ready {
                    tracks,
                    bytes,
                    modified,
                } => {
                    let modified: chrono::DateTime<chrono::Utc> = modified.into();
                    writeln!(
                        out,
                        "status: ready\ntracks: {tracks}\nbytes: {bytes}\nmodified: {}",
                        modified.to_rfc3339()
                    )?;
                }
            },
        }
        return Ok(());
    }
    let mut cached = None;
    if !matches.get_flag("no-cache") {
        match store.load() {
            Ok(value) => cached = value,
            Err(e) => writeln!(diagnostics, "WARN: cache unavailable ({})", safe_error(&e))?,
        }
    }
    let tracks = if let Some(tracks) = cached {
        writeln!(
            diagnostics,
            "Using cached Shazam library ({} tracks)",
            tracks.len()
        )?;
        tracks
    } else {
        let cookie = browser::acquire(
            matches
                .get_one::<String>("cookie")
                .map(String::as_str)
                .unwrap_or_default(),
            "",
            matches.get_one::<String>("browser").map(String::as_str),
            matches.get_one::<String>("profile").unwrap(),
            cancel,
        )
        .await?;
        writeln!(diagnostics, "Fetching Shazam library")?;
        let tracks = client
            .fetch_all(&cookie, cancel, |fetched, total| {
                writeln!(diagnostics, "Retrieved {fetched} tracks (total: {total})")?;
                Ok(())
            })
            .await?;
        if !matches.get_flag("no-save")
            && let Err(e) = store.save(&tracks)
        {
            writeln!(diagnostics, "WARN: cache write failed ({})", safe_error(&e))?;
        }
        tracks
    };
    if let Some(("library", sub)) = matches.subcommand()
        && let Some(("export", sub)) = sub.subcommand()
    {
        let path = sub.get_one::<PathBuf>("file").unwrap();
        output::export(path, &tracks)?;
        writeln!(
            out,
            "Exported {} tracks to: {}",
            tracks.len(),
            crate::display(path)
        )?;
        return Ok(());
    }
    output::write(
        out,
        &tracks,
        *matches.get_one::<Format>("output").unwrap(),
        matches.get_one::<String>("suffix").unwrap(),
        mode == Mode::Agent,
    )
}

/// Browser and HTTP errors with potentially sensitive context are already redacted.
/// Filesystem diagnostics use their native kind rather than embedding absolute paths.
pub fn safe_error(error: &Error) -> String {
    match error {
        Error::Io(e) => format!("filesystem error: {}", e.kind()),
        _ => error.to_string(),
    }
}
pub fn exit_code(error: &Error) -> u8 {
    if matches!(error, Error::Cancelled) {
        130
    } else {
        1
    }
}
