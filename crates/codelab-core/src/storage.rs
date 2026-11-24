use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::error::{DomainError, ErrorCode};
use crate::model::{GLOBAL_DATA_SCHEMA_VERSION, StorageRecoveryNotice, StoreKind};

static BACKUP_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    path: PathBuf,
    kind: StoreKind,
    _marker: std::marker::PhantomData<T>,
}

impl<T> JsonStore<T>
where
    T: Default + Serialize + DeserializeOwned,
{
    pub(crate) fn new(data_dir: &Path, filename: &str, kind: StoreKind) -> Self {
        Self {
            path: data_dir.join(filename),
            kind,
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn load_or_recover<F>(&self, is_valid: F) -> Result<LoadOutcome<T>, DomainError>
    where
        F: Fn(&T) -> bool,
    {
        if !self.path.exists() {
            return Ok(LoadOutcome {
                value: T::default(),
                recovery: None,
            });
        }

        // Read raw bytes so invalid UTF-8 corruption triggers quarantine & recovery
        // instead of aborting startup with a read error.
        let bytes = fs::read(&self.path).map_err(|err| {
            DomainError::new(ErrorCode::StorageReadFailed)
                .with_detail("store", store_name(self.kind))
                .with_detail("reason", err.to_string())
        })?;

        match serde_json::from_slice::<VersionedEnvelope<T>>(&bytes) {
            Ok(envelope)
                if envelope.schema_version == GLOBAL_DATA_SCHEMA_VERSION
                    && is_valid(&envelope.data) =>
            {
                Ok(LoadOutcome {
                    value: envelope.data,
                    recovery: None,
                })
            }
            _ => {
                let backup_path = self.quarantine_corrupt_file()?;
                let default_value = T::default();
                self.save_atomic(&default_value)?;
                Ok(LoadOutcome {
                    value: default_value,
                    recovery: Some(StorageRecoveryNotice {
                        store: self.kind,
                        error_code: ErrorCode::StorageCorrupted,
                        backup_path: Some(backup_path.to_string_lossy().into_owned()),
                    }),
                })
            }
        }
    }

    pub(crate) fn save_atomic(&self, value: &T) -> Result<(), DomainError> {
        let parent = self.parent_dir()?;
        fs::create_dir_all(parent).map_err(|err| self.write_error(err))?;

        let envelope = VersionedEnvelope {
            schema_version: GLOBAL_DATA_SCHEMA_VERSION,
            data: value,
        };
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(|err| self.write_error(err))?;

        let mut tmp = NamedTempFile::new_in(parent).map_err(|err| self.write_error(err))?;
        tmp.write_all(&bytes).map_err(|err| self.write_error(err))?;
        tmp.write_all(b"\n").map_err(|err| self.write_error(err))?;
        tmp.as_file()
            .sync_all()
            .map_err(|err| self.write_error(err))?;

        // NamedTempFile::persist performs atomic replacement on both Unix (rename)
        // and Windows (MoveFileExW MOVEFILE_REPLACE_EXISTING) without deleting first.
        tmp.persist(&self.path)
            .map_err(|err| self.write_error(err.error))?;

        sync_parent_dir(parent);
        Ok(())
    }

    fn quarantine_corrupt_file(&self) -> Result<PathBuf, DomainError> {
        let parent = self.parent_dir()?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("store.json");
        let seq = BACKUP_SEQ.fetch_add(1, Ordering::Relaxed);
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_path = parent.join(format!("{file_name}.corrupt.{timestamp}.{seq}"));

        fs::rename(&self.path, &backup_path).map_err(|err| self.write_error(err))?;
        Ok(backup_path)
    }

    fn parent_dir(&self) -> Result<&Path, DomainError> {
        self.path.parent().ok_or_else(|| {
            DomainError::new(ErrorCode::StorageWriteFailed)
                .with_detail("store", store_name(self.kind))
        })
    }

    fn write_error(&self, err: impl std::fmt::Display) -> DomainError {
        DomainError::new(ErrorCode::StorageWriteFailed)
            .with_detail("store", store_name(self.kind))
            .with_detail("reason", err.to_string())
    }
}

#[cfg(unix)]
fn sync_parent_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_dir: &Path) {}

fn store_name(kind: StoreKind) -> &'static str {
    match kind {
        StoreKind::Settings => "settings",
        StoreKind::Recents => "recents",
        StoreKind::Trust => "trust",
    }
}
