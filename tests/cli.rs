use cli_command_kit::Mode;
use serde_json::json;
use shazam_cli::{
    cli,
    history::{HistoryClient, parse_tracks},
    storage::Store,
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn cached_library_formats_exports_and_cache_commands_need_no_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("library.json"));
    store
        .save(&parse_tracks(&json!([{"title":"Song","artist":"Artist"}])).unwrap())
        .unwrap();
    let client = HistoryClient::new("http://127.0.0.1:1/").unwrap();
    let cancel = CancellationToken::new();
    for (args, contains) in [
        (
            vec!["shazam-cli", "-o", "query", "--suffix", "Extended Mix"],
            "Artist - Song Extended Mix",
        ),
        (vec!["shazam-cli", "library", "list"], "title: Song"),
        (vec!["shazam-cli", "cache", "status"], "tracks: 1"),
        (vec!["shazam-cli", "--version"], "0.1.0\n"),
        (vec!["shazam-cli", "skill"], cli::SKILL),
    ] {
        let matches = cli::command(Mode::Agent)
            .unwrap()
            .try_get_matches_from(args)
            .unwrap();
        let mut out = vec![];
        let mut err = vec![];
        cli::execute_with(
            &matches,
            Mode::Agent,
            &cancel,
            &store,
            &client,
            &mut out,
            &mut err,
        )
        .await
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains(contains));
    }
    let path = dir.path().join("export.json");
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "library", "export", path.to_str().unwrap()])
        .unwrap();
    let mut out = vec![];
    cli::execute_with(
        &matches,
        Mode::Agent,
        &cancel,
        &store,
        &client,
        &mut out,
        &mut vec![],
    )
    .await
    .unwrap();
    assert!(path.exists());
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "cache", "clear"])
        .unwrap();
    cli::execute_with(
        &matches,
        Mode::Agent,
        &cancel,
        &store,
        &client,
        &mut out,
        &mut vec![],
    )
    .await
    .unwrap();
    assert!(store.load().unwrap().is_none());
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "cache", "status"])
        .unwrap();
    out.clear();
    cli::execute_with(
        &matches,
        Mode::Agent,
        &cancel,
        &store,
        &client,
        &mut out,
        &mut vec![],
    )
    .await
    .unwrap();
    assert!(String::from_utf8(out).unwrap().contains("status: empty"));
}

#[tokio::test]
async fn fresh_session_refreshes_cache_and_no_save_preserves_previous_snapshot() {
    use axum::{Json, Router, routing::get};
    let app = Router::new().route(
        "/",
        get(|| async { Json(json!({"tags":[{"track":{"title":"Fresh","subtitle":"Artist"}}]})) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("library.json"));
    let client = HistoryClient::new(format!("http://{addr}/")).unwrap();
    let cancel = CancellationToken::new();
    for extra in [vec!["--no-save"], vec![]] {
        let mut args = vec![
            "shazam-cli",
            "--cookie",
            "session=FIXTURE",
            "--no-cache",
            "-o",
            "query",
        ];
        args.extend(extra);
        let matches = cli::command(Mode::Agent)
            .unwrap()
            .try_get_matches_from(args)
            .unwrap();
        let mut out = vec![];
        cli::execute_with(
            &matches,
            Mode::Agent,
            &cancel,
            &store,
            &client,
            &mut out,
            &mut vec![],
        )
        .await
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "Artist - Fresh\n");
    }
    assert_eq!(store.load().unwrap().unwrap()[0].title, "Fresh");
    std::fs::write(store.path(), b"bad JSON").unwrap();
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "--cookie", "session=FIXTURE"])
        .unwrap();
    let mut diagnostics = vec![];
    cli::execute_with(
        &matches,
        Mode::Agent,
        &cancel,
        &store,
        &client,
        &mut vec![],
        &mut diagnostics,
    )
    .await
    .unwrap();
    assert!(
        String::from_utf8(diagnostics)
            .unwrap()
            .contains("WARN: cache unavailable")
    );
    task.abort();
}

#[test]
fn native_binary_version_help_and_errors_are_streamed_once() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_shazam-cli");
    for agent in ["0", "1"] {
        let version = Command::new(binary)
            .args(["--version"])
            .env("AGENT", agent)
            .output()
            .unwrap();
        assert!(version.status.success());
        assert_eq!(version.stdout, b"0.1.0\n");
        let skill = Command::new(binary)
            .arg("skill")
            .env("AGENT", agent)
            .output()
            .unwrap();
        assert!(skill.status.success());
        assert_eq!(skill.stdout, cli::SKILL.as_bytes());
        for path in [
            vec!["--help"],
            vec!["library", "--help"],
            vec!["library", "list", "--help"],
            vec!["library", "export", "--help"],
            vec!["cache", "--help"],
            vec!["cache", "status", "--help"],
            vec!["cache", "clear", "--help"],
            vec!["skill", "--help"],
            vec!["help", "library"],
        ] {
            let help = Command::new(binary)
                .args(path)
                .env("AGENT", agent)
                .output()
                .unwrap();
            assert!(help.status.success());
            if agent == "1" {
                assert!(
                    String::from_utf8(help.stdout)
                        .unwrap()
                        .starts_with("ALERT: Agents must read `AGENT=1 shazam-cli skill`")
                );
            }
        }
    }
    for args in [
        vec!["skill", "extra"],
        vec!["--output", "invalid"],
        vec!["library", "export"],
        vec!["library", "list", "extra"],
    ] {
        let error = Command::new(binary).args(args).output().unwrap();
        assert_eq!(error.status.code(), Some(2));
        assert!(error.stdout.is_empty());
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("shazam-cli/library.json")).unwrap();
    let error = Command::new(binary)
        .args(["cache", "clear"])
        .env("XDG_CACHE_HOME", dir.path())
        .output()
        .unwrap();
    assert_eq!(error.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(error.stderr)
            .unwrap()
            .matches("ERR:")
            .count(),
        1
    );
}
