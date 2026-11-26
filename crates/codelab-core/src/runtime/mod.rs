mod download;
mod install;
mod model;
mod probe;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use crate::storage::JsonStore;
use crate::{CourseLanguage, DomainError, ErrorCode, StorageRecoveryNotice, StoreKind};

pub use download::{HttpRuntimeDownloader, RuntimeArtifact, RuntimeDownloadAdapter};
use install::RuntimeRecipe;

pub use model::*;
pub use probe::{HostProbeResult, HostToolchainProbe};
pub(crate) use probe::{executable_name, find_host_executable, probe_executable_output};

const LANGUAGES: [(CourseLanguage, &[HostTool], HostTool); 4] = [
    (
        CourseLanguage::JsTs,
        &[HostTool::Bun, HostTool::Node],
        HostTool::TypeScriptLanguageServer,
    ),
    (CourseLanguage::Go, &[HostTool::Go], HostTool::Gopls),
    (CourseLanguage::Java, &[HostTool::Javac], HostTool::Jdtls),
    (
        CourseLanguage::Python,
        &[HostTool::Python3, HostTool::Python, HostTool::UvPython],
        HostTool::Pyright,
    ),
];

const SUPPORT_TOOLS: [HostTool; 3] = [HostTool::Git, HostTool::Curl, HostTool::Uv];

pub struct RuntimeAdapters {
    pub host: Arc<dyn HostToolchainProbe>,
    pub downloads: Arc<dyn RuntimeDownloadAdapter>,
}

impl Default for RuntimeAdapters {
    fn default() -> Self {
        Self {
            host: Arc::new(probe::SystemToolchainProbe),
            downloads: Arc::new(HttpRuntimeDownloader),
        }
    }
}

pub(crate) struct RuntimeManager {
    report: Mutex<PreflightReport>,
    preferences: JsonStore<Vec<CourseLanguage>>,
    downloads: Arc<dyn RuntimeDownloadAdapter>,
    host: Arc<dyn HostToolchainProbe>,
    on_update: Box<dyn Fn(PreflightReport) + Send + Sync>,
}

impl RuntimeManager {
    pub(crate) fn open(
        data_dir: &Path,
        adapters: RuntimeAdapters,
        on_update: impl Fn(PreflightReport) + Send + Sync + 'static,
    ) -> Result<(Arc<Self>, Option<StorageRecoveryNotice>), DomainError> {
        let preferences = JsonStore::new(
            data_dir,
            "runtime-preferences.json",
            StoreKind::RuntimePreferences,
        );
        let loaded = preferences.load_or_recover(|languages: &Vec<CourseLanguage>| {
            languages
                .iter()
                .enumerate()
                .all(|(index, language)| !languages[..index].contains(language))
        })?;
        let install_dir = data_dir.join("runtimes");
        let languages = LANGUAGES
            .iter()
            .map(|&(language, _, _)| {
                let mut row = LanguagePreflight {
                    language,
                    status: ProbeStatus::Missing,
                    system: Vec::new(),
                    lsp: None,
                    managed: RuntimeRecipe::for_language(language)
                        .and_then(|recipe| recipe.installed(&install_dir)),
                    selected: None,
                    prefer_managed: loaded.value.contains(&language),
                    can_install: false,
                    install: None,
                };
                row.resolve(false);
                row
            })
            .collect();
        let manager = Arc::new(Self {
            report: Mutex::new(PreflightReport {
                revision: 0,
                checked: false,
                probing: false,
                installing_count: 0,
                languages,
                tools: Vec::new(),
                go_proxy: None,
                install_dir,
            }),
            preferences,
            downloads: adapters.downloads,
            host: adapters.host,
            on_update: Box::new(on_update),
        });
        Ok((manager, loaded.recovery))
    }

    pub(crate) fn locate_language_server(&self, tool: HostTool) -> Option<PathBuf> {
        self.host.locate_language_server(tool)
    }

    pub(crate) fn snapshot(&self) -> PreflightReport {
        self.lock_report().clone()
    }

    pub(crate) fn refresh(self: &Arc<Self>) -> Result<PreflightReport, DomainError> {
        let mut report = self.lock_report();
        if report.probing {
            return Ok(report.clone());
        }
        let managed_go = managed_go_path(&report).map(Path::to_path_buf);
        let runtime = Arc::clone(self);
        thread::Builder::new()
            .name("codelab-preflight".into())
            .spawn(move || {
                let tools = LANGUAGES
                    .into_iter()
                    .flat_map(|(_, tools, lsp)| tools.iter().copied().chain([lsp]))
                    .chain(SUPPORT_TOOLS)
                    .chain([HostTool::GoProxy]);
                let checks = probe::probe_all(&runtime.host, managed_go.as_deref(), tools);
                runtime.finish_probe(checks, managed_go.as_deref());
            })
            .map_err(|_| DomainError::new(ErrorCode::RuntimeTaskStartFailed))?;
        report.probing = true;
        Ok(self.publish(&mut report))
    }

