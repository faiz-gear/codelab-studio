use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use cap_fs_ext::MetadataExt;
use cap_std::fs::{Dir, Metadata};

use crate::filesystem::{open_canonical_dir, open_dir_at};

pub(super) const METADATA_DIR: &str = ".codelab";

pub(crate) struct WorkspacePaths {
    pub root: PathBuf,
    pub dir: Dir,
    protected_file: Option<PathBuf>,
}

pub(super) fn invalid_path() -> io::Error {
    io::Error::from(io::ErrorKind::PermissionDenied)
}

pub(super) fn is_metadata(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| name.eq_ignore_ascii_case(METADATA_DIR))
}

pub(crate) fn is_sensitive_name(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name == ".env"
            || name.starts_with(".env.")
            || name.contains("secret")
            || name.contains("key")
    })
}

pub(crate) fn is_sensitive_path(path: &Path) -> bool {
    path.iter()
        .any(|name| is_metadata(name) || is_sensitive_name(name))
}

fn is_sensitive(name: &OsStr) -> bool {
    is_sensitive_name(name)
}

pub(super) fn is_ignored(path: &Path) -> bool {
    path.iter().any(|name| {
        is_metadata(name) || is_sensitive(name) || name == ".git" || name == "node_modules"
    })
}

pub(super) fn identity(metadata: &Metadata) -> (u64, u64) {
    (metadata.dev(), metadata.ino())
}

fn is_safe_component(name: &OsStr) -> bool {
    // A colon inside a Windows component can address a file's alternate stream.
    !cfg!(windows)
        || name
            .to_str()
            .is_some_and(|name| !name.contains([':', '\\', '/']) && !name.ends_with(['.', ' ']))
}

pub(crate) fn relative_path(path: &Path) -> io::Result<PathBuf> {
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) if is_safe_component(name) => relative.push(name),
            Component::CurDir => (),
            _ => return Err(invalid_path()),
        }
    }
    Ok(relative)
}

pub(super) fn wire_path(path: &Path) -> io::Result<String> {
    path.iter()
        .map(|part| part.to_str().ok_or_else(invalid_path))
        .collect::<io::Result<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

impl WorkspacePaths {
    pub fn open(root: PathBuf, protected_file: Option<PathBuf>) -> io::Result<Self> {
        let dir = open_canonical_dir(&root)?;
        Ok(Self {
            root,
            dir,
            protected_file,
        })
    }

    pub fn input(&self, input: &str) -> io::Result<PathBuf> {
        let current = open_canonical_dir(&self.root).map_err(|_| invalid_path())?;
        if identity(&current.dir_metadata()?) != identity(&self.dir.dir_metadata()?) {
            return Err(invalid_path());
        }
        let path = Path::new(input);
        if path.is_absolute() {
            if path
                .components()
                .any(|component| component == Component::ParentDir)
            {
                return Err(invalid_path());
            }
            // Native dialogs and users may supply an absolute path through a root alias.
            if let Ok(path) = path.strip_prefix(&self.root) {
                return relative_path(path);
            }
            let parent = path.parent().ok_or_else(invalid_path)?;
            let name = path.file_name().ok_or_else(invalid_path)?;
            if !is_safe_component(name) {
                return Err(invalid_path());
            }
            let parent = fs::canonicalize(parent).map_err(|_| invalid_path())?;
            return parent
                .join(name)
                .strip_prefix(&self.root)
                .map_err(|_| invalid_path())
                .and_then(relative_path);
        }
        relative_path(path)
    }

    pub fn validate(&self, path: &Path) -> io::Result<()> {
        if path
            .iter()
            .any(|name| is_metadata(name) || is_sensitive(name))
            || self.protected_file.as_deref() == Some(path)
        {
            return Err(invalid_path());
        }
        Ok(())
    }

    pub fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        self.validate(path)?;
        let absolute = fs::canonicalize(self.root.join(path))?;
        let resolved = absolute
            .strip_prefix(&self.root)
            .map_err(|_| invalid_path())?;
        self.validate(resolved)?;
        Ok(resolved.to_path_buf())
    }

    pub fn validate_missing(&self, path: &Path) -> io::Result<()> {
        self.validate(path)?;
        for ancestor in path.ancestors() {
            match fs::symlink_metadata(self.root.join(ancestor)) {
                Ok(metadata) if !metadata.is_dir() => return Err(invalid_path()),
                Ok(_) => return self.canonical(ancestor).map(|_| ()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Err(invalid_path())
    }

    pub fn parent(&self, path: &Path) -> io::Result<(Dir, PathBuf)> {
        self.validate(path)?;
        let name = path.file_name().ok_or_else(invalid_path)?;
        let parent = self.canonical(path.parent().ok_or_else(invalid_path)?)?;
        let resolved = parent.join(name);
        self.validate(&resolved)?;
        Ok((open_dir_at(&self.dir, &parent)?, resolved))
    }
}
