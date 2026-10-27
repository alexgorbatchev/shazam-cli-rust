use cli_command_kit::Mode;
use serde_json::json;
use shazam_cli::{
    Error, cli,
    history::{HistoryClient, parse_tracks},
    output,
    storage::{Status, Store, atomic_write},
};
use tokio_util::sync::CancellationToken;

#[test]
fn malformed_collections_and_timestamps_are_handled_without_guessing() {
    let tracks=parse_tracks(&json!({"outer":[{"history":{"items":[
        {"track":{"name":"No Date","artists":[{"name":"Artist"}],"images":{"coverart":"cover"}},"timestamp":"invalid"},
        {"title":"Out of range","timestamp":9223372036854775807_i64}
    ]}}]})).unwrap();
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|t| t.timestamp.is_none()));
    assert_eq!(tracks[0].artist, "Artist");
    assert_eq!(tracks[0].cover_art, "cover");
    assert!(parse_tracks(&json!({"unrelated":[]})).is_err());
    for url in [
        "not a URL",
        "file:///etc/passwd",
        "https://user:password@example.com/",
    ] {
        assert!(HistoryClient::new(url).is_err());
    }
}

#[test]
fn failed_atomic_export_preserves_previous_bytes_and_storage_errors_are_visible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    std::fs::write(&path, b"previous").unwrap();
    let error = atomic_write(&path, |out| {
        use std::io::Write;
        out.write_all(b"partial")?;
        Err(Error::Invalid("test failure".into()))
    })
    .unwrap_err();
    assert!(matches!(error, Error::Invalid(_)));
    assert_eq!(std::fs::read(&path).unwrap(), b"previous");
    let store = Store::new(dir.path().join("directory"));
    std::fs::create_dir(store.path()).unwrap();
    assert!(store.load().is_err());
    assert!(store.save(&[]).is_err());
    assert!(store.status().is_err());
    assert!(store.clear().is_err());
    let blocked = Store::new(path.join("library.json"));
    assert!(blocked.load().is_err());
    assert!(blocked.save(&[]).is_err());
    assert!(blocked.status().is_err());
    assert!(output::export(&path.join("file"), &[]).is_err());
}

#[tokio::test]
async fn bad_http_bodies_and_partial_pages_do_not_replace_the_cache() {
    use axum::{
        Router,
        extract::{Path, Query},
        http::StatusCode,
        routing::get,
    };
    use std::collections::HashMap;
    let app=Router::new().route("/{mode}",get(|Path(mode):Path<String>,q:Query<HashMap<String,String>>|async move {
        match mode.as_str() {
            "large"=>(StatusCode::OK,"x".repeat(8*1024*1024+1)),
            "broken"=>(StatusCode::OK,"{bad json".into()),
            "partial" if q["offset"]=="0"=>(StatusCode::OK,json!({"tags":(0..100).map(|i|json!({"title":format!("{i}")})).collect::<Vec<_>>()} ).to_string()),
            _=>(StatusCode::UNAUTHORIZED,"session=SECRET_ONLY_RESPONSE".into())
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let cancel = CancellationToken::new();
    for (path, expected) in [("large", 0), ("broken", 1), ("partial", 2)] {
        let client = HistoryClient::new(format!("http://{addr}/{path}")).unwrap();
        let err = client
            .fetch_all("session=TEST_ONLY", &cancel, |_, _| Ok(()))
            .await
            .unwrap_err();
        assert!(match expected {
            0 => matches!(err, Error::TooLarge),
            1 => matches!(err, Error::Json(_)),
            _ => matches!(err, Error::Status(401)),
        });
        assert!(!err.to_string().contains("SECRET_ONLY_RESPONSE"));
    }
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("library.json"));
    store
        .save(&parse_tracks(&json!([{"title":"Original"}])).unwrap())
        .unwrap();
    let original = std::fs::read(store.path()).unwrap();
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "--no-cache", "--cookie", "session=TEST_ONLY"])
        .unwrap();
    let client = HistoryClient::new(format!("http://{addr}/partial")).unwrap();
    assert!(
        cli::execute_with(
            &matches,
            Mode::Agent,
            &cancel,
            &store,
            &client,
            &mut vec![],
            &mut vec![]
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), original);
    task.abort();
}

#[tokio::test]
async fn raw_offsets_include_filtered_entries_and_page_delay_is_cancellable() {
    use axum::{Json, Router, extract::Query, routing::get};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        time::Duration,
    };
    let calls = Arc::new(Mutex::new(vec![]));
    let seen = calls.clone();
    let app=Router::new().route("/",get(move |q:Query<HashMap<String,String>>|{let seen=seen.clone();async move {
        let offset=q["offset"].parse::<usize>().unwrap();seen.lock().unwrap().push(offset);
        Json(if offset==0 {json!({"tracks":(0..100).map(|i|if i==1 {json!({"invalid":true})}else{json!({"title":format!("Song {i}")})}).collect::<Vec<_>>()})}else{json!({"tags":[{"title":"Final"}]})})
    }}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = HistoryClient::new(format!("http://{addr}/")).unwrap();
    let cancel = CancellationToken::new();
    let tracks = client.fetch_all("x", &cancel, |_, _| Ok(())).await.unwrap();
    assert_eq!(tracks.len(), 100);
    assert_eq!(tracks.last().unwrap().title, "Final");
    assert_eq!(*calls.lock().unwrap(), vec![0, 100]);
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        client.fetch_all("x", &cancel, move |_, _| {
            signal.cancel();
            Ok(())
        }),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(Error::Cancelled)));
    task.abort();
}

#[tokio::test]
async fn corrupt_status_cache_write_warning_and_cancellation_are_reported() {
    use axum::{Router, routing::get};
    let app = Router::new().route("/", get(|| async { "{\"tags\":[]}" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    std::fs::write(&path, b"bad").unwrap();
    let store = Store::new(path.clone());
    let client = HistoryClient::new(format!("http://{addr}/")).unwrap();
    let cancel = CancellationToken::new();
    assert!(matches!(store.status().unwrap(), Status::Corrupt { .. }));
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "cache", "status"])
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
    assert!(String::from_utf8(out).unwrap().contains("status: corrupt"));
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["shazam-cli", "--no-cache", "--cookie", "test=X"])
        .unwrap();
    let mut err = vec![];
    cli::execute_with(
        &matches,
        Mode::Agent,
        &cancel,
        &store,
        &client,
        &mut vec![],
        &mut err,
    )
    .await
    .unwrap();
    assert!(
        String::from_utf8(err)
            .unwrap()
            .contains("WARN: cache write failed")
    );
    cancel.cancel();
    assert!(matches!(
        cli::execute_with(
            &matches,
            Mode::Agent,
            &cancel,
            &store,
            &client,
            &mut vec![],
            &mut vec![]
        )
        .await,
        Err(Error::Cancelled)
    ));
    assert_eq!(cli::exit_code(&Error::Cancelled), 130);
    assert_eq!(cli::exit_code(&Error::Cookie), 1);
    task.abort();
}
