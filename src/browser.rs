use crate::{Error, Result, check_cancel};
use rookie_cookies::{AppBoundPolicy, CancellationHandle, ReadRequest, SendContext};
use tokio_util::sync::CancellationToken;

struct CancelRead(CancellationHandle);
impl Drop for CancelRead {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Resolve credentials without combining cookies from unrelated browser profiles.
pub fn resolve_with(
    manual: &str,
    environment: &str,
    browser: Option<&str>,
    profile: &str,
    mut read: impl FnMut(Option<&str>, &str) -> Result<String>,
) -> Result<String> {
    if !manual.is_empty() {
        return Ok(manual.to_owned());
    }
    if !environment.is_empty() {
        return Ok(environment.to_owned());
    }
    read(browser, profile)
}

pub fn header(snapshot: &rookie_cookies::ReadResult) -> Result<String> {
    let cookie = snapshot
        .header(
            &SendContext::url(crate::history::ENDPOINT).top_level_site("https://www.shazam.com"),
        )
        .map_err(|_| Error::Cookie)?;
    if cookie.is_empty() {
        Err(Error::Cookie)
    } else {
        Ok(cookie)
    }
}

fn read(browser: Option<&str>, profile: &str, handle: &CancellationHandle) -> Result<String> {
    let acquire = |id: &str, profile: &str| {
        let snapshot = rookie_cookies::read(
            ReadRequest::browser(id)
                .profile(profile)
                .app_bound(AppBoundPolicy::Disabled)
                .cancellation(handle.clone()),
        )
        .map_err(|_| Error::Cookie)?;
        header(&snapshot)
    };
    if let Some(id) = browser {
        return acquire(id, profile);
    }
    for browser in rookie_cookies::supported_browsers().map_err(|_| Error::Cookie)? {
        if handle.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let control = rookie_cookies::ExecutionControl::default()
            .app_bound(AppBoundPolicy::Disabled)
            .cancellation(handle.clone());
        if let Ok(profiles) = rookie_cookies::profiles_with(browser.id.as_str(), control) {
            for profile in profiles {
                if let Ok(cookie) =
                    acquire(browser.id.as_str(), profile.profile.profile_id.as_str())
                {
                    return Ok(cookie);
                }
            }
        }
    }
    Err(Error::Cookie)
}

pub async fn acquire(
    manual: &str,
    environment: &str,
    browser: Option<&str>,
    profile: &str,
    cancel: &CancellationToken,
) -> Result<String> {
    check_cancel(cancel)?;
    if !manual.is_empty() || !environment.is_empty() {
        return resolve_with(manual, environment, browser, profile, |_, _| {
            Err(Error::Cookie)
        });
    }
    let browser = browser.map(str::to_owned);
    let profile = profile.to_owned();
    let handle = CancellationHandle::new();
    let signal = handle.clone();
    let _guard = CancelRead(signal.clone());
    let mut job = tokio::task::spawn_blocking(move || read(browser.as_deref(), &profile, &handle));
    tokio::select! { biased;
        _=cancel.cancelled()=>{ signal.cancel(); let _=job.await; Err(Error::Cancelled) },
        result=&mut job=>result.map_err(|_|Error::Cookie)?,
    }
}
