use crate::{Result, history::Track};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub struct Store {
    path: PathBuf,
}
#[derive(Debug)]
pub enum Status {
    Empty,
    Corrupt {
        bytes: u64,
    },
    Ready {
        tracks: usize,
        bytes: u64,
        modified: std::time::SystemTime,
    },
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn standard() -> Result<Self> {
        let base = dirs::cache_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".cache")))
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "cache directory unavailable")
            })?;
        Ok(Self::new(base.join("shazam-cli/library.json")))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> Result<Option<Vec<Track>>> {
        match std::fs::read(&self.path) {
            Ok(data) => Ok(Some(serde_json::from_slice(&data)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self, tracks: &[Track]) -> Result<()> {
        atomic_write(&self.path, |out| {
            serde_json::to_writer_pretty(&mut *out, tracks)?;
            writeln!(out)?;
            Ok(())
        })
    }
    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn status(&self) -> Result<Status> {
        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Status::Empty),
            Err(e) => return Err(e.into()),
        };
        match self.load() {
            Ok(Some(t)) => Ok(Status::Ready {
                tracks: t.len(),
                bytes: metadata.len(),
                modified: metadata.modified()?,
            }),
            Err(crate::Error::Json(_)) => Ok(Status::Corrupt {
                bytes: metadata.len(),
            }),
            Err(e) => Err(e),
            Ok(None) => Ok(Status::Empty),
        }
    }
}

pub fn atomic_write(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> Result<()>,
) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    write(staged.as_file_mut())?;
    staged.as_file_mut().sync_all()?;
    staged.persist(path).map_err(|e| e.error)?;
    Ok(())
}
