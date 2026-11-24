use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use flate2::read::GzDecoder;
use serde::Deserialize;

use crate::{CourseLanguage, DomainError, ErrorCode};

use super::probe::executable_name;
use super::{InstallProgress, ManagedRuntime, RuntimeArtifact, RuntimeDownloadAdapter};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    bun_version: String,
    go_version: String,
    java_version: String,
    python_version: String,
    platforms: HashMap<String, PlatformArtifacts>,
    junit: RuntimeArtifact,
    python_wheels: Vec<RuntimeArtifact>,
    windows_python_wheel: RuntimeArtifact,
}

#[derive(Deserialize)]
struct PlatformArtifacts {
    bun: RuntimeArtifact,
    go: RuntimeArtifact,
    java: RuntimeArtifact,
    uv: RuntimeArtifact,
    python: RuntimeArtifact,
}

// The catalog uses publisher digests. Python builds and installation keys match
// the pinned uv release's download-metadata.json.
static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("artifacts.json")).expect("bundled runtime catalog")
});

#[derive(Clone, Copy)]
enum Layout {
    Wrapped,
    Flat,
    File,
}

struct Asset<'a> {
    artifact: &'a RuntimeArtifact,
    destination: PathBuf,
    layout: Layout,
}

pub(super) struct RuntimeRecipe {
    version: &'static str,
    directory: String,
    executable: PathBuf,
    test_executor: PathBuf,
    uv: Option<PathBuf>,
    required_files: Vec<PathBuf>,
    assets: Vec<Asset<'static>>,
}

impl RuntimeRecipe {
    pub(super) fn for_language(language: CourseLanguage) -> Option<Self> {
        let catalog = &*CATALOG;
        let os = env::consts::OS;
        let arch = env::consts::ARCH;
        let platform = catalog.platforms.get(&format!("{os}-{arch}"))?;
        let mut recipe = match language {
            CourseLanguage::JsTs => Self::toolchain(
                "bun",
                &catalog.bun_version,
                &platform.bun,
                PathBuf::from(executable_name("bun")),
            ),
            CourseLanguage::Go => Self::toolchain(
                "go",
                &catalog.go_version,
                &platform.go,
                Path::new("bin").join(executable_name("go")),
            ),
            CourseLanguage::Java => {
                // The Windows ARM64 build uses Temurin's x64 JDK under Windows emulation.
                let home = if os == "macos" { "Contents/Home" } else { "" };
                let bin = Path::new(home).join("bin");
                let mut recipe = Self::toolchain(
                    "java",
                    &catalog.java_version,
                    &platform.java,
                    bin.join(executable_name("java")),
                );
                recipe.test_executor = PathBuf::from(catalog.junit.filename());
                recipe.assets.push(Asset {
                    artifact: &catalog.junit,
                    destination: recipe.test_executor.clone(),
                    layout: Layout::File,
                });
                recipe.required_files.push(
                    Path::new("toolchain")
                        .join(bin)
                        .join(executable_name("javac")),
                );
                recipe
            }
            CourseLanguage::Python => Self::python(catalog, platform, os, arch),
        };
        recipe.required_files.push(recipe.executable.clone());
        recipe.required_files.push(recipe.test_executor.clone());
        Some(recipe)
    }

    fn toolchain(
        name: &str,
        version: &'static str,
        artifact: &'static RuntimeArtifact,
        executable: PathBuf,
    ) -> Self {
        let destination = PathBuf::from("toolchain");
        let executable = destination.join(executable);
        Self {
            version,
            directory: format!("{name}-{version}"),
            test_executor: executable.clone(),
            executable,
            uv: None,
            required_files: Vec::new(),
            assets: vec![Asset {
                artifact,
                destination,
                layout: Layout::Wrapped,
            }],
        }
    }

    fn python(
        catalog: &'static Catalog,
        platform: &'static PlatformArtifacts,
        os: &str,
        arch: &str,
    ) -> Self {
        let version = &catalog.python_version;
        let python_os = if os == "macos" { "darwin" } else { os };
        let libc = if os == "linux" { "gnu" } else { "none" };
        let install_dir =
            Path::new("python").join(format!("cpython-{version}-{python_os}-{arch}-{libc}"));
        let (python, site_packages) = if os == "windows" {
            ("python.exe", "Lib/site-packages")
        } else {
            ("bin/python3.12", "lib/python3.12/site-packages")
        };
        let uv_dir = PathBuf::from("uv");
        let uv = uv_dir.join(executable_name("uv"));
        let executable = install_dir.join(python);
        let site_packages = install_dir.join(site_packages);
        let mut assets = vec![
            Asset {
                artifact: &platform.uv,
                destination: uv_dir,
                layout: if os == "windows" {
                    Layout::Flat
                } else {
                    Layout::Wrapped
                },
            },
            Asset {
                artifact: &platform.python,
                destination: install_dir,
                layout: Layout::Wrapped,
            },
        ];
        // These pinned wheels are pure Python. Their root contents are importable
        // from site-packages, so python -m pytest works after relocating the tree.
        let wheels = catalog
            .python_wheels
            .iter()
            .chain((os == "windows").then_some(&catalog.windows_python_wheel));
        assets.extend(wheels.map(|artifact| Asset {
            artifact,
            destination: site_packages.clone(),
            layout: Layout::Flat,
        }));
        Self {
            version,
            directory: format!("python-{version}"),
            test_executor: executable.clone(),
            executable,
            uv: Some(uv.clone()),
            required_files: vec![
                uv,
                site_packages.join("pytest/__main__.py"),
                site_packages.join("pytest_jsonreport/plugin.py"),
            ],
            assets,
        }
    }

