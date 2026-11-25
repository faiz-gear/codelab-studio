use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};
use url::Url;

use super::{CourseSource, OpenedCourse, Starter, pack};
use crate::filesystem::{open_canonical_dir, open_canonical_file};
use crate::{DomainError, ErrorCode};

pub trait GitFetchAdapter: Send + Sync {
    /// Check out the upstream default branch into the supplied empty directory.
    fn checkout(&self, url: &str, destination: &Path) -> io::Result<()>;
}

pub(crate) const CACHE_DIRECTORY: &str = "git-courses";

pub(crate) struct GitCli;

impl GitFetchAdapter for GitCli {
    fn checkout(&self, url: &str, destination: &Path) -> io::Result<()> {
        // Preview occurs before Trust. Avoid local Git hooks, checkout filters, remote
        // helpers, and submodules while downloading files from an external Course Pack.
        let transport = Url::parse(url).ok();
        let url = transport.as_ref().map_or(url, Url::as_str);
        let mut git = Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.as_encoded_bytes().starts_with(b"GIT_") {
                git.env_remove(name);
            }
        }
        let status = git
            .current_dir(destination)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes -oConnectTimeout=10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args([
                "-c",
                "protocol.allow=never",
                "-c",
                "protocol.https.allow=always",
                "-c",
                "protocol.http.allow=always",
                "-c",
                "protocol.ssh.allow=always",
                "-c",
                "http.lowSpeedLimit=1",
                "-c",
                "http.lowSpeedTime=30",
                "clone",
                "--quiet",
                "--depth=1",
                "--no-tags",
                "--no-recurse-submodules",
                "--template=",
                "--",
                url,
                ".",
            ])
            .status()?;
        if !status.success() {
            return Err(io::Error::other("git clone failed"));
        }
        Ok(())
    }
}

struct CachedSnapshot {
    snapshot: String,
    fetch_url: String,
}

pub(crate) struct PreparedCourse {
    opened: OpenedCourse,
    snapshot: Option<PreparedSnapshot>,
}

struct PreparedSnapshot {
    staging: TempDir,
    permissions: Vec<(PathBuf, fs::Permissions)>,
    new_pointer: NamedTempFile,
    pointer: PathBuf,
}

pub(crate) fn prepare(
    data_dir: &Path,
    raw_url: &str,
    fetcher: &dyn GitFetchAdapter,
    refresh: bool,
) -> Result<PreparedCourse, DomainError> {
    let url = normalize_git_url(raw_url)
        .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
    let namespace = namespace(data_dir, &url);
    create_cache_directory(&namespace)?;
    let pointer = namespace.join("current");
    let cached = read_pointer(&pointer, &url).map_err(cache_error)?;
    if !refresh && let Some(cached) = &cached {
        return Ok(PreparedCourse {
            opened: open_snapshot(&namespace.join(&cached.snapshot), url)?,
            snapshot: None,
        });
    }

    let fetch_url = cached
        .as_ref()
        .map_or(raw_url.trim(), |cached| cached.fetch_url.as_str());
    let staging = TempDir::with_prefix_in("snapshot-", &namespace).map_err(cache_error)?;
    fetcher
        .checkout(fetch_url, staging.path())
        .map_err(|_| DomainError::new(ErrorCode::GitCourseFetchFailed).with_detail("url", &url))?;
    let git_metadata = staging.path().join(".git");
    match fs::remove_dir_all(git_metadata) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(cache_error(error)),
    }
    let opened = open_snapshot(staging.path(), url)?;
    let permissions = snapshot_permissions(staging.path()).map_err(cache_error)?;
    let mut new_pointer = NamedTempFile::new_in(&namespace).map_err(cache_error)?;
    let snapshot = staging
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DomainError::new(ErrorCode::GitCourseCacheFailed))?;
    writeln!(new_pointer, "{snapshot}\n{fetch_url}").map_err(cache_error)?;
    new_pointer.flush().map_err(cache_error)?;
    new_pointer.as_file().sync_all().map_err(cache_error)?;

    Ok(PreparedCourse {
        opened,
        snapshot: Some(PreparedSnapshot {
            staging,
            permissions,
            new_pointer,
            pointer,
        }),
    })
}

impl PreparedCourse {
    pub(crate) fn publish(self) -> Result<OpenedCourse, DomainError> {
        if let Some(PreparedSnapshot {
            staging,
            permissions,
            new_pointer,
            pointer,
        }) = self.snapshot
        {
            // Keep previous snapshots available for Workspaces seeded from them.
            let publish = || -> Result<(), DomainError> {
                for (path, original) in &permissions {
                    let mut readonly = original.clone();
                    readonly.set_readonly(true);
                    fs::set_permissions(path, readonly).map_err(cache_error)?;
                }
                new_pointer.persist(&pointer).map_err(cache_error)?;
                Ok(())
            };
            if let Err(error) = publish() {
                for (path, original) in permissions.into_iter().rev() {
                    let _ = fs::set_permissions(path, original);
                }
                return Err(error);
            }
            let _ = staging.keep();
            #[cfg(unix)]
            if let Some(directory) = pointer.parent().and_then(|path| fs::File::open(path).ok()) {
                let _ = directory.sync_all();
            }
        }
        Ok(self.opened)
    }
}

