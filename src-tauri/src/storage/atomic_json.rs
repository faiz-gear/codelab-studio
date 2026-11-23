use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::domain::error::{AppError, ErrorCode};
use crate::domain::events::StorageCorruptedEvent;

static NEXT_TMP_SEQ: AtomicU64 = AtomicU64::new(1);
static NEXT_CORRUPT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("storage JSON error at '{path}': {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid storage path '{path}'")]
    InvalidPath { path: PathBuf },
    #[error("storage domain error: {0}")]
    Domain(AppError),
}

impl From<StorageError> for AppError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Domain(app_err) => app_err,
            StorageError::Json { path, source } => {
                AppError::new(ErrorCode::SchemaValidationFailed, "error.invalidInput")
                    .with_param(
                        "field",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("storage"),
                    )
                    .with_detail(source.to_string())
            }
            StorageError::Io { path, source } => {
                AppError::new(ErrorCode::StorageCorruptedRecovered, "error.unknown")
                    .with_param("detail", format!("{}: {source}", path.display()))
                    .with_detail(source.to_string())
                    .with_retryable(true)
            }
            StorageError::InvalidPath { path } => {
                AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
                    .with_param("field", path.display().to_string())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome<T> {
    CreatedDefault(T),
    Loaded(T),
    Migrated {
        value: T,
        from_version: u64,
        backup_name: String,
    },
    RecoveredFromBackup {
        value: T,
        backup_name: String,
        reason: String,
    },
    ResetAfterCorruption {
        value: T,
        event: StorageCorruptedEvent,
        reason: String,
    },
}

impl<T> LoadOutcome<T> {
    pub fn value(&self) -> &T {
        match self {
            Self::CreatedDefault(v)
            | Self::Loaded(v)
            | Self::Migrated { value: v, .. }
            | Self::RecoveredFromBackup { value: v, .. }
            | Self::ResetAfterCorruption { value: v, .. } => v,
        }
    }

    pub fn into_value(self) -> T {
        match self {
            Self::CreatedDefault(v)
            | Self::Loaded(v)
            | Self::Migrated { value: v, .. }
            | Self::RecoveredFromBackup { value: v, .. }
            | Self::ResetAfterCorruption { value: v, .. } => v,
        }
    }

    pub fn corruption_event(&self) -> Option<&StorageCorruptedEvent> {
        match self {
            Self::ResetAfterCorruption { event, .. } => Some(event),
            _ => None,
        }
    }
}

enum ParsedDoc<T> {
    Current(T),
    Migrated { value: T, from_version: u64 },
}

#[derive(Debug, Clone)]
pub struct AtomicJsonStore {
    path: PathBuf,
}

impl AtomicJsonStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn backup_path(&self) -> Result<PathBuf, StorageError> {
        let name = self.file_name_str()?;
        Ok(self.path.with_file_name(format!("{name}.bak")))
    }

    pub fn write_atomic<T: Serialize + DeserializeOwned>(
        &self,
        value: &T,
    ) -> Result<(), StorageError> {
        self.write_atomic_internal(value, true)
    }

    pub fn read_valid<T: DeserializeOwned>(&self) -> Result<T, StorageError> {
        let bytes = fs::read(&self.path).map_err(|source| StorageError::Io {
            path: self.path.clone(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })
    }

    pub fn load_or_init<T, F>(&self, default_fn: F) -> Result<LoadOutcome<T>, StorageError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> T,
    {
        self.load_or_init_with_migrator(default_fn, |_, _| None)
    }

    pub fn load_or_init_with_migrator<T, F, M>(
        &self,
        default_fn: F,
        mut migrator: M,
    ) -> Result<LoadOutcome<T>, StorageError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> T,
        M: FnMut(u64, Value) -> Option<Result<T, String>>,
    {
        let file_name = self.file_name_str()?.to_owned();
        let bak_path = self.backup_path()?;
        let bak_name = bak_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| StorageError::InvalidPath {
                path: bak_path.clone(),
            })?
            .to_owned();

        if !self.path.exists() {
            if bak_path.exists() {
                match read_and_parse::<T, _>(&bak_path, &mut migrator) {
                    Ok(ParsedDoc::Current(value)) | Ok(ParsedDoc::Migrated { value, .. }) => {
                        self.write_atomic_internal(&value, false)?;
                        return Ok(LoadOutcome::RecoveredFromBackup {
                            value,
                            backup_name: bak_name,
                            reason: "primary file missing; restored from backup".to_owned(),
                        });
                    }
                    Err(bak_err) => {
                        let quarantined_bak = quarantine_corrupt_file(&bak_path)?;
                        let value = default_fn();
                        self.write_atomic_internal(&value, false)?;
                        return Ok(LoadOutcome::ResetAfterCorruption {
                            value,
                            event: StorageCorruptedEvent {
                                file_name,
                                backup_name: quarantined_bak,
                            },
                            reason: format!("primary missing and backup corrupt: {bak_err}"),
                        });
                    }
                }
            }

            let value = default_fn();
            self.write_atomic_internal(&value, false)?;
            return Ok(LoadOutcome::CreatedDefault(value));
        }

        let primary_bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(err) => {
                return Err(StorageError::Io {
                    path: self.path.clone(),
                    source: err,
                });
            }
        };

        match parse_or_migrate::<T, _>(&primary_bytes, &mut migrator) {
            Ok(ParsedDoc::Current(value)) => Ok(LoadOutcome::Loaded(value)),
            Ok(ParsedDoc::Migrated {
                value,
                from_version,
            }) => {
                write_bytes_atomically(&bak_path, &primary_bytes)?;
                self.write_atomic_internal(&value, false)?;
                Ok(LoadOutcome::Migrated {
                    value,
                    from_version,
                    backup_name: bak_name,
                })
            }
            Err(primary_err) => {
                if bak_path.exists() {
                    match read_and_parse::<T, _>(&bak_path, &mut migrator) {
                        Ok(ParsedDoc::Current(value)) | Ok(ParsedDoc::Migrated { value, .. }) => {
                            self.write_atomic_internal(&value, false)?;
                            return Ok(LoadOutcome::RecoveredFromBackup {
                                value,
                                backup_name: bak_name,
                                reason: primary_err,
                            });
                        }
                        Err(bak_err) => {
                            let quarantined_primary = quarantine_corrupt_file(&self.path)?;
                            let _ = quarantine_corrupt_file(&bak_path)?;
                            let value = default_fn();
                            self.write_atomic_internal(&value, false)?;
                            return Ok(LoadOutcome::ResetAfterCorruption {
                                value,
                                event: StorageCorruptedEvent {
                                    file_name,
                                    backup_name: quarantined_primary,
                                },
                                reason: format!("primary: {primary_err}; backup: {bak_err}"),
                            });
                        }
                    }
                }

                let quarantined_primary = quarantine_corrupt_file(&self.path)?;
                let value = default_fn();
                self.write_atomic_internal(&value, false)?;
                Ok(LoadOutcome::ResetAfterCorruption {
                    value,
                    event: StorageCorruptedEvent {
                        file_name,
                        backup_name: quarantined_primary,
                    },
                    reason: primary_err,
                })
            }
        }
    }

    fn write_atomic_internal<T: Serialize + DeserializeOwned>(
        &self,
        value: &T,
        update_backup_if_primary_valid: bool,
    ) -> Result<(), StorageError> {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })?;
        bytes.push(b'\n');

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorageError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let tmp_path = next_temp_path(&self.path)?;
        write_temp_and_sync(&tmp_path, &bytes)?;

        if update_backup_if_primary_valid && self.path.exists() {
            if let Ok(existing_bytes) = fs::read(&self.path) {
                if serde_json::from_slice::<T>(&existing_bytes).is_ok() {
                    let bak_path = self.backup_path()?;
                    if let Err(err) = write_bytes_atomically(&bak_path, &existing_bytes) {
                        let _ = fs::remove_file(&tmp_path);
                        return Err(err);
                    }
                }
            }
        }

        if let Err(source) = atomic_replace(&tmp_path, &self.path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(StorageError::Io {
                path: self.path.clone(),
                source,
            });
        }

        Ok(())
    }

    fn file_name_str(&self) -> Result<&str, StorageError> {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| StorageError::InvalidPath {
                path: self.path.clone(),
            })
    }
}

