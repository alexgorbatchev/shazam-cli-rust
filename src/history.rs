use crate::{Error, Result, check_cancel};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const ENDPOINT: &str = "https://www.shazam.com/services/profile/v1/en-US/US/web/profile";
const MAX_BODY: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub key: String,
    pub title: String,
    pub artist: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover_art: String,
}

fn text(value: &Value, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

fn parse_list(list: &[Value]) -> Vec<Track> {
    list.iter()
        .filter_map(|item| {
            let record = item.get("track").filter(|v| v.is_object()).unwrap_or(item);
            let title = text(record, &["title", "name"]);
            if title.is_empty() {
                return None;
            }
            let timestamp = item.get("timestamp").and_then(|v| {
                v.as_i64()
                    .or_else(|| {
                        v.as_f64()
                            .filter(|n| {
                                n.is_finite() && *n >= i64::MIN as f64 && *n < i64::MAX as f64
                            })
                            .map(|n| n.trunc() as i64)
                    })
                    .and_then(|n| {
                        if n > 100_000_000_000 {
                            DateTime::from_timestamp_millis(n)
                        } else {
                            DateTime::from_timestamp(n, 0)
                        }
                    })
                    .or_else(|| {
                        v.as_str().and_then(|s| {
                            DateTime::parse_from_rfc3339(s)
                                .ok()
                                .map(|t| t.with_timezone(&Utc))
                        })
                    })
            });
            let mut artist = text(record, &["subtitle", "artist"]);
            if artist.is_empty() {
                artist = text(&record["artists"][0], &["name"]);
            }
            Some(Track {
                key: text(item, &["key", "id"]),
                title,
                artist,
                timestamp,
                url: text(record, &["url"]),
                cover_art: text(&record["images"], &["coverart", "default"]),
            })
        })
        .collect()
}

fn find_tracks(value: &Value, empty: &mut bool) -> Option<(Vec<Track>, usize)> {
    if let Some(list) = value.as_array() {
        return list.iter().find_map(|v| find_tracks(v, empty));
    }
    let object = value.as_object()?;
    for name in ["tags", "tracks", "library", "history", "items"] {
        if let Some(child) = object.get(name) {
            if let Some(list) = child.as_array() {
                if list.is_empty() {
                    *empty = true;
                } else {
                    let parsed = parse_list(list);
                    if !parsed.is_empty() {
                        return Some((parsed, list.len()));
                    }
                }
            }
            if let Some(found) = find_tracks(child, empty) {
                return Some(found);
            }
        }
    }
    object
        .iter()
        .filter(|(k, _)| !["tags", "tracks", "library", "history", "items"].contains(&k.as_str()))
        .find_map(|(_, v)| find_tracks(v, empty))
}

fn parse_page(value: &Value) -> Result<(Vec<Track>, usize)> {
    if let Some(list) = value.as_array() {
        return Ok((parse_list(list), list.len()));
    }
    let mut empty = false;
    if let Some(found) = find_tracks(value, &mut empty) {
        return Ok(found);
    }
    if empty {
        Ok((Vec::new(), 0))
    } else {
        Err(Error::Invalid(
            "no track collection in the Shazam response".into(),
        ))
    }
}
pub fn parse_tracks(value: &Value) -> Result<Vec<Track>> {
    parse_page(value).map(|(tracks, _)| tracks)
}

pub struct HistoryClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
}
impl HistoryClient {
    pub fn new(endpoint: impl AsRef<str>) -> Result<Self> {
        let endpoint = reqwest::Url::parse(endpoint.as_ref())
            .map_err(|_| Error::Invalid("invalid Shazam endpoint URL".into()))?;
        if !["https", "http"].contains(&endpoint.scheme())
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
        {
            return Err(Error::Invalid(
                "Shazam endpoint must be an HTTP(S) URL without credentials".into(),
            ));
        }
        Ok(Self { endpoint, http:reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(15)).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36").build()? })
    }
    pub async fn fetch_all(
        &self,
        cookie: &str,
        cancel: &CancellationToken,
        mut progress: impl FnMut(usize, usize) -> Result<()>,
    ) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();
        let mut offset = 0;
        loop {
            check_cancel(cancel)?;
            let request = self
                .http
                .get(self.endpoint.clone())
                .query(&[("limit", 100), ("offset", offset)])
                .header("accept", "application/json, text/plain, */*")
                .header("cookie", cookie);
            let response = tokio::select! { biased; _=cancel.cancelled()=>return Err(Error::Cancelled), response=request.send()=>response? };
            if response.status() != reqwest::StatusCode::OK {
                return Err(Error::Status(response.status().as_u16()));
            }
            if response
                .content_length()
                .is_some_and(|n| n > MAX_BODY as u64)
            {
                return Err(Error::TooLarge);
            }
            let mut body = Vec::new();
            let mut stream = response.bytes_stream();
            loop {
                let next = tokio::select! { biased; _=cancel.cancelled()=>return Err(Error::Cancelled), next=stream.next()=>next };
                let Some(chunk) = next else { break };
                let chunk = chunk?;
                if body.len().saturating_add(chunk.len()) > MAX_BODY {
                    return Err(Error::TooLarge);
                }
                body.extend_from_slice(&chunk);
            }
            let (page, count) = parse_page(&serde_json::from_slice(&body)?)?;
            if count == 0 {
                return Ok(tracks);
            }
            let fetched = page.len();
            tracks.extend(page);
            progress(fetched, tracks.len())?;
            offset += count;
            if count < 100 {
                return Ok(tracks);
            }
            tokio::select! { biased; _=cancel.cancelled()=>return Err(Error::Cancelled), _=tokio::time::sleep(Duration::from_millis(300))=>{} }
        }
    }
}
