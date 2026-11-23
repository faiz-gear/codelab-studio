use std::collections::HashSet;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CODELAB_DIR_NAME: &str = ".codelab";
pub const CODELAB_HOME_ENV: &str = "CODELAB_HOME";

static PROCESS_STARTED_AT_MS: OnceLock<u64> = OnceLock::new();
static HELD_WORKSPACE_LOCKS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
static NEXT_LOCK_NONCE: AtomicU64 = AtomicU64::new(1);

#[cfg(unix)]
const MAX_LOCK_INODE_RETRIES: usize = 4;

pub fn current_process_started_at_ms() -> u64 {
    *PROCESS_STARTED_AT_MS.get_or_init(unix_timestamp_ms)
}

fn held_workspace_locks() -> &'static Mutex<HashSet<PathBuf>> {
    HELD_WORKSPACE_LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodelabHomePaths {
    root: PathBuf,
}

impl CodelabHomePaths {
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn resolve_default() -> io::Result<Self> {
        if let Ok(override_dir) = std::env::var(CODELAB_HOME_ENV) {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return Ok(Self {
                    root: PathBuf::from(trimmed),
                });
            }
        }

        let home = dirs::home_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not resolve user home directory for ~/.codelab",
            )
        })?;
        Ok(Self {
            root: home.join(CODELAB_DIR_NAME),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn lock_file(&self) -> PathBuf {
        self.root.join(".lock")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn trusted_packs_path(&self) -> PathBuf {
        self.root.join("trusted_packs.json")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn runtimes_manifest_path(&self) -> PathBuf {
        self.runtimes_dir().join("manifest.json")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
}

#[derive(Debug)]
pub struct DirectoryFileLock {
    _file: File,
}

impl DirectoryFileLock {
    pub fn lock_blocking(lock_path: &Path) -> io::Result<Self> {
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        file.lock()?;
        Ok(Self { _file: file })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    canonical_root: PathBuf,
}

impl WorkspacePaths {
    pub fn from_root(root: &Path, create_if_missing: bool) -> io::Result<Self> {
        if create_if_missing {
            fs::create_dir_all(root)?;
        }
        let canonical_root = dunce::canonicalize(root)?;
        if !canonical_root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "workspace path '{}' is not a directory",
                    canonical_root.display()
                ),
            ));
        }
        Ok(Self { canonical_root })
    }

    pub fn root(&self) -> &Path {
        &self.canonical_root
    }

    pub fn root_string(&self) -> String {
        self.canonical_root.to_string_lossy().into_owned()
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.canonical_root.join(CODELAB_DIR_NAME)
    }

    pub fn workspace_json(&self) -> PathBuf {
        self.meta_dir().join("workspace.json")
    }

    pub fn progress_json(&self) -> PathBuf {
        self.meta_dir().join("progress.json")
    }

    pub fn chat_json(&self) -> PathBuf {
        self.meta_dir().join("chat.json")
    }

    pub fn metrics_json(&self) -> PathBuf {
        self.meta_dir().join("metrics.json")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.meta_dir().join(".lock")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLockRecord {
    pub pid: u32,
    pub process_started_at_ms: u64,
    pub nonce: u64,
    pub acquired_at: String,
    pub workspace_root: String,
}

#[derive(Debug, Error)]
pub enum WorkspaceLockError {
    #[error(
        "workspace '{workspace_root}' is already locked by live process {pid} (since {acquired_at})"
    )]
    AlreadyLocked {
        pid: u32,
        acquired_at: String,
        workspace_root: String,
    },
    #[error("workspace lock I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug)]
pub struct WorkspaceLock {
    lock_path: PathBuf,
    file: Option<File>,
    record: WorkspaceLockRecord,
    reclaimed_stale: bool,
}

impl WorkspaceLock {
    pub fn acquire(paths: &WorkspacePaths) -> Result<Self, WorkspaceLockError> {
        let meta_dir = paths.meta_dir();
        fs::create_dir_all(&meta_dir)?;
        let lock_path = paths.lock_file();

        {
            let mut held = held_workspace_locks()
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if !held.insert(lock_path.clone()) {
                return Err(read_holder_error(&lock_path, paths.root_string()));
            }
        }

        match acquire_kernel_lock(&lock_path, paths.root_string()) {
            Ok((file, record, reclaimed_stale)) => Ok(Self {
                lock_path,
                file: Some(file),
                record,
                reclaimed_stale,
            }),
            Err(err) => {
                let mut held = held_workspace_locks()
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                held.remove(&lock_path);
                Err(err)
            }
        }
    }

    pub fn record(&self) -> &WorkspaceLockRecord {
        &self.record
    }

    pub fn reclaimed_stale(&self) -> bool {
        self.reclaimed_stale
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(ref file) = self.file {
                if same_open_inode(file, &self.lock_path) {
                    let _ = fs::remove_file(&self.lock_path);
                }
            }
            let _ = self.file.take();
        }

        #[cfg(not(unix))]
        {
            let _ = self.file.take();
            let _ = fs::remove_file(&self.lock_path);
        }

        let mut held = held_workspace_locks()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        held.remove(&self.lock_path);
    }
}

fn acquire_kernel_lock(
    lock_path: &Path,
    workspace_root: String,
) -> Result<(File, WorkspaceLockRecord, bool), WorkspaceLockError> {
    let mut file = open_and_lock_file(lock_path, &workspace_root)?;

    let mut prior_bytes = Vec::new();
    let _ = file.seek(SeekFrom::Start(0));
    let _ = file.read_to_end(&mut prior_bytes);
    let reclaimed_stale = !prior_bytes.is_empty();

    let record = WorkspaceLockRecord {
        pid: std::process::id(),
        process_started_at_ms: current_process_started_at_ms(),
        nonce: NEXT_LOCK_NONCE.fetch_add(1, Ordering::Relaxed),
        acquired_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        workspace_root,
    };

    let mut payload = serde_json::to_vec_pretty(&record)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    payload.push(b'\n');

    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&payload)?;
    file.sync_all()?;

    Ok((file, record, reclaimed_stale))
}

