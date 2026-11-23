use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::process::Command;
use uuid::Uuid;

use super::git_cache::{commit_staged_pack, try_cache_hit, CommitStagedPackArgs};
use crate::course::pack_loader::bytes_to_hex;
use crate::domain::error::{AppError, ErrorCode};

pub use super::git_cache::GitCacheResult;
pub use super::git_url::{validate_git_ref, validate_git_url};

pub const DEFAULT_GIT_CLONE_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_MAX_PACK_BYTES: u64 = 100 * 1024 * 1024;
pub const DEFAULT_MAX_GIT_METADATA_BYTES: u64 = 100 * 1024 * 1024;
pub const DEFAULT_MAX_CACHE_PACKS: usize = 20;
pub const DEFAULT_MAX_CACHE_TOTAL_BYTES: u64 = 500 * 1024 * 1024;

const REV_PARSE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitImportConfig {
    pub clone_timeout: Duration,
    pub max_pack_bytes: u64,
    pub max_git_metadata_bytes: u64,
    pub max_cache_packs: usize,
    pub max_cache_total_bytes: u64,
}

impl Default for GitImportConfig {
    fn default() -> Self {
        Self {
            clone_timeout: DEFAULT_GIT_CLONE_TIMEOUT,
            max_pack_bytes: DEFAULT_MAX_PACK_BYTES,
            max_git_metadata_bytes: DEFAULT_MAX_GIT_METADATA_BYTES,
            max_cache_packs: DEFAULT_MAX_CACHE_PACKS,
            max_cache_total_bytes: DEFAULT_MAX_CACHE_TOTAL_BYTES,
        }
    }
}

struct StagingCleanupGuard {
    path: Option<PathBuf>,
}

impl StagingCleanupGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for StagingCleanupGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = super::git_cache::remove_dir_all_force(&path);
        }
    }
}

pub async fn fetch_git_pack(
    cache_root: &Path,
    url: &str,
    git_ref: Option<&str>,
    refresh: bool,
    config: &GitImportConfig,
) -> Result<GitCacheResult, AppError> {
    let validated_url = validate_git_url(url)?;
    let validated_ref = match git_ref {
        Some(raw) => Some(validate_git_ref(raw)?),
        None => None,
    };

    let source_key = git_source_key(&validated_url, validated_ref.as_deref());
    let dir_name = sha256_hex(source_key.as_bytes());
    let cache_root_owned = cache_root.to_path_buf();
    let target_dir = cache_root_owned.join(&dir_name);

    {
        let root_for_create = cache_root_owned.clone();
        tokio::task::spawn_blocking(move || fs::create_dir_all(&root_for_create))
            .await
            .map_err(join_error_to_app_error)?
            .map_err(|err| {
                git_import_error(
                    format!("failed to create git cache directory: {err}"),
                    false,
                )
            })?;
    }

    if !refresh {
        let cache_root_check = cache_root_owned.clone();
        let dir_name_check = dir_name.clone();
        let source_key_check = source_key.clone();
        let hit = tokio::task::spawn_blocking(move || {
            try_cache_hit(&cache_root_check, &dir_name_check, &source_key_check)
        })
        .await
        .map_err(join_error_to_app_error)??;

        if let Some(result) = hit {
            return Ok(result);
        }
    }

    let staging_dir =
        cache_root_owned.join(format!(".staging-{dir_name}-{}", Uuid::new_v4().simple()));
    let mut staging_guard = StagingCleanupGuard::new(staging_dir.clone());

    clone_repository(
        &validated_url,
        validated_ref.as_deref(),
        &staging_dir,
        config,
    )
    .await?;

    let head_commit = read_head_commit(&staging_dir).await;

    let staging_for_inspect = staging_dir.clone();
    let max_pack_bytes = config.max_pack_bytes;
    let max_git_metadata_bytes = config.max_git_metadata_bytes;
    let url_for_size_err = validated_url.clone();
    let size_bytes = tokio::task::spawn_blocking(move || {
        strip_dot_git_and_measure(
            &staging_for_inspect,
            max_pack_bytes,
            max_git_metadata_bytes,
            &url_for_size_err,
        )
    })
    .await
    .map_err(join_error_to_app_error)??;

    let cache_root_commit = cache_root_owned.clone();
    let dir_name_commit = dir_name.clone();
    let source_key_commit = source_key.clone();
    let url_commit = validated_url.clone();
    let ref_commit = validated_ref.clone();
    let head_commit_copy = head_commit.clone();
    let staging_commit = staging_dir.clone();
    let max_cache_packs = config.max_cache_packs;
    let max_cache_total_bytes = config.max_cache_total_bytes;

    tokio::task::spawn_blocking(move || {
        commit_staged_pack(CommitStagedPackArgs {
            cache_root: &cache_root_commit,
            staging_dir: &staging_commit,
            dir_name: &dir_name_commit,
            source_key: source_key_commit,
            url: url_commit,
            git_ref: ref_commit,
            head_commit: head_commit_copy,
            size_bytes,
            max_cache_packs,
            max_cache_total_bytes,
        })
    })
    .await
    .map_err(join_error_to_app_error)??;

    staging_guard.disarm();

    Ok(GitCacheResult {
        cache_dir: target_dir,
        source_key,
        cache_hit: false,
        head_commit,
        size_bytes,
    })
}

