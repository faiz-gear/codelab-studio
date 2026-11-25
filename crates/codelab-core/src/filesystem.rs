use std::io;
use std::path::Path;

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};

pub(crate) fn open_regular(dir: &Dir, path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    // A file may be swapped for a named pipe between validation and open.
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = dir.open_with(path, &options)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    Ok(file)
}

pub(crate) fn open_dir_at(root: &Dir, path: &Path) -> io::Result<Dir> {
    let mut dir = root.try_clone()?;
    for name in path.iter() {
        dir = dir.open_dir_nofollow(name)?;
    }
    Ok(dir)
}

/// Open an already canonical path without following a newer link in any component.
pub(crate) fn open_canonical_dir(path: &Path) -> io::Result<Dir> {
    let volume = path
        .ancestors()
        .last()
        .filter(|root| root.is_absolute())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let relative = path
        .strip_prefix(volume)
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    let directory = Dir::open_ambient_dir(volume, ambient_authority())?;
    open_dir_at(&directory, relative)
}

pub(crate) fn open_canonical_file(path: &Path) -> io::Result<File> {
    let parent = open_canonical_dir(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    open_regular(
        &parent,
        Path::new(path.file_name().ok_or(io::ErrorKind::InvalidInput)?),
    )
}