#[cfg(unix)]
fn open_and_lock_file(lock_path: &Path, workspace_root: &str) -> Result<File, WorkspaceLockError> {
    for _ in 0..MAX_LOCK_INODE_RETRIES {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;

        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(read_holder_error(lock_path, workspace_root.to_owned()));
            }
            Err(TryLockError::Error(err)) => return Err(WorkspaceLockError::Io(err)),
        }

        if same_open_inode(&file, lock_path) {
            return Ok(file);
        }
    }

    Err(read_holder_error(lock_path, workspace_root.to_owned()))
}

#[cfg(not(unix))]
fn open_and_lock_file(lock_path: &Path, workspace_root: &str) -> Result<File, WorkspaceLockError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;

    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => {
            Err(read_holder_error(lock_path, workspace_root.to_owned()))
        }
        Err(TryLockError::Error(err)) => Err(WorkspaceLockError::Io(err)),
    }
}

fn read_holder_error(lock_path: &Path, fallback_root: String) -> WorkspaceLockError {
    if let Ok(raw) = fs::read(lock_path) {
        if let Ok(existing) = serde_json::from_slice::<WorkspaceLockRecord>(&raw) {
            return WorkspaceLockError::AlreadyLocked {
                pid: existing.pid,
                acquired_at: existing.acquired_at,
                workspace_root: existing.workspace_root,
            };
        }
    }
    WorkspaceLockError::AlreadyLocked {
        pid: std::process::id(),
        acquired_at: "active".to_owned(),
        workspace_root: fallback_root,
    }
}

#[cfg(unix)]
fn same_open_inode(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(fd_meta) = file.metadata() else {
        return false;
    };
    let Ok(path_meta) = fs::metadata(path) else {
        return false;
    };
    fd_meta.dev() == path_meta.dev() && fd_meta.ino() == path_meta.ino()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codelab_home_paths_exposes_expected_layout() {
        let home = CodelabHomePaths::from_root(PathBuf::from("/tmp/codelab-test-home"));
        assert_eq!(
            home.config_path(),
            PathBuf::from("/tmp/codelab-test-home/config.json")
        );
        assert_eq!(
            home.trusted_packs_path(),
            PathBuf::from("/tmp/codelab-test-home/trusted_packs.json")
        );
        assert_eq!(
            home.runtimes_manifest_path(),
            PathBuf::from("/tmp/codelab-test-home/runtimes/manifest.json")
        );
        assert_eq!(
            home.cache_dir(),
            PathBuf::from("/tmp/codelab-test-home/cache")
        );
    }

    #[test]
    fn workspace_lock_blocks_concurrent_acquisition_and_releases_on_drop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");

        let lock = WorkspaceLock::acquire(&paths).expect("first acquisition succeeds");
        assert!(!lock.reclaimed_stale());
        assert!(paths.lock_file().exists());

        let second = WorkspaceLock::acquire(&paths);
        assert!(matches!(
            second,
            Err(WorkspaceLockError::AlreadyLocked { pid, .. }) if pid == std::process::id()
        ));

        drop(lock);
        assert!(!paths.lock_file().exists());

        let reacquired = WorkspaceLock::acquire(&paths).expect("reacquire after drop succeeds");
        assert!(!reacquired.reclaimed_stale());
    }

    #[test]
    fn workspace_lock_reclaims_unlocked_stale_lock_even_if_pid_was_reused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");

        let stale = WorkspaceLockRecord {
            pid: 1,
            process_started_at_ms: 1_700_000_000_000,
            nonce: 99,
            acquired_at: "2026-01-01T00:00:00Z".to_owned(),
            workspace_root: paths.root_string(),
        };
        fs::write(
            paths.lock_file(),
            serde_json::to_vec_pretty(&stale).expect("serialize stale lock"),
        )
        .expect("write stale lock");

        let lock = WorkspaceLock::acquire(&paths).expect("reclaim unlocked stale lock");
        assert!(lock.reclaimed_stale());
        assert_eq!(lock.record().pid, std::process::id());
    }

    #[test]
    fn workspace_lock_reclaims_corrupt_unlocked_lock_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");
        fs::write(paths.lock_file(), b"{truncated-lock").expect("write corrupt lock");

        let lock = WorkspaceLock::acquire(&paths).expect("reclaim corrupt lock");
        assert!(lock.reclaimed_stale());
    }

    #[test]
    fn workspace_lock_rejects_external_kernel_held_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");

        let external_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(paths.lock_file())
            .expect("open external lock");
        external_file.try_lock().expect("hold external kernel lock");

        let attempt = WorkspaceLock::acquire(&paths);
        assert!(matches!(
            attempt,
            Err(WorkspaceLockError::AlreadyLocked { .. })
        ));

        drop(external_file);

        let acquired = WorkspaceLock::acquire(&paths).expect("acquire after external unlock");
        assert_eq!(acquired.record().pid, std::process::id());
    }
}