#[derive(Debug)]
pub struct AtomicJsonFile<T> {
    store: AtomicJsonStore,
    write_lock: Mutex<()>,
    _marker: PhantomData<T>,
}

impl<T> AtomicJsonFile<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(path: PathBuf) -> Self {
        Self {
            store: AtomicJsonStore::new(path),
            write_lock: Mutex::new(()),
            _marker: PhantomData,
        }
    }

    pub fn path(&self) -> &Path {
        self.store.path()
    }

    pub fn load_or_init<F>(&self, default_fn: F) -> Result<LoadOutcome<T>, StorageError>
    where
        F: FnOnce() -> T,
    {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.store.load_or_init(default_fn)
    }

    pub fn load_or_init_with_migrator<F, M>(
        &self,
        default_fn: F,
        migrator: M,
    ) -> Result<LoadOutcome<T>, StorageError>
    where
        F: FnOnce() -> T,
        M: FnMut(u64, Value) -> Option<Result<T, String>>,
    {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.store.load_or_init_with_migrator(default_fn, migrator)
    }

    pub fn read(&self) -> Result<T, StorageError> {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.store.read_valid()
    }

    pub fn write(&self, value: &T) -> Result<(), StorageError> {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.store.write_atomic(value)
    }

    pub fn update<R, F>(&self, mutator: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut T) -> Result<R, StorageError>,
    {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut current = self.store.read_valid::<T>()?;
        let output = mutator(&mut current)?;
        self.store.write_atomic(&current)?;
        Ok(output)
    }

    pub fn load_or_init_and_update<R, D, F>(
        &self,
        default_fn: D,
        mutator: F,
    ) -> Result<(R, Option<StorageCorruptedEvent>), StorageError>
    where
        D: FnOnce() -> T,
        F: FnOnce(&mut T) -> Result<R, StorageError>,
    {
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let outcome = self.store.load_or_init(default_fn)?;
        let corruption = outcome.corruption_event().cloned();
        let mut current = outcome.into_value();
        let output = mutator(&mut current)?;
        self.store.write_atomic(&current)?;
        Ok((output, corruption))
    }
}

