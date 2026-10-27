pub mod browser;
pub mod cli;
pub mod history;
pub mod output;
pub mod storage;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("operation cancelled")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
    #[error("Shazam returned HTTP {0}; the response body is withheld")]
    Status(u16),
    #[error("Shazam response exceeds the 8 MiB limit")]
    TooLarge,
    #[error("browser cookie lookup failed; use --cookie or SHAZAM_COOKIE with an existing session")]
    Cookie,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

pub fn check_cancel(cancel: &tokio_util::sync::CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

pub fn display(path: &std::path::Path) -> String {
    cli_command_kit::display_path(path)
}