    pub(crate) fn set_preference(
        &self,
        preference: RuntimePreference,
    ) -> Result<PreflightReport, DomainError> {
        let mut report = self.lock_report();
        let index = language_index(preference.language);
        if report.languages[index].prefer_managed == preference.prefer_managed {
            return Ok(report.clone());
        }
        let languages = report
            .languages
            .iter()
            .filter_map(|row| {
                let prefer_managed = if row.language == preference.language {
                    preference.prefer_managed
                } else {
                    row.prefer_managed
                };
                prefer_managed.then_some(row.language)
            })
            .collect();
        self.preferences.save_atomic(&languages)?;
        report.languages[index].prefer_managed = preference.prefer_managed;
        Ok(self.publish(&mut report))
    }

    pub(crate) fn install(
        self: &Arc<Self>,
        language: CourseLanguage,
    ) -> Result<PreflightReport, DomainError> {
        let mut report = self.lock_report();
        let index = language_index(language);
        if !report.languages[index].can_install {
            return Ok(report.clone());
        }
        let recipe = RuntimeRecipe::for_language(language)
            .ok_or_else(|| DomainError::new(ErrorCode::RuntimePlatformUnsupported))?;
        let root = report.install_dir.clone();
        let runtime = Arc::clone(self);
        thread::Builder::new()
            .name(format!("codelab-runtime-{language:?}"))
            .spawn(move || {
                let result = recipe.install(&root, runtime.downloads.as_ref(), |progress| {
                    let mut report = runtime.lock_report();
                    report.languages[index].install = Some(progress);
                    runtime.publish(&mut report);
                });
                let mut report = runtime.lock_report();
                let row = &mut report.languages[index];
                row.install = Some(match result {
                    Ok(managed) => {
                        row.managed = Some(managed);
                        InstallProgress::Installed
                    }
                    Err(error) => InstallProgress::Failed { error },
                });
                let recheck_proxy = language == CourseLanguage::Go
                    && matches!(row.install, Some(InstallProgress::Installed))
                    && !report.probing;
                runtime.publish(&mut report);
                drop(report);
                if recheck_proxy {
                    let _ = runtime.refresh();
                }
            })
            .map_err(|_| DomainError::new(ErrorCode::RuntimeTaskStartFailed))?;
        report.languages[index].install = Some(InstallProgress::Preparing);
        Ok(self.publish(&mut report))
    }

    fn finish_probe(
        self: &Arc<Self>,
        checks: HashMap<HostTool, ToolCheck>,
        probed_managed_go: Option<&Path>,
    ) {
        let mut report = self.lock_report();
        report.checked = true;
        report.probing = false;
        report.tools = SUPPORT_TOOLS
            .iter()
            .map(|tool| checks[tool].clone())
            .collect();
        report.go_proxy = checks.get(&HostTool::GoProxy).cloned();
        let install_dir = report.install_dir.clone();
        for (row, (_, tools, lsp)) in report.languages.iter_mut().zip(LANGUAGES) {
            row.system = tools.iter().map(|tool| checks[tool].clone()).collect();
            row.lsp = checks.get(&lsp).cloned();
            row.managed = RuntimeRecipe::for_language(row.language)
                .and_then(|recipe| recipe.installed(&install_dir));
        }
        let recheck_proxy = managed_go_path(&report) != probed_managed_go;
        self.publish(&mut report);
        drop(report);
        if recheck_proxy {
            let _ = self.refresh();
        }
    }

    fn lock_report(&self) -> MutexGuard<'_, PreflightReport> {
        self.report
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn publish(&self, report: &mut PreflightReport) -> PreflightReport {
        for row in &mut report.languages {
            row.resolve(report.checked);
        }
        report.installing_count = report
            .languages
            .iter()
            .filter(|row| {
                row.install
                    .as_ref()
                    .is_some_and(InstallProgress::is_running)
            })
            .count();
        report.revision += 1;
        (self.on_update)(report.clone());
        report.clone()
    }
}

fn managed_go_path(report: &PreflightReport) -> Option<&Path> {
    report.languages[language_index(CourseLanguage::Go)]
        .managed
        .as_ref()
        .map(|runtime| runtime.executable.as_path())
}

fn language_index(language: CourseLanguage) -> usize {
    LANGUAGES
        .iter()
        .position(|(candidate, _, _)| *candidate == language)
        .expect("all course languages have a Preflight row")
}

impl LanguagePreflight {
    fn resolve(&mut self, checked: bool) {
        let system = self
            .system
            .iter()
            .find(|tool| tool.status == ProbeStatus::Ok)
            .map(|check| check.tool);
        self.selected = match (system, self.managed.is_some(), self.prefer_managed) {
            (_, true, true) | (None, true, _) => Some(RuntimeSource::Managed),
            (Some(tool), _, _) => Some(RuntimeSource::System { tool }),
            _ => None,
        };
        self.status = if self.selected.is_some() {
            ProbeStatus::Ok
        } else if self
            .system
            .iter()
            .any(|tool| tool.status == ProbeStatus::Warn)
        {
            ProbeStatus::Warn
        } else {
            ProbeStatus::Missing
        };
        self.can_install = self.managed.is_none()
            && (self.prefer_managed || (checked && system.is_none()))
            && !self
                .install
                .as_ref()
                .is_some_and(InstallProgress::is_running);
    }
}
