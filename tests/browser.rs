use shazam_cli::{Error, browser};
use tokio_util::sync::CancellationToken;

#[test]
fn cookie_precedence_and_single_profile_selection_are_explicit() {
    let mut calls = vec![];
    let mut reader = |browser: Option<&str>, profile: &str| {
        calls.push((browser.map(str::to_owned), profile.to_owned()));
        Ok("read=FIXTURE".into())
    };
    assert_eq!(
        browser::resolve_with("flag=X", "env=X", Some("chrome"), "P", &mut reader).unwrap(),
        "flag=X"
    );
    assert_eq!(
        browser::resolve_with("", "env=X", Some("chrome"), "P", &mut reader).unwrap(),
        "env=X"
    );
    assert_eq!(
        browser::resolve_with("", "", Some("chrome"), "P", &mut reader).unwrap(),
        "read=FIXTURE"
    );
    assert_eq!(
        browser::resolve_with("", "", None, "Default", &mut reader).unwrap(),
        "read=FIXTURE"
    );
    assert_eq!(
        calls,
        vec![
            (Some("chrome".into()), "P".into()),
            (None, "Default".into())
        ]
    );
}

#[test]
fn native_cookie_reader_obeys_domain_expiry_path_and_secure_scope() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cookies.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE moz_cookies (host TEXT,path TEXT,isSecure INTEGER,expiry INTEGER,name TEXT,value TEXT,isHttpOnly INTEGER,sameSite INTEGER);
      INSERT INTO moz_cookies VALUES ('.shazam.com','/',1,4102444800,'session','FIXTURE',1,0);
      INSERT INTO moz_cookies VALUES ('evilshazam.com','/',1,4102444800,'evil','DO_NOT_SEND',0,0);
      INSERT INTO moz_cookies VALUES ('.shazam.com','/',1,1,'expired','DO_NOT_SEND',0,0);
      INSERT INTO moz_cookies VALUES ('.shazam.com','/other',1,4102444800,'path','DO_NOT_SEND',0,0);").unwrap();
    drop(db);
    let snapshot = rookie_cookies::from_path(rookie_cookies::FromPathRequest::new(&path)).unwrap();
    assert_eq!(browser::header(&snapshot).unwrap(), "session=FIXTURE");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("DELETE FROM moz_cookies", []).unwrap();
    drop(db);
    let snapshot = rookie_cookies::from_path(rookie_cookies::FromPathRequest::new(&path)).unwrap();
    assert!(matches!(browser::header(&snapshot), Err(Error::Cookie)));
}

#[tokio::test]
async fn native_adapter_rejects_unknown_browser_and_observes_cancellation_without_accessing_profiles()
 {
    let cancel = CancellationToken::new();
    assert_eq!(
        browser::acquire("manual=X", "", None, "Default", &cancel)
            .await
            .unwrap(),
        "manual=X"
    );
    assert_eq!(
        browser::acquire("", "env=X", None, "Default", &cancel)
            .await
            .unwrap(),
        "env=X"
    );
    assert!(matches!(
        browser::acquire("", "", Some("not-a-real-browser"), "Default", &cancel).await,
        Err(Error::Cookie)
    ));
    cancel.cancel();
    assert!(matches!(
        browser::acquire("manual=X", "", None, "Default", &cancel).await,
        Err(Error::Cancelled)
    ));
}