pub fn cleanup_stale_tmp_files(dir: &Path) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut removed = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.contains(".json.tmp.") || name.contains(".bak.tmp.") {
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
                removed.push(name.to_owned());
            }
        }
    }
    removed.sort();
    Ok(removed)
}

fn read_and_parse<T, M>(path: &Path, migrator: &mut M) -> Result<ParsedDoc<T>, String>
where
    T: DeserializeOwned,
    M: FnMut(u64, Value) -> Option<Result<T, String>>,
{
    let bytes = fs::read(path).map_err(|err| format!("read failed: {err}"))?;
    parse_or_migrate(&bytes, migrator)
}

fn parse_or_migrate<T, M>(bytes: &[u8], migrator: &mut M) -> Result<ParsedDoc<T>, String>
where
    T: DeserializeOwned,
    M: FnMut(u64, Value) -> Option<Result<T, String>>,
{
    let raw: Value = serde_json::from_slice(bytes).map_err(|err| format!("invalid JSON: {err}"))?;
    let Some(version) = raw.get("schemaVersion").and_then(Value::as_u64) else {
        return Err("missing or non-integer schemaVersion".to_owned());
    };

    if version == 1 {
        let parsed: T = serde_json::from_value(raw)
            .map_err(|err| format!("schema v1 validation failed: {err}"))?;
        return Ok(ParsedDoc::Current(parsed));
    }

    match migrator(version, raw) {
        Some(Ok(value)) => Ok(ParsedDoc::Migrated {
            value,
            from_version: version,
        }),
        Some(Err(err)) => Err(format!(
            "migration from schemaVersion {version} failed: {err}"
        )),
        None => Err(format!("unsupported schemaVersion {version}")),
    }
}

fn next_temp_path(target: &Path) -> Result<PathBuf, StorageError> {
    let name =
        target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| StorageError::InvalidPath {
                path: target.to_path_buf(),
            })?;
    let pid = std::process::id();
    let seq = NEXT_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    Ok(target.with_file_name(format!("{name}.tmp.{pid}.{seq}")))
}

