use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cap_std::fs::Dir;
use cap_tempfile::TempFile;
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::error::{DomainError, ErrorCode};
use crate::filesystem::open_regular;
use crate::model::{GLOBAL_DATA_SCHEMA_VERSION, StorageRecoveryNotice, StoreKind};

static BACKUP_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedEnvelope<T> {
    schema_version: u32,
    data: T,
}

#[derive(Debug)]
pub(crate) struct LoadOutcome<T> {
    pub value: T,
    pub recovery: Option<StorageRecoveryNotice>,
}

pub(crate) struct JsonStore<T> {
    directory: PathBuf,
    name: &'static str,
    scope: Option<Arc<Dir>>,
    kind: StoreKind,
    _marker: std::marker::PhantomData<T>,
}

impl<T> JsonStore<T>
where
    T: Serialize + DeserializeOwned,
{
    pub(crate) fn new(data_dir: &Path, filename: &'static str, kind: StoreKind) -> Self {
        Self {
            directory: data_dir.to_owned(),
            name: filename,
            scope: None,
            kind,
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn scoped(mut self, directory: Arc<Dir>) -> Self {
        self.scope = Some(directory);
        self
    }

    pub(crate) fn load_or_recover(
        &self,
        is_valid: impl Fn(&T) -> bool,
    ) -> Result<LoadOutcome<T>, DomainError>
    where
        T: Default,
    {
        self.load_or_recover_with(T::default(), is_valid)
    }

    pub(crate) fn load_or_recover_with(
        &self,
        default: T,
        is_valid: impl Fn(&T) -> bool,
    ) -> Result<LoadOutcome<T>, DomainError> {
        let bytes = match self.read_bytes() {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(LoadOutcome {
                    value: default,
                    recovery: None,
                });
            }
            Err(err) => return Err(self.error(ErrorCode::StorageReadFailed, err)),
        };
        // Raw bytes include invalid UTF-8, and an empty buffer here also covers symlinks.
        if let Ok(envelope) = serde_json::from_slice::<VersionedEnvelope<T>>(&bytes)
            && envelope.schema_version == GLOBAL_DATA_SCHEMA_VERSION
            && is_valid(&envelope.data)
        {
            return Ok(LoadOutcome {
                value: envelope.data,
                recovery: None,
            });
        }

        let backup_path = self.quarantine_corrupt_file()?;
        self.save_atomic(&default)?;
        Ok(LoadOutcome {
            value: default,
            recovery: Some(StorageRecoveryNotice {
                store: self.kind,
                error_code: ErrorCode::StorageCorrupted,
                backup_path: Some(backup_path.to_string_lossy().into_owned()),
            }),
        })
    }

    fn read_bytes(&self) -> io::Result<Vec<u8>> {
        let path = self.directory.join(self.name);
        if let Some(dir) = &self.scope {
            let name = Path::new(self.name);
            let metadata = dir.symlink_metadata(name)?;
            if metadata.is_symlink() {
                return Ok(Vec::new());
            }
            let mut bytes = Vec::new();
            open_regular(dir, name)?.read_to_end(&mut bytes)?;
            return Ok(bytes);
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_symlink() {
            return Ok(Vec::new());
        }
        if !metadata.is_file() {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        fs::read(path)
    }

    pub(crate) fn save_atomic(&self, value: &T) -> Result<(), DomainError> {
        let envelope = VersionedEnvelope {
            schema_version: GLOBAL_DATA_SCHEMA_VERSION,
            data: value,
        };
        let mut bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|err| self.error(ErrorCode::StorageWriteFailed, err))?;
        bytes.push(b'\n');
        self.persist(&bytes)
            .map_err(|err| self.error(ErrorCode::StorageWriteFailed, err))
    }

    fn persist(&self, bytes: &[u8]) -> io::Result<()> {
        if let Some(dir) = &self.scope {
            // Keep all operations relative to the already opened directory. A renamed
            // parent or replaced symlink cannot redirect a metadata write.
            let mut temp = TempFile::new(dir)?;
            temp.write_all(bytes)?;
            temp.as_file().sync_all()?;
            temp.replace(self.name)?;
            #[cfg(unix)]
            if let Ok(dir) = dir.open(".") {
                let _ = dir.sync_all();
            }
        } else {
            fs::create_dir_all(&self.directory)?;
            let mut temp = NamedTempFile::new_in(&self.directory)?;
            temp.write_all(bytes)?;
            temp.as_file().sync_all()?;
            temp.persist(self.directory.join(self.name))
                .map_err(|err| err.error)?;
            #[cfg(unix)]
            if let Ok(dir) = fs::File::open(&self.directory) {
                let _ = dir.sync_all();
            }
        }
        Ok(())
    }

    fn quarantine_corrupt_file(&self) -> Result<PathBuf, DomainError> {
        let seq = BACKUP_SEQ.fetch_add(1, Ordering::Relaxed);
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.9fZ");
        let backup = format!("{}.corrupt.{timestamp}.{seq}", self.name);
        let backup_path = self.directory.join(&backup);
        let result = match &self.scope {
            Some(dir) => dir.rename(self.name, dir.as_ref(), backup),
            None => fs::rename(self.directory.join(self.name), &backup_path),
        };
        result.map_err(|err| self.error(ErrorCode::StorageWriteFailed, err))?;
        Ok(backup_path)
    }

    fn error(&self, code: ErrorCode, err: impl std::fmt::Display) -> DomainError {
        DomainError::new(code)
            .with_detail("store", store_name(self.kind))
            .with_detail("reason", err.to_string())
    }
}

fn store_name(kind: StoreKind) -> &'static str {
    match kind {
        StoreKind::Settings => "settings",
        StoreKind::RuntimePreferences => "runtimePreferences",
        StoreKind::Recents => "recents",
        StoreKind::Trust => "trust",
        StoreKind::Workspace => "workspace",
        StoreKind::Progress => "progress",
        StoreKind::Metrics => "metrics",
        StoreKind::TutorHistory => "tutorHistory",
    }
}
