use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{CourseLanguage, DomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostTool {
    Bun,
    Node,
    Go,
    Javac,
    Python3,
    Python,
    Uv,
    UvPython,
    Git,
    Curl,
    TypeScriptLanguageServer,
    Gopls,
    Jdtls,
    Pyright,
    GoProxy,
}

impl HostTool {
    pub fn command(self) -> &'static str {
        match self {
            Self::Bun => "bun",
            Self::Node => "node",
            Self::Go | Self::GoProxy => "go",
            Self::Javac => "javac",
            Self::Python3 => "python3",
            Self::Python => "python",
            Self::Uv | Self::UvPython => "uv",
            Self::Git => "git",
            Self::Curl => "curl",
            Self::TypeScriptLanguageServer => "typescript-language-server",
            Self::Gopls => "gopls",
            Self::Jdtls => "jdtls",
            Self::Pyright => "pyright-langserver",
        }
    }

    pub(super) fn is_lsp(self) -> bool {
        matches!(
            self,
            Self::TypeScriptLanguageServer | Self::Gopls | Self::Jdtls | Self::Pyright
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus {
    Ok,
    Warn,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeProblem {
    VersionTooOld,
    TimedOut,
    Failed,
    ProxyDisabled,
    InvalidProxy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCheck {
    pub tool: HostTool,
    pub status: ProbeStatus,
    pub value: Option<String>,
    pub executable: Option<PathBuf>,
    pub problem: Option<ProbeProblem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum RuntimeSource {
    System { tool: HostTool },
    Managed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePreference {
    pub language: CourseLanguage,
    pub prefer_managed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRuntime {
    pub version: String,
    pub executable: PathBuf,
    pub test_executor: PathBuf,
    pub uv: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "phase",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InstallProgress {
    Preparing,
    Downloading {
        artifact: String,
        received_bytes: u64,
        total_bytes: Option<u64>,
    },
    Extracting {
        artifact: String,
    },
    Installed,
    Failed {
        error: DomainError,
    },
}

impl InstallProgress {
    pub(super) fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Downloading { .. } | Self::Extracting { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePreflight {
    pub language: CourseLanguage,
    pub status: ProbeStatus,
    pub system: Vec<ToolCheck>,
    pub lsp: Option<ToolCheck>,
    pub managed: Option<ManagedRuntime>,
    pub selected: Option<RuntimeSource>,
    pub prefer_managed: bool,
    pub can_install: bool,
    pub install: Option<InstallProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub revision: u64,
    pub checked: bool,
    pub probing: bool,
    pub installing_count: usize,
    pub languages: Vec<LanguagePreflight>,
    pub tools: Vec<ToolCheck>,
    pub go_proxy: Option<ToolCheck>,
    pub install_dir: PathBuf,
}