fn write_temp_and_sync(tmp_path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path)
        .map_err(|source| StorageError::Io {
            path: tmp_path.to_path_buf(),
            source,
        })?;

    if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(tmp_path);
        return Err(StorageError::Io {
            path: tmp_path.to_path_buf(),
            source,
        });
    }

    Ok(())
}

fn write_bytes_atomically(target: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source| StorageError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let tmp_path = next_temp_path(target)?;
    write_temp_and_sync(&tmp_path, bytes)?;
    if let Err(source) = atomic_replace(&tmp_path, target) {
        let _ = fs::remove_file(&tmp_path);
        return Err(StorageError::Io {
            path: target.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn quarantine_corrupt_file(path: &Path) -> Result<String, StorageError> {
    let base_name =
        path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| StorageError::InvalidPath {
                path: path.to_path_buf(),
            })?;
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let mut candidate_name = format!("{base_name}.corrupt.{unix_ms}");
    let mut candidate_path = path.with_file_name(&candidate_name);
    while candidate_path.exists() {
        let seq = NEXT_CORRUPT_SEQ.fetch_add(1, Ordering::Relaxed);
        candidate_name = format!("{base_name}.corrupt.{unix_ms}.{seq}");
        candidate_path = path.with_file_name(&candidate_name);
    }

    atomic_replace(path, &candidate_path).map_err(|source| StorageError::Io {
        path: candidate_path,
        source,
    })?;
    Ok(candidate_name)
}

#[cfg(unix)]
pub(crate) fn atomic_replace(src: &Path, dst: &Path) -> io::Result<()> {
    fs::rename(src, dst)?;
    if let Some(parent) = dst.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn atomic_replace(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let src_wide: Vec<u16> = src
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dst_wide: Vec<u16> = dst
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let ok = unsafe {
        MoveFileExW(
            src_wide.as_ptr(),
            dst_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn atomic_replace(src: &Path, dst: &Path) -> io::Result<()> {
    fs::rename(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    use crate::domain::workspace::StorageSchemaVersion;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SampleDoc {
        schema_version: StorageSchemaVersion,
        label: String,
        counter: u32,
    }

    fn sample(label: &str, counter: u32) -> SampleDoc {
        SampleDoc {
            schema_version: StorageSchemaVersion::V1,
            label: label.to_owned(),
            counter,
        }
    }

    #[test]
    fn atomic_write_creates_backup_and_leaves_no_tmp_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sample.json");
        let store = AtomicJsonStore::new(path.clone());

        store.write_atomic(&sample("v1", 1)).expect("write v1");
        assert!(!store.backup_path().unwrap().exists());

        store.write_atomic(&sample("v2", 2)).expect("write v2");
        let current: SampleDoc = store.read_valid().expect("read current");
        assert_eq!(current, sample("v2", 2));

        let bak_bytes = fs::read(store.backup_path().unwrap()).expect("read .bak");
        let bak_doc: SampleDoc = serde_json::from_slice(&bak_bytes).expect("parse .bak");
        assert_eq!(bak_doc, sample("v1", 1));

        let leftover_tmp: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp."))
            .collect();
        assert!(leftover_tmp.is_empty());
    }

    #[test]
    fn truncated_primary_recovers_automatically_from_valid_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.json");
        let store = AtomicJsonStore::new(path.clone());

        store.write_atomic(&sample("good-v1", 1)).expect("write v1");
        store.write_atomic(&sample("good-v2", 2)).expect("write v2");

        fs::write(&path, br#"{"schemaVersion": 1, "label": "trun"#).expect("truncate primary");

        let outcome = store
            .load_or_init(|| sample("default", 0))
            .expect("recover from backup");

        match outcome {
            LoadOutcome::RecoveredFromBackup {
                value, backup_name, ..
            } => {
                assert_eq!(value, sample("good-v1", 1));
                assert_eq!(backup_name, "workspace.json.bak");
            }
            other => panic!("expected RecoveredFromBackup, got {other:?}"),
        }

        let restored: SampleDoc = store.read_valid().expect("read restored primary");
        assert_eq!(restored, sample("good-v1", 1));

        let bak_still_valid: SampleDoc =
            serde_json::from_slice(&fs::read(store.backup_path().unwrap()).unwrap()).unwrap();
        assert_eq!(bak_still_valid, sample("good-v1", 1));
    }

    #[test]
    fn corrupt_primary_and_corrupt_backup_quarantine_and_reset_to_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("progress.json");
        let store = AtomicJsonStore::new(path.clone());

        fs::write(&path, b"not-json-at-all").expect("corrupt primary");
        fs::write(store.backup_path().unwrap(), b"{broken-bak").expect("corrupt bak");

        let outcome = store
            .load_or_init(|| sample("safe-default", 0))
            .expect("reset after corruption");

        let quarantine_name = match outcome {
            LoadOutcome::ResetAfterCorruption { value, event, .. } => {
                assert_eq!(value, sample("safe-default", 0));
                assert_eq!(event.file_name, "progress.json");
                assert!(event.backup_name.starts_with("progress.json.corrupt."));
                event.backup_name
            }
            other => panic!("expected ResetAfterCorruption, got {other:?}"),
        };

        assert_eq!(
            fs::read(dir.path().join(&quarantine_name)).expect("read quarantined file"),
            b"not-json-at-all"
        );
        assert!(!store.backup_path().unwrap().exists());
        assert_eq!(
            store.read_valid::<SampleDoc>().expect("read reset primary"),
            sample("safe-default", 0)
        );
    }

    #[test]
    fn unsupported_schema_version_triggers_backup_recovery_or_quarantine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        let store = AtomicJsonStore::new(path.clone());

        store
            .write_atomic(&sample("v1-saved", 7))
            .expect("write v1");
        store
            .write_atomic(&sample("v1-newer", 8))
            .expect("write v1 #2");

        fs::write(
            &path,
            br#"{"schemaVersion": 2, "label": "future", "counter": 99}"#,
        )
        .expect("write unsupported v2");

        let outcome = store
            .load_or_init(|| sample("default", 0))
            .expect("fallback to v1 backup");
        assert!(matches!(
            outcome,
            LoadOutcome::RecoveredFromBackup { value, .. } if value == sample("v1-saved", 7)
        ));
    }

    #[test]
    fn legacy_schema_migrator_upgrades_in_place_and_backs_up_original() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.json");
        let store = AtomicJsonStore::new(path.clone());

        let legacy_raw = br#"{"schemaVersion": 0, "legacyName": "migrated-course", "count": 42}"#;
        fs::write(&path, legacy_raw).expect("write legacy v0");

        let outcome = store
            .load_or_init_with_migrator(
                || sample("default", 0),
                |ver, raw| {
                    if ver != 0 {
                        return None;
                    }
                    let label = raw
                        .get("legacyName")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "missing legacyName".to_owned());
                    let counter = raw
                        .get("count")
                        .and_then(Value::as_u64)
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| "missing count".to_owned());
                    Some(match (label, counter) {
                        (Ok(label), Ok(counter)) => Ok(sample(label, counter)),
                        (Err(e), _) | (_, Err(e)) => Err(e),
                    })
                },
            )
            .expect("migrate v0 to v1");

        assert!(matches!(
            outcome,
            LoadOutcome::Migrated {
                value,
                from_version: 0,
                backup_name,
            } if value == sample("migrated-course", 42) && backup_name == "workspace.json.bak"
        ));

        let on_disk: SampleDoc = store.read_valid().expect("read migrated v1");
        assert_eq!(on_disk, sample("migrated-course", 42));
        assert_eq!(
            fs::read(store.backup_path().unwrap()).expect("read pre-migration backup"),
            legacy_raw
        );
    }

    #[test]
    fn cleanup_stale_tmp_files_sweeps_only_temp_artifacts() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("workspace.json"), b"{}").unwrap();
        fs::write(dir.path().join("workspace.json.bak"), b"{}").unwrap();
        fs::write(dir.path().join("workspace.json.tmp.1234.1"), b"partial").unwrap();
        fs::write(dir.path().join("progress.json.bak.tmp.1234.2"), b"partial").unwrap();

        let removed = cleanup_stale_tmp_files(dir.path()).expect("cleanup tmp");
        assert_eq!(
            removed,
            vec![
                "progress.json.bak.tmp.1234.2".to_owned(),
                "workspace.json.tmp.1234.1".to_owned(),
            ]
        );
        assert!(dir.path().join("workspace.json").exists());
        assert!(dir.path().join("workspace.json.bak").exists());
    }
}