async fn read_head_commit(repo_dir: &Path) -> Option<String> {
    let child = Command::new("git")
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .arg("-C")
        .arg(repo_dir)
        .args(["rev-parse", "HEAD"])
        .spawn()
        .ok()?;

    let output = tokio::time::timeout(REV_PARSE_TIMEOUT, child.wait_with_output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let trimmed = sha.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

async fn clone_repository(
    url: &str,
    git_ref: Option<&str>,
    staging_dir: &Path,
    config: &GitImportConfig,
) -> Result<(), AppError> {
    let mut command = Command::new("git");
    command
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "true")
        .args([
            "clone",
            "--depth",
            "1",
            "--single-branch",
            "-c",
            "protocol.file.allow=never",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.symlinks=false",
        ]);
    if let Some(branch) = git_ref {
        command.args(["--branch", branch]);
    }
    command.arg("--").arg(url).arg(staging_dir);

    let mut child = command
        .spawn()
        .map_err(|err| git_import_error(format!("failed to launch git: {err}"), false))?;
    let deadline = tokio::time::Instant::now() + config.clone_timeout;

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| git_import_error(format!("git clone process failed: {err}"), true))?
        {
            if status.success() {
                return Ok(());
            }
            return Err(git_import_error(
                format!("git clone failed ({status})"),
                true,
            ));
        }

        if tokio::time::Instant::now() >= deadline {
            stop_child(&mut child).await;
            return Err(clone_timeout_error(config.clone_timeout));
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
        if !staging_dir.exists() {
            continue;
        }

        let staging_for_check = staging_dir.to_path_buf();
        let url_for_check = url.to_owned();
        let max_pack_bytes = config.max_pack_bytes;
        let max_git_metadata_bytes = config.max_git_metadata_bytes;
        let size_check = tokio::task::spawn_blocking(move || {
            let limits = StagingSizeLimits {
                pack_bytes: max_pack_bytes,
                git_metadata_bytes: max_git_metadata_bytes,
                url_label: &url_for_check,
            };
            let mut usage = StagingSizeUsage::default();
            inspect_staging_tree(
                &staging_for_check,
                &staging_for_check,
                false,
                &limits,
                &mut usage,
            )
        })
        .await
        .map_err(join_error_to_app_error)?;

        if let Err(err) = size_check {
            stop_child(&mut child).await;
            return Err(err);
        }
    }
}

async fn stop_child(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn clone_timeout_error(timeout: Duration) -> AppError {
    git_import_error(
        format!("git clone timed out after {}ms", timeout.as_millis()),
        true,
    )
}

struct StagingSizeLimits<'a> {
    pack_bytes: u64,
    git_metadata_bytes: u64,
    url_label: &'a str,
}

