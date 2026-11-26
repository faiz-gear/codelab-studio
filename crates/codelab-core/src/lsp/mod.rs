mod protocol;
mod worker;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

pub use protocol::*;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::execution::Directory;
use crate::filesystem::{open_dir_at, open_regular};
use crate::runtime::{HostTool, RuntimeManager};
use crate::workspace::{MAX_WORKSPACE_TEXT_BYTES, WorkspacePaths};
use crate::{AppEvent, CourseLanguage};

const MAX_SERVERS: usize = 3;
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDocumentUpdate {
    pub workspace_id: u64,
    pub version: i32,
    pub document: Option<LanguageDocument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageDocument {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageStatus {
    Starting,
    Ready,
    Missing,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDocumentVersion {
    pub workspace_id: u64,
    pub path: String,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStatusChanged {
    #[serde(flatten)]
    pub document: LanguageDocumentVersion,
    pub language: CourseLanguage,
    pub status: LanguageStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageDiagnostics {
    #[serde(flatten)]
    pub document: LanguageDocumentVersion,
    pub diagnostics: Vec<LanguageDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageFeature {
    Completion,
    Hover,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageFeatureRequest {
    pub workspace_id: u64,
    pub version: i32,
    pub feature: LanguageFeature,
    pub line: u32,
    pub character: u32,
}

pub(crate) type Emit = Arc<dyn Fn(AppEvent) + Send + Sync>;

#[derive(Default)]
pub(crate) struct LanguageServers {
    servers: HashMap<Module, worker::Worker>,
    workspace_id: Option<u64>,
    version: i32,
    active: Option<Module>,
    path: Option<String>,
    unavailable: Option<LanguageStatus>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Module {
    root: PathBuf,
    language: CourseLanguage,
}

struct Source {
    document: LanguageDocumentVersion,
    uri: Url,
    language_id: &'static str,
    text: String,
}

struct Language {
    course: CourseLanguage,
    tool: HostTool,
    id: &'static str,
    markers: &'static [&'static str],
}

impl Language {
    fn for_path(path: &Path) -> Option<Self> {
        use CourseLanguage::*;
        const JS_TS: &[&str] = &["package.json", "tsconfig.json", "jsconfig.json"];
        let extension = path.extension()?.to_str()?;
        let (course, tool, id, markers): (_, _, _, &[_]) = match extension {
            "go" => (Go, HostTool::Gopls, "go", &["go.mod", "go.work"]),
            "js" | "mjs" | "cjs" => (
                JsTs,
                HostTool::TypeScriptLanguageServer,
                "javascript",
                JS_TS,
            ),
            "jsx" => (
                JsTs,
                HostTool::TypeScriptLanguageServer,
                "javascriptreact",
                JS_TS,
            ),
            "ts" | "mts" | "cts" => (
                JsTs,
                HostTool::TypeScriptLanguageServer,
                "typescript",
                JS_TS,
            ),
            "tsx" => (
                JsTs,
                HostTool::TypeScriptLanguageServer,
                "typescriptreact",
                JS_TS,
            ),
            "py" | "pyi" => (
                Python,
                HostTool::Pyright,
                "python",
                &[
                    "pyproject.toml",
                    "pyrightconfig.json",
                    "setup.py",
                    "setup.cfg",
                ],
            ),
            "java" => (
                Java,
                HostTool::Jdtls,
                "java",
                &[
                    "pom.xml",
                    "build.gradle",
                    "build.gradle.kts",
                    "settings.gradle",
                    "settings.gradle.kts",
                ],
            ),
            _ => return None,
        };
        Some(Self {
            course,
            tool,
            id,
            markers,
        })
    }

    fn directory(&self, workspace: &WorkspacePaths, path: &Path) -> std::io::Result<Directory> {
        for relative in path.parent().into_iter().flat_map(Path::ancestors) {
            let handle = open_dir_at(&workspace.dir, relative)?;
            if relative.as_os_str().is_empty()
                || self.markers.iter().any(|marker| {
                    workspace
                        .canonical(&relative.join(marker))
                        .is_ok_and(|marker| open_regular(&workspace.dir, &marker).is_ok())
                })
            {
                return Ok(Directory {
                    path: workspace.root.join(relative),
                    handle,
                });
            }
        }
        Err(std::io::ErrorKind::InvalidInput.into())
    }
}

impl LanguageServers {
    pub(crate) fn suspend(&mut self) {
        self.servers.clear();
        self.active = None;
        self.path = None;
        self.unavailable = None;
    }

    pub(crate) fn sync(
        &mut self,
        files: &WorkspacePaths,
        update: LanguageDocumentUpdate,
        runtime: &RuntimeManager,
        emit: &Emit,
    ) {
        if self.workspace_id != Some(update.workspace_id) {
            self.workspace_id = Some(update.workspace_id);
            self.version = 0;
        }
        if update.version <= self.version {
            return;
        }
        self.version = update.version;
        let Some(document) = update.document else {
            self.path = None;
            self.select(None);
            return;
        };
        let new_path = self.path.as_ref() != Some(&document.path);
        if new_path {
            self.path = Some(document.path.clone());
            self.unavailable = None;
        }
        let Some(language) = Language::for_path(Path::new(&document.path)) else {
            self.select(None);
            return;
        };
        let target = LanguageDocumentVersion {
            workspace_id: update.workspace_id,
            path: document.path,
            version: update.version,
        };
        let result = self.sync_source(
            files,
            &target,
            document.text,
            &language,
            new_path,
            runtime,
            emit,
        );
        if let Err(status) = result {
            self.select(None);
            self.unavailable = Some(status);
            status_event(emit, &target, language.course, status);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sync_source(
        &mut self,
        files: &WorkspacePaths,
        target: &LanguageDocumentVersion,
        text: String,
        language: &Language,
        new_path: bool,
        runtime: &RuntimeManager,
        emit: &Emit,
    ) -> Result<(), LanguageStatus> {
        use LanguageStatus::{Degraded, Missing};
        if let Some(status) = self.unavailable {
            return Err(status);
        }
        if text.len() > MAX_WORKSPACE_TEXT_BYTES {
            // The next edit can bring the draft back below the limit.
            status_event(emit, target, language.course, Degraded);
            self.select(None);
            return Ok(());
        }
        // The editor supplies an already contained canonical Workspace path, but
        // revalidate it at the process boundary before selecting any Module Root.
        let relative = files
            .input(&target.path)
            .and_then(|path| files.canonical(&path))
            .map_err(|_| Degraded)?;
        if open_regular(&files.dir, &relative).is_err() {
            return Err(Degraded);
        }
        let directory = language.directory(files, &relative).map_err(|_| Degraded)?;
        let uri = file_uri(&files.root.join(relative)).ok_or(Degraded)?;
        let module = Module {
            root: directory.path.clone(),
            language: language.course,
        };
        self.select(Some(&module));
        let source = Arc::new(Source {
            document: target.clone(),
            uri,
            language_id: language.id,
            text,
        });
        if new_path {
            self.servers.retain(|_, worker| !worker.finished());
        }
        if let Some(worker) = self.servers.get_mut(&module) {
            if worker.focus(Some(Arc::clone(&source))) {
                worker.last_used = Instant::now();
                return Ok(());
            }
            if !new_path {
                return Err(Degraded);
            }
            self.servers.remove(&module);
        }
        let executable = runtime
            .locate_language_server(language.tool)
            .ok_or(Missing)?;
        if self.servers.len() >= MAX_SERVERS {
            let oldest = self
                .servers
                .iter()
                .min_by_key(|(_, server)| server.last_used)
                .map(|(module, _)| module.clone())
                .expect("nonempty pool");
            self.servers.remove(&oldest);
        }
        status_event(emit, target, language.course, LanguageStatus::Starting);
        let worker =
            worker::Worker::start(executable, directory, language, source, Arc::clone(emit))
                .map_err(|_| Degraded)?;
        self.servers.insert(module, worker);
        Ok(())
    }

    fn select(&mut self, next: Option<&Module>) {
        if self.active.as_ref() == next {
            return;
        }
        if let Some(previous) = self
            .active
            .take()
            .and_then(|module| self.servers.get_mut(&module))
        {
            previous.last_used = Instant::now();
            previous.focus(None);
        }
        self.active = next.cloned();
    }

    pub(crate) fn request(
        &self,
        request: LanguageFeatureRequest,
    ) -> Option<mpsc::Receiver<LanguageFeatureResult>> {
        if request.version != self.version {
            return None;
        }
        self.servers.get(self.active.as_ref()?)?.request(request)
    }
}

fn file_uri(path: &Path) -> Option<Url> {
    #[cfg(windows)]
    let path = dunce::simplified(path);
    Url::from_file_path(path).ok()
}

fn status_event(
    emit: &Emit,
    target: &LanguageDocumentVersion,
    language: CourseLanguage,
    status: LanguageStatus,
) {
    emit(AppEvent::LanguageStatusChanged(LanguageStatusChanged {
        document: target.clone(),
        language,
        status,
    }));
}
