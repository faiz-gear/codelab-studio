use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::git_importer::git_import_error;
use crate::domain::error::AppError;
use crate::domain::workspace::StorageSchemaVersion;
use crate::platform::DirectoryFileLock;
use crate::storage::AtomicJsonFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCacheResult {
    pub cache_dir: PathBuf,
    pub source_key: String,
    pub cache_hit: bool,
    pub head_commit: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitCacheEntry {
    dir_name: String,
    source_key: String,
    url: String,
    git_ref: Option<String>,
    head_commit: Option<String>,
    size_bytes: u64,
    last_used_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitCacheIndex {
    schema_version: StorageSchemaVersion,
    #[serde(default)]
    entries: Vec<GitCacheEntry>,
}

impl Default for GitCacheIndex {
    fn default() -> Self {
        Self {
            schema_version: StorageSchemaVersion::V1,
            entries: Vec::new(),
        }
    }
}

pub(super) struct CommitStagedPackArgs<'a> {
    pub(super) cache_root: &'a Path,
    pub(super) staging_dir: &'a Path,
    pub(super) dir_name: &'a str,
    pub(super) source_key: String,
    pub(super) url: String,
    pub(super) git_ref: Option<String>,
    pub(super) head_commit: Option<String>,
    pub(super) size_bytes: u64,
    pub(super) max_cache_packs: usize,
    pub(super) max_cache_total_bytes: u64,
}

pub(super) fn try_cache_hit(
    cache_root: &Path,
    dir_name: &str,
    source_key: &str,
) -> Result<Option<GitCacheResult>, AppError> {
    let lock_path = cache_root.join(".lock");
    let _lock = DirectoryFileLock::lock_blocking(&lock_path)
        .map_err(|err| git_import_error(format!("failed to lock git cache: {err}"), true))?;

    let target_dir = cache_root.join(dir_name);
    if !target_dir.is_dir() {
        return Ok(None);
    }

    let index_file = AtomicJsonFile::<GitCacheIndex>::new(cache_root.join("index.json"));
    let (found, _) = index_file
        .load_or_init_and_update(GitCacheIndex::default, |index| {
            let next_ts = next_monotonic_timestamp_ms(&index.entries);
            if let Some(entry) = index
                .entries
                .iter_mut()
                .find(|entry| entry.dir_name == dir_name)
            {
                entry.last_used_at_ms = next_ts;
                Ok(Some((entry.head_commit.clone(), entry.size_bytes)))
            } else {
                Ok(None)
            }
        })
        .map_err(AppError::from)?;

    Ok(found.map(|(head_commit, size_bytes)| GitCacheResult {
        cache_dir: target_dir,
        source_key: source_key.to_owned(),
        cache_hit: true,
        head_commit,
        size_bytes,
    }))
}

pub(super) fn commit_staged_pack(args: CommitStagedPackArgs<'_>) -> Result<(), AppError> {
    let lock_path = args.cache_root.join(".lock");
    let _lock = DirectoryFileLock::lock_blocking(&lock_path)
        .map_err(|err| git_import_error(format!("failed to lock git cache: {err}"), true))?;

    let target_dir = args.cache_root.join(args.dir_name);
    let displaced = if target_dir.exists() {
        let evict_path = args.cache_root.join(format!(
            ".evict-{}-{}",
            args.dir_name,
            Uuid::new_v4().simple()
        ));
        fs::rename(&target_dir, &evict_path).map_err(|err| {
            git_import_error(
                format!("failed to rotate existing git cache entry: {err}"),
                true,
            )
        })?;
        Some(evict_path)
    } else {
        None
    };

    if let Err(err) = fs::rename(args.staging_dir, &target_dir) {
        if let Some(previous) = displaced {
            let _ = fs::rename(&previous, &target_dir);
        }
        return Err(git_import_error(
            format!("failed to finalize git cache directory: {err}"),
            true,
        ));
    }

    if let Some(previous) = displaced {
        let _ = remove_dir_all_force(&previous);
    }

    let index_file = AtomicJsonFile::<GitCacheIndex>::new(args.cache_root.join("index.json"));
    let (evicted_dirs, _) = index_file
        .load_or_init_and_update(GitCacheIndex::default, |index| {
            let next_ts = next_monotonic_timestamp_ms(&index.entries);
            index
                .entries
                .retain(|entry| entry.dir_name != args.dir_name);
            index.entries.push(GitCacheEntry {
                dir_name: args.dir_name.to_owned(),
                source_key: args.source_key,
                url: args.url,
                git_ref: args.git_ref,
                head_commit: args.head_commit,
                size_bytes: args.size_bytes,
                last_used_at_ms: next_ts,
            });

            let mut evicted = Vec::new();
            while index.entries.len() > args.max_cache_packs
                || total_cache_bytes(&index.entries) > args.max_cache_total_bytes
            {
                let oldest_idx = index
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.dir_name != args.dir_name)
                    .min_by_key(|(_, entry)| entry.last_used_at_ms)
                    .map(|(idx, _)| idx);

                let Some(idx) = oldest_idx else {
                    break;
                };
                evicted.push(index.entries.remove(idx).dir_name);
            }

            Ok(evicted)
        })
        .map_err(AppError::from)?;

    for dir_name in evicted_dirs {
        let _ = remove_dir_all_force(&args.cache_root.join(dir_name));
    }

    Ok(())
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn next_monotonic_timestamp_ms(entries: &[GitCacheEntry]) -> u64 {
    let max_existing = entries
        .iter()
        .map(|entry| entry.last_used_at_ms)
        .max()
        .unwrap_or(0);
    current_unix_ms().max(max_existing.saturating_add(1))
}

fn total_cache_bytes(entries: &[GitCacheEntry]) -> u64 {
    entries
        .iter()
        .fold(0u64, |total, entry| total.saturating_add(entry.size_bytes))
}

pub(super) fn remove_dir_all_force(path: &Path) -> std::io::Result<()> {
    if !path.exists() && fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    if fs::remove_dir_all(path).is_ok() {
        return Ok(());
    }
    make_tree_writable(path);
    fs::remove_dir_all(path)
}

fn make_tree_writable(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        let required = if metadata.is_dir() { 0o700 } else { 0o200 };
        if mode & required != required {
            permissions.set_mode(mode | required);
            let _ = fs::set_permissions(path, permissions);
        }
    }

    #[cfg(not(unix))]
    {
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
    }

    if metadata.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                make_tree_writable(&entry.path());
            }
        }
    }
}