#[derive(Default)]
struct StagingSizeUsage {
    pack_bytes: u64,
    git_metadata_bytes: u64,
}

fn inspect_staging_tree(
    dir: &Path,
    staging_root: &Path,
    in_git_metadata: bool,
    limits: &StagingSizeLimits<'_>,
    usage: &mut StagingSizeUsage,
) -> Result<(), AppError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(git_import_error(
                format!(
                    "failed to inspect cloned repository '{}': {err}",
                    dir.display()
                ),
                false,
            ));
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(git_import_error(
                    format!("failed to read cloned repository entry: {err}"),
                    false,
                ));
            }
        };
        let path = entry.path();
        let is_git_metadata = in_git_metadata
            || (dir == staging_root && entry.file_name() == std::ffi::OsStr::new(".git"));
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(git_import_error(
                    format!("failed to stat cloned entry '{}': {err}", path.display()),
                    false,
                ));
            }
        };

        if metadata.file_type().is_symlink() {
            return Err(
                AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
                    .with_param("field", "gitRepo")
                    .with_detail(format!(
                        "symlink rejected inside cloned git course pack: {}",
                        path.display()
                    )),
            );
        }

        if metadata.is_dir() {
            inspect_staging_tree(&path, staging_root, is_git_metadata, limits, usage)?;
        } else {
            let (total_bytes, limit, resource_name) = if is_git_metadata {
                usage.git_metadata_bytes = usage.git_metadata_bytes.saturating_add(metadata.len());
                (
                    usage.git_metadata_bytes,
                    limits.git_metadata_bytes,
                    "Git metadata",
                )
            } else {
                usage.pack_bytes = usage.pack_bytes.saturating_add(metadata.len());
                (usage.pack_bytes, limits.pack_bytes, "unpacked course pack")
            };
            if total_bytes > limit {
                return Err(cloned_size_limit_error(
                    limits.url_label,
                    resource_name,
                    total_bytes,
                    limit,
                ));
            }
        }
    }

    Ok(())
}

fn cloned_size_limit_error(
    url_label: &str,
    resource_name: &str,
    actual_bytes: u64,
    max_bytes: u64,
) -> AppError {
    AppError::new(
        ErrorCode::FileTooLargeOrBinary,
        "error.fileTooLargeOrBinary",
    )
    .with_param("path", url_label)
    .with_detail(format!(
        "cloned {resource_name} size ({actual_bytes} bytes) exceeds limit of {max_bytes} bytes"
    ))
}

fn strip_dot_git_and_measure(
    staging_dir: &Path,
    max_pack_bytes: u64,
    max_git_metadata_bytes: u64,
    url_label: &str,
) -> Result<u64, AppError> {
    let limits = StagingSizeLimits {
        pack_bytes: max_pack_bytes,
        git_metadata_bytes: max_git_metadata_bytes,
        url_label,
    };
    let mut usage = StagingSizeUsage::default();
    inspect_staging_tree(staging_dir, staging_dir, false, &limits, &mut usage)?;

    let dot_git = staging_dir.join(".git");
    if let Ok(meta) = fs::symlink_metadata(&dot_git) {
        if meta.is_dir() {
            super::git_cache::remove_dir_all_force(&dot_git).map_err(|err| {
                git_import_error(format!("failed to strip .git directory: {err}"), false)
            })?;
        } else {
            fs::remove_file(&dot_git).map_err(|err| {
                git_import_error(format!("failed to strip .git file: {err}"), false)
            })?;
        }
    }

    let mut total_bytes = 0u64;
    inspect_cloned_tree(staging_dir, max_pack_bytes, url_label, &mut total_bytes)?;
    Ok(total_bytes)
}

