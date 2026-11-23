use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CODELAB_DIR_NAME: &str = ".codelab";
pub const CODELAB_HOME_ENV: &str = "CODELAB_HOME";

static PROCESS_STARTED_AT_MS: OnceLock<u64> = OnceLock::new();
static LOCK_OP_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
static NEXT_LOCK_NONCE: AtomicU64 = AtomicU64::new(1);

const MAX_LOCK_RECLAIM_ATTEMPTS: usize = 4;
const CORRUPT_LOCK_GRACE_PERIOD: Duration = Duration::from_millis(250);

pub fn current_process_started_at_ms() -> u64 {
    *PROCESS_STARTED_AT_MS.get_or_init(unix_timestamp_ms)
}

fn lock_op_guard() -> &'static Mutex<()> {
    LOCK_OP_GUARD.get_or_init(|| Mutex::new(()))
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
    #[error("workspace '{workspace_root}' is already locked by live process {pid} (since {acquired_at})")]
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
    record: WorkspaceLockRecord,
    reclaimed_stale: bool,
}

impl WorkspaceLock {
    pub fn acquire(paths: &WorkspacePaths) -> Result<Self, WorkspaceLockError> {
        let meta_dir = paths.meta_dir();
        fs::create_dir_all(&meta_dir)?;
        let lock_path = paths.lock_file();

        let _guard = lock_op_guard()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let record = WorkspaceLockRecord {
            pid: std::process::id(),
            process_started_at_ms: current_process_started_at_ms(),
            nonce: NEXT_LOCK_NONCE.fetch_add(1, Ordering::Relaxed),
            acquired_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            workspace_root: paths.root_string(),
        };

        let mut reclaimed_stale = false;

        for _ in 0..MAX_LOCK_RECLAIM_ATTEMPTS {
            match try_create_lock_file(&lock_path, &record) {
                Ok(()) => {
                    return Ok(Self {
                        lock_path,
                        record,
                        reclaimed_stale,
                    });
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    match inspect_existing_lock(&lock_path)? {
                        ExistingLockStatus::Missing => continue,
                        ExistingLockStatus::Active(existing) => {
                            return Err(WorkspaceLockError::AlreadyLocked {
                                pid: existing.pid,
                                acquired_at: existing.acquired_at,
                                workspace_root: existing.workspace_root,
                            });
                        }
                        ExistingLockStatus::Stale => {
                            if let Err(remove_err) = fs::remove_file(&lock_path) {
                                if remove_err.kind() != io::ErrorKind::NotFound {
                                    return Err(WorkspaceLockError::Io(remove_err));
                                }
                            }
                            reclaimed_stale = true;
                        }
                        ExistingLockStatus::UnreadableFresh => {
                            return Err(WorkspaceLockError::AlreadyLocked {
                                pid: 0,
                                acquired_at: "unknown".to_owned(),
                                workspace_root: paths.root_string(),
                            });
                        }
                    }
                }
                Err(err) => return Err(WorkspaceLockError::Io(err)),
            }
        }

        Err(WorkspaceLockError::AlreadyLocked {
            pid: 0,
            acquired_at: "contended".to_owned(),
            workspace_root: paths.root_string(),
        })
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
        let _guard = lock_op_guard()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Ok(raw) = fs::read(&self.lock_path) else {
            return;
        };
        let Ok(on_disk) = serde_json::from_slice::<WorkspaceLockRecord>(&raw) else {
            return;
        };
        if on_disk.pid == self.record.pid
            && on_disk.process_started_at_ms == self.record.process_started_at_ms
            && on_disk.nonce == self.record.nonce
        {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

enum ExistingLockStatus {
    Missing,
    Active(WorkspaceLockRecord),
    Stale,
    UnreadableFresh,
}

fn try_create_lock_file(lock_path: &Path, record: &WorkspaceLockRecord) -> io::Result<()> {
    let mut payload = serde_json::to_vec_pretty(record)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    payload.push(b'\n');

    let tmp_path = lock_path.with_file_name(format!(".lock.tmp.{}.{}", record.pid, record.nonce));

    {
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        tmp_file.write_all(&payload)?;
        tmp_file.sync_all()?;
    }

    let link_result = fs::hard_link(&tmp_path, lock_path);
    let _ = fs::remove_file(&tmp_path);

    match link_result {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(err),
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(lock_path)?;
            file.write_all(&payload)?;
            file.sync_all()?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn inspect_existing_lock(lock_path: &Path) -> io::Result<ExistingLockStatus> {
    let raw = match fs::read(lock_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(ExistingLockStatus::Missing);
        }
        Err(err) => return Err(err),
    };

    match serde_json::from_slice::<WorkspaceLockRecord>(&raw) {
        Ok(existing) => {
            if is_lock_record_live(&existing) {
                Ok(ExistingLockStatus::Active(existing))
            } else {
                Ok(ExistingLockStatus::Stale)
            }
        }
        Err(_) => {
            let is_fresh = fs::metadata(lock_path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .is_some_and(|age| age < CORRUPT_LOCK_GRACE_PERIOD && raw.is_empty());

            if is_fresh {
                Ok(ExistingLockStatus::UnreadableFresh)
            } else {
                Ok(ExistingLockStatus::Stale)
            }
        }
    }
}

fn is_lock_record_live(record: &WorkspaceLockRecord) -> bool {
    if record.pid == 0 {
        return false;
    }

    let my_pid = std::process::id();
    if record.pid == my_pid {
        return record.process_started_at_ms == current_process_started_at_ms();
    }

    is_other_pid_alive(record.pid)
}

#[cfg(unix)]
fn is_other_pid_alive(pid: u32) -> bool {
    let Ok(pid_t) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid_t <= 0 {
        return false;
    }
    let rc = unsafe { libc::kill(pid_t, 0) };
    if rc == 0 {
        return true;
    }
    let errno = io::Error::last_os_error().raw_os_error();
    errno == Some(libc::EPERM)
}

#[cfg(windows)]
fn is_other_pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return io::Error::last_os_error().raw_os_error() == Some(ERROR_ACCESS_DENIED as i32);
    }

    let mut exit_code: u32 = 0;
    let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    unsafe {
        CloseHandle(handle);
    }
    ok != 0 && exit_code == STILL_ACTIVE as u32
}

#[cfg(not(any(unix, windows)))]
fn is_other_pid_alive(_pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn dead_child_pid() -> u32 {
        #[cfg(unix)]
        let mut child = Command::new("/usr/bin/true")
            .spawn()
            .or_else(|_| Command::new("true").spawn())
            .expect("spawn short-lived child");

        #[cfg(windows)]
        let mut child = Command::new("cmd")
            .args(["/C", "exit", "0"])
            .spawn()
            .expect("spawn short-lived child");

        let pid = child.id();
        let _ = child.wait();
        pid
    }

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
    fn workspace_lock_reclaims_dead_pid_stale_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");

        let stale = WorkspaceLockRecord {
            pid: dead_child_pid(),
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

        let lock = WorkspaceLock::acquire(&paths).expect("reclaim stale dead-PID lock");
        assert!(lock.reclaimed_stale());
        assert_eq!(lock.record().pid, std::process::id());
    }

    #[test]
    fn workspace_lock_reclaims_pid_reused_lock_with_different_start_timestamp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");

        let stale = WorkspaceLockRecord {
            pid: std::process::id(),
            process_started_at_ms: current_process_started_at_ms().saturating_sub(60_000),
            nonce: 1,
            acquired_at: "2026-01-01T00:00:00Z".to_owned(),
            workspace_root: paths.root_string(),
        };
        fs::write(
            paths.lock_file(),
            serde_json::to_vec_pretty(&stale).expect("serialize stale lock"),
        )
        .expect("write stale lock");

        let lock = WorkspaceLock::acquire(&paths).expect("reclaim PID-reused stale lock");
        assert!(lock.reclaimed_stale());
        assert_eq!(
            lock.record().process_started_at_ms,
            current_process_started_at_ms()
        );
    }

    #[test]
    fn workspace_lock_reclaims_corrupt_lock_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");
        fs::create_dir_all(paths.meta_dir()).expect("create .codelab");
        fs::write(paths.lock_file(), b"{truncated-lock").expect("write corrupt lock");

        let lock = WorkspaceLock::acquire(&paths).expect("reclaim corrupt lock");
        assert!(lock.reclaimed_stale());
    }

    #[test]
    fn workspace_lock_drop_does_not_remove_replaced_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("workspace paths");

        let lock = WorkspaceLock::acquire(&paths).expect("acquire lock");
        let other = WorkspaceLockRecord {
            pid: std::process::id(),
            process_started_at_ms: current_process_started_at_ms(),
            nonce: lock.record().nonce + 1000,
            acquired_at: "2026-11-23T00:00:00Z".to_owned(),
            workspace_root: paths.root_string(),
        };
        fs::write(
            paths.lock_file(),
            serde_json::to_vec_pretty(&other).expect("serialize other lock"),
        )
        .expect("replace lock");

        drop(lock);
        assert!(paths.lock_file().exists());
    }
}
