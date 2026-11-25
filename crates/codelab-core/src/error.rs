use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidCodeFontSize,
    InvalidWorkspacePath,
    InvalidCourseMetadata,
    InvalidProgressCount,
    InvalidTrustSource,
    CourseNotFound,
    CoursePathInvalid,
    CourseMarkdownTooLarge,
    CourseMarkdownInvalid,
    CourseManifestInvalid,
    CourseChecksInvalid,
    GitCourseFetchFailed,
    GitCourseCacheFailed,
    NoActiveGitCourse,
    NoActiveDraft,
    InvalidDraftPath,
    DraftDirectoryNotEmpty,
    DraftWriteFailed,
    TrialWorkspaceFailed,
    NoActiveCourse,
    CourseUntrusted,
    NoActiveWorkspace,
    WorkspacePathOutsideRoot,
    WorkspaceBoundToAnotherCourse,
    WorkspaceFileNotFound,
    WorkspaceFileExists,
    WorkspaceFileReadFailed,
    WorkspaceFileWriteFailed,
    WorkspaceFileNotText,
    WorkspaceWatchFailed,
    WorkspaceStarterFailed,
    ExternalIdeOpenFailed,
    StorageReadFailed,
    StorageWriteFailed,
    StorageCorrupted,
    RuntimeTaskStartFailed,
    RuntimePlatformUnsupported,
    RuntimeDownloadFailed,
    RuntimeIntegrityFailed,
    RuntimeInstallFailed,
    ExecutionFailed,
    VerificationBusy,
    CheckNotFound,
    ExerciseCompletionBlocked,
    DaemonPortConflict,
    DaemonNotReady,
    CommandBlockNotFound,
    CommandCwdInvalid,
    CommandShellUnavailable,
    TerminalNotFound,
    ShutdownFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code:?}")]
#[serde(rename_all = "camelCase")]
pub struct DomainError {
    pub code: ErrorCode,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl DomainError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            details: BTreeMap::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}
