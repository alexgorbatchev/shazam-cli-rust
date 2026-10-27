use serde_json::json;
use shazam_cli::{
    Error,
    history::{HistoryClient, parse_tracks},
    output::{self, Format},
    storage::Store,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[test]
fn parses_nested_flat_empty_and_timestamp_variants_without_losing_tracks() {
    let tracks = parse_tracks(&json!({"tags":[],"nested":{"tracks":[
        {"id":"one","timestamp":1700000000000_i64,"name":"雪","artists":[{"name":"Artist"}],"images":{"default":"https://cover"}},
        {"key":"two","timestamp":"2023-11-14T22:13:20Z","track":{"title":"Second","subtitle":"Other","url":"https://song"}},
        {"title":"Third","timestamp":1700000000,"artist":"Solo"}, {"invalid":true}
    ]}})).unwrap();
    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks[0].title, "雪");
    assert_eq!(tracks[0].artist, "Artist");
    assert_eq!(tracks[0].timestamp, tracks[1].timestamp);
    assert_eq!(tracks[1].timestamp, tracks[2].timestamp);
    assert_eq!(tracks[0].cover_art, "https://cover");
    assert_eq!(tracks[1].url, "https://song");
    assert!(parse_tracks(&json!({"tags":[]})).unwrap().is_empty());
    assert!(parse_tracks(&json!([])).unwrap().is_empty());
    assert!(parse_tracks(&json!({"user":{"email":"secret"}})).is_err());
    assert!(parse_tracks(&json!(null)).is_err());
}

#[test]
fn decimal_epoch_timestamps_keep_source_seconds_and_milliseconds_semantics() {
    let tracks = parse_tracks(&json!([
        {"title":"Seconds","timestamp":1700000000.9},
        {"title":"Milliseconds","timestamp":1700000000000.9},
        {"title":"Out of range","timestamp":1e30}
    ]))
    .unwrap();
    assert_eq!(tracks[0].timestamp.unwrap().timestamp(), 1700000000);
    assert_eq!(tracks[0].timestamp, tracks[1].timestamp);
    assert!(tracks[2].timestamp.is_none());
}

#[tokio::test]
async fn paginates_exact_full_page_then_empty_and_keeps_cookies_off_redirects() {
    use axum::{Json, Router, extract::Query, http::HeaderMap, routing::get};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    let app=Router::new().route("/profile",get(move |q:Query<HashMap<String,String>>, h:HeaderMap| {
        let seen=seen.clone(); async move {
            assert_eq!(h["cookie"],"session=TEST_ONLY"); assert_eq!(q["limit"],"100");
            let offset=q["offset"].parse::<usize>().unwrap(); seen.lock().unwrap().push(offset);
            Json(json!({"tags":if offset==0 {(0..100).map(|i|json!({"key":i.to_string(),"track":{"title":format!("Song {i}"),"subtitle":"Artist"}})).collect::<Vec<_>>()}else{vec![]}}))
        }
    })).route("/redirect",get(||async{(axum::http::StatusCode::FOUND,[("location","/profile")],"session=DO_NOT_PRINT")}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let cancel = CancellationToken::new();
    let client = HistoryClient::new(format!("http://{addr}/profile")).unwrap();
    let mut pages = vec![];
    let tracks = client
        .fetch_all("session=TEST_ONLY", &cancel, |n, t| {
            pages.push((n, t));
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(tracks.len(), 100);
    assert_eq!(*calls.lock().unwrap(), vec![0, 100]);
    assert_eq!(pages, vec![(100, 100)]);
    let err = HistoryClient::new(format!("http://{addr}/redirect"))
        .unwrap()
        .fetch_all("session=TEST_ONLY", &cancel, |_, _| Ok(()))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Status(302)));
    assert!(!err.to_string().contains("DO_NOT_PRINT"));
    cancel.cancel();
    assert!(matches!(
        client.fetch_all("x", &cancel, |_, _| Ok(())).await,
        Err(Error::Cancelled)
    ));
    task.abort();
}

#[test]
fn atomic_storage_exports_and_output_propagate_writer_failures() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("cache/library.json"));
    assert!(store.load().unwrap().is_none());
    store.clear().unwrap();
    let tracks =
        parse_tracks(&json!([{"title":"Snow, 雪","artist":"A\"B","timestamp":1700000000}]))
            .unwrap();
    store.save(&tracks).unwrap();
    assert_eq!(store.load().unwrap().unwrap(), tracks);
    assert!(matches!(
        store.status().unwrap(),
        shazam_cli::storage::Status::Ready { tracks: 1, .. }
    ));
    let json_path = dir.path().join("export/one.JSON");
    output::export(&json_path, &tracks).unwrap();
    assert_eq!(
        serde_json::from_slice::<Vec<shazam_cli::history::Track>>(
            &std::fs::read(&json_path).unwrap()
        )
        .unwrap(),
        tracks
    );
    let csv_path = dir.path().join("two.csv");
    output::export(&csv_path, &tracks).unwrap();
    let rows = csv::Reader::from_path(csv_path)
        .unwrap()
        .records()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(&rows[0][2], "Snow, 雪");
    for format in [Format::Table, Format::List, Format::Csv, Format::Query] {
        let mut out = Vec::new();
        output::write(&mut out, &tracks, format, "Extended Mix", false).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("Snow, 雪"));
        assert!(output::write(&mut std::io::sink(), &[], format, "", true).is_ok());
        struct Broken;
        impl std::io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(output::write(&mut Broken, &tracks, format, "", true).is_err());
    }
    std::fs::write(store.path(), b"corrupt").unwrap();
    assert!(store.load().is_err());
    assert!(matches!(
        store.status().unwrap(),
        shazam_cli::storage::Status::Corrupt { .. }
    ));
    store.clear().unwrap();
    assert!(matches!(
        store.status().unwrap(),
        shazam_cli::storage::Status::Empty
    ));
}

#[tokio::test]
async fn cancellation_interrupts_a_delayed_body() {
    use axum::{Router, routing::get};
    let app = Router::new().route(
        "/",
        get(|| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            "{\"tags\":[]}"
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let cancel = CancellationToken::new();
    let other = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        other.cancel()
    });
    let client = HistoryClient::new(format!("http://{addr}/")).unwrap();
    assert!(matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            client.fetch_all("x", &cancel, |_, _| Ok(()))
        )
        .await
        .unwrap(),
        Err(Error::Cancelled)
    ));
    task.abort();
}