fn inspect_cloned_tree(
    dir: &Path,
    max_pack_bytes: u64,
    url_label: &str,
    total_bytes: &mut u64,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir).map_err(|err| {
        git_import_error(
            format!(
                "failed to inspect cloned repository '{}': {err}",
                dir.display()
            ),
            false,
        )
    })? {
        let entry = entry.map_err(|err| {
            git_import_error(format!("failed to read cloned entry: {err}"), false)
        })?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|err| {
            git_import_error(
                format!("failed to stat cloned entry '{}': {err}", path.display()),
                false,
            )
        })?;

        if meta.file_type().is_symlink() {
            return Err(
                AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
                    .with_param("field", "gitRepo")
                    .with_detail(format!(
                        "symlink rejected inside cloned git course pack: {}",
                        path.display()
                    )),
            );
        }

        if meta.is_dir() {
            inspect_cloned_tree(&path, max_pack_bytes, url_label, total_bytes)?;
        } else if meta.is_file() {
            *total_bytes = total_bytes.saturating_add(meta.len());
            if *total_bytes > max_pack_bytes {
                return Err(cloned_size_limit_error(
                    url_label,
                    "unpacked course pack",
                    *total_bytes,
                    max_pack_bytes,
                ));
            }
        }
    }

    Ok(())
}

fn git_source_key(url: &str, git_ref: Option<&str>) -> String {
    format!("git:{}:{url}{}", url.len(), git_ref.unwrap_or_default())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    bytes_to_hex(&hasher.finalize())
}

pub(super) fn git_import_error(detail: String, retryable: bool) -> AppError {
    AppError::new(ErrorCode::SchemaValidationFailed, "error.gitImportFailed")
        .with_param("detail", detail.as_str())
        .with_detail(detail)
        .with_retryable(retryable)
}

fn join_error_to_app_error(err: tokio::task::JoinError) -> AppError {
    AppError::new(ErrorCode::Cancelled, "error.unknown")
        .with_param("detail", err.to_string())
        .with_detail(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_keys_distinguish_url_and_ref_boundaries() {
        let url = "https://example.com/course";
        assert_ne!(
            git_source_key(url, Some("lesson")),
            git_source_key("https://example.com/course@lesson", None)
        );
        assert_ne!(
            git_source_key(url, Some("unit@lesson")),
            git_source_key("https://example.com/course@unit", Some("lesson"))
        );
    }

    #[test]
    fn staging_limits_track_course_files_and_git_metadata_separately() {
        let staging = tempfile::tempdir().expect("staging dir");
        let git_objects = staging.path().join(".git/objects/pack");
        fs::create_dir_all(&git_objects).expect("create git metadata directory");
        fs::write(staging.path().join("README.md"), vec![b'R'; 100]).expect("write course file");
        fs::write(git_objects.join("objects.pack"), vec![b'G'; 64])
            .expect("write git metadata file");

        let limits = StagingSizeLimits {
            pack_bytes: 100,
            git_metadata_bytes: 64,
            url_label: "fixture",
        };
        let mut usage = StagingSizeUsage::default();
        inspect_staging_tree(staging.path(), staging.path(), false, &limits, &mut usage)
            .expect("Git metadata must not count against the unpacked pack limit");
        assert_eq!(usage.pack_bytes, 100);
        assert_eq!(usage.git_metadata_bytes, 64);

        let limits = StagingSizeLimits {
            pack_bytes: 99,
            git_metadata_bytes: 64,
            url_label: "fixture",
        };
        let pack_error = inspect_staging_tree(
            staging.path(),
            staging.path(),
            false,
            &limits,
            &mut StagingSizeUsage::default(),
        )
        .expect_err("course files must use the unpacked pack limit");
        assert!(pack_error
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("unpacked course pack"));

        let limits = StagingSizeLimits {
            pack_bytes: 100,
            git_metadata_bytes: 63,
            url_label: "fixture",
        };
        let git_error = inspect_staging_tree(
            staging.path(),
            staging.path(),
            false,
            &limits,
            &mut StagingSizeUsage::default(),
        )
        .expect_err("Git metadata must use its separate temporary storage limit");
        assert!(git_error
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("Git metadata"));
    }
}