    pub(super) fn installed(&self, install_dir: &Path) -> Option<ManagedRuntime> {
        self.runtime_at(&install_dir.join(&self.directory))
    }

    fn runtime_at(&self, root: &Path) -> Option<ManagedRuntime> {
        self.required_files
            .iter()
            .all(|path| root.join(path).is_file())
            .then(|| ManagedRuntime {
                version: self.version.to_owned(),
                executable: root.join(&self.executable),
                test_executor: root.join(&self.test_executor),
                uv: self.uv.as_ref().map(|path| root.join(path)),
            })
    }

    pub(super) fn install(
        &self,
        root: &Path,
        downloader: &dyn RuntimeDownloadAdapter,
        mut publish: impl FnMut(InstallProgress),
    ) -> Result<ManagedRuntime, DomainError> {
        if let Some(existing) = self.installed(root) {
            return Ok(existing);
        }
        fs::create_dir_all(root).map_err(install_error)?;
        let staging = tempfile::Builder::new()
            .prefix(".install-")
            .tempdir_in(root)
            .map_err(install_error)?;
        let payload = staging.path().join("runtime");
        fs::create_dir(&payload).map_err(install_error)?;
        for asset in &self.assets {
            let artifact = asset.artifact.filename().to_owned();
            let download = staging.path().join("download");
            publish(InstallProgress::Downloading {
                artifact: artifact.clone(),
                received_bytes: 0,
                total_bytes: None,
            });
            {
                let mut file = File::create(&download).map_err(install_error)?;
                downloader.download(
                    asset.artifact,
                    &mut file,
                    &mut |received_bytes, total_bytes| {
                        publish(InstallProgress::Downloading {
                            artifact: artifact.clone(),
                            received_bytes,
                            total_bytes,
                        });
                    },
                )?;
            }
            let destination = payload.join(&asset.destination);
            publish(InstallProgress::Extracting { artifact });
            unpack(asset, &download, &destination, staging.path()).map_err(install_error)?;
        }

        if self.runtime_at(&payload).is_none() {
            return Err(DomainError::new(ErrorCode::RuntimeInstallFailed));
        }

        // Multiple app processes can stage independently. Only one publishes a given
        // release, and a concurrent installer can reuse the complete published tree.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join(format!(".{}.lock", self.directory)))
            .map_err(install_error)?;
        lock.lock().map_err(install_error)?;
        if let Some(existing) = self.installed(root) {
            return Ok(existing);
        }
        let destination = root.join(&self.directory);
        if destination.symlink_metadata().is_ok() {
            fs::rename(&destination, staging.path().join("incomplete")).map_err(install_error)?;
        }
        fs::rename(&payload, &destination).map_err(install_error)?;
        self.installed(root)
            .ok_or_else(|| DomainError::new(ErrorCode::RuntimeInstallFailed))
    }
}

fn unpack(
    asset: &Asset<'_>,
    download: &Path,
    destination: &Path,
    staging: &Path,
) -> io::Result<()> {
    if matches!(asset.layout, Layout::File) {
        return fs::rename(download, destination);
    }
    let target = if matches!(asset.layout, Layout::Wrapped) {
        staging.join("unpacked")
    } else {
        destination.to_path_buf()
    };
    fs::create_dir_all(&target)?;
    let input = File::open(download)?;
    if asset.artifact.filename().ends_with(".tar.gz") {
        let mut archive = tar::Archive::new(GzDecoder::new(input));
        archive.set_overwrite(false);
        archive.unpack(&target)?;
    } else {
        zip::ZipArchive::new(input)?.extract(&target)?;
    }
    if matches!(asset.layout, Layout::Wrapped) {
        let mut entries = fs::read_dir(&target)?;
        let folder = entries
            .next()
            .transpose()?
            .ok_or(io::ErrorKind::InvalidData)?;
        if !folder.file_type()?.is_dir() || entries.next().is_some() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        fs::create_dir_all(destination.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        fs::rename(folder.path(), destination)?;
        fs::remove_dir(target)?;
    }
    Ok(())
}

fn install_error(_: io::Error) -> DomainError {
    DomainError::new(ErrorCode::RuntimeInstallFailed)
}