fn snapshot_permissions(root: &Path) -> io::Result<Vec<(PathBuf, fs::Permissions)>> {
    fn collect(root: &Path, permissions: &mut Vec<(PathBuf, fs::Permissions)>) -> io::Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                collect(&entry.path(), permissions)?;
            } else if kind.is_file() {
                permissions.push((entry.path(), entry.metadata()?.permissions()));
            }
        }
        #[cfg(unix)]
        permissions.push((root.to_owned(), fs::symlink_metadata(root)?.permissions()));
        Ok(())
    }

    let mut permissions = Vec::new();
    collect(root, &mut permissions)?;
    Ok(permissions)
}

fn create_cache_directory(namespace: &Path) -> Result<(), DomainError> {
    for directory in [
        namespace.parent().expect("cache namespace has parent"),
        namespace,
    ] {
        match fs::create_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(cache_error(error)),
        }
        open_canonical_dir(directory).map_err(cache_error)?;
    }
    Ok(())
}

fn read_pointer(path: &Path, url: &str) -> io::Result<Option<CachedSnapshot>> {
    let file = match open_canonical_file(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut content = String::new();
    file.take(16 * 1024).read_to_string(&mut content)?;
    let mut lines = content.lines();
    let snapshot = lines.next().unwrap_or_default();
    let fetch_url = lines.next().unwrap_or_default();
    if !valid_snapshot_key(snapshot)
        || normalize_git_url(fetch_url).as_deref() != Some(url)
        || lines.next().is_some()
    {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    Ok(Some(CachedSnapshot {
        snapshot: snapshot.into(),
        fetch_url: fetch_url.into(),
    }))
}

pub(crate) fn valid_snapshot_key(key: &str) -> bool {
    key.strip_prefix("snapshot-").is_some_and(|name| {
        !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}

pub(crate) fn snapshot_key(starter: Option<&Starter>) -> Option<&str> {
    let Starter::Local { root, .. } = starter? else {
        return None;
    };
    root.file_name()?
        .to_str()
        .filter(|key| valid_snapshot_key(key))
}

pub(crate) fn snapshot_starter(
    data_dir: &Path,
    url: &str,
    key: &str,
) -> Result<Option<Starter>, DomainError> {
    if !valid_snapshot_key(key) {
        return Err(DomainError::new(ErrorCode::GitCourseCacheFailed));
    }
    let path = namespace(data_dir, url).join(key);
    open_canonical_dir(&path).map_err(cache_error)?;
    pack::starter_from_manifest(&path).map(Some)
}

fn namespace(data_dir: &Path, url: &str) -> PathBuf {
    data_dir
        .join(CACHE_DIRECTORY)
        .join(format!("{:x}", Sha256::digest(url.as_bytes())))
}

fn open_snapshot(path: &Path, url: String) -> Result<OpenedCourse, DomainError> {
    open_canonical_dir(path).map_err(cache_error)?;
    let mut course = pack::load_local_course_pack(&path.to_string_lossy())?;
    course.course.source = CourseSource::GitPack { url };
    course.watched_files.clear();
    Ok(course)
}

fn cache_error(_err: impl std::fmt::Display) -> DomainError {
    DomainError::new(ErrorCode::GitCourseCacheFailed)
}

pub(crate) fn normalize_git_url(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.chars().any(|ch| ch.is_whitespace() || ch.is_control()) || raw.contains(['\\', '?', '#'])
    {
        return None;
    }
    if let Some(scp) = raw.strip_prefix("git@")
        && let Some((host, path)) = scp.split_once(':')
        && !host.is_empty()
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte))
        && !host.starts_with('-')
    {
        let prefix = if path.starts_with('/') { "/" } else { "" };
        return Some(format!(
            "git@{}:{prefix}{}",
            host.to_ascii_lowercase(),
            clean_path(path)?
        ));
    }
    let parsed = Url::parse(raw).ok()?;
    let scheme = parsed.scheme();
    if !matches!(scheme, "http" | "https" | "ssh")
        || parsed.password().is_some()
        || (scheme != "ssh" && !parsed.username().is_empty())
        || !raw.contains("://")
    {
        return None;
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let user = if parsed.username().is_empty() {
        String::new()
    } else {
        format!("{}@", parsed.username())
    };
    let port = parsed
        .port()
        .map_or_else(String::new, |port| format!(":{port}"));
    Some(format!(
        "{scheme}://{user}{host}{port}/{}",
        clean_path(parsed.path())?
    ))
}

fn clean_path(path: &str) -> Option<&str> {
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    (!path.is_empty()).then_some(path)
}
