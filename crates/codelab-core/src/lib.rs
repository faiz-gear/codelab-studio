mod app;
mod course;
mod error;
mod execution;
mod filesystem;
mod lsp;
mod model;
mod runtime;
mod storage;
mod tutor;
mod workspace;

pub use app::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, CleanupHook, CourseTrustChanged,
    EventSubscription, RecentWorkspaceOpened,
};
pub use course::authoring::{
    CourseDraft, CourseDraftRequest, DraftDiagnostic, DraftEdit, DraftFilePath, DraftFileWrite,
    DraftPreview, DraftPreviewRequest, TrialWorkspaceOpened,
};
pub use course::{
    Chapter, ChapterKind, ChapterPresentation, CheckKind, CommandBlock, CommandMode, ContentBlock,
    CourseCheckSpec, CourseSource, CourseSourceChangedEvent, CourseSummary, DaemonRequirement,
    GitFetchAdapter, HttpExpectation, LoadedCourse, MAX_MARKDOWN_BYTES, OpenCourseRequest,
    ThreeStepProjection, ValueOrigin,
};
pub use error::{DomainError, ErrorCode};
pub use execution::model::*;
pub use execution::verification::model::*;
pub use lsp::{
    CompletionTextEdit, DiagnosticCode, LanguageCompletion, LanguageCompletionItem,
    LanguageDiagnostic, LanguageDiagnostics, LanguageDocument, LanguageDocumentUpdate,
    LanguageDocumentVersion, LanguageFeature, LanguageFeatureRequest, LanguageFeatureResult,
    LanguageHover, LanguageMarkup, LanguagePosition, LanguageRange, LanguageStatus,
    LanguageStatusChanged, LanguageTextEdit,
};
pub use model::{
    AccentPreset, AppBootstrap, CourseLanguage, DEFAULT_CODE_FONT_SIZE, GLOBAL_DATA_SCHEMA_VERSION,
    GlobalSettings, Locale, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE, RecentWorkspace,
    RecordRecentWorkspaceInput, ShutdownReport, StorageRecoveryNotice, StoreKind, ThemeMode,
    TrustEntry, TrustSource, TrustSourceKind, TrustState, UpdateSettingsPatch,
};
pub use runtime::{
    HostProbeResult, HostTool, HostToolchainProbe, HttpRuntimeDownloader, InstallProgress,
    LanguagePreflight, ManagedRuntime, PreflightReport, ProbeProblem, ProbeStatus, RuntimeAdapters,
    RuntimeArtifact, RuntimeDownloadAdapter, RuntimePreference, RuntimeSource, ToolCheck,
};
pub use tutor::{
    AcpAgentPreset, AcpStdioBackend, BackendEvent, BackendEventSink, BackendPermissionPrompt,
    ContextChip, ContextDimension, ContextQuote, ContextQuoteSource, ContextSnapshot,
    ContextSnapshotInput, MAX_CONTEXT_INPUT_TOKENS, PermissionPolicy, RespondTutorPermission,
    SendTutorPrompt, TeachingStyle, TutorBackend, TutorBackendSession, TutorFileChange,
    TutorHandshakeRequest, TutorHandshakeResult, TutorHistory, TutorLaunchSpec, TutorMessage,
    TutorMessageStatus, TutorPermissionKind, TutorPermissionRequest, TutorPermissionStatus,
    TutorRole, TutorSessionSnapshot, TutorSessionStatus, TutorSettings, TutorSettingsPatch,
    TutorTurnInput, WorkspaceTutorRequest,
};
pub use workspace::{
    ExternalIde, MAX_WORKSPACE_TEXT_BYTES, WorkspaceComparison, WorkspaceEntry, WorkspaceEntryKind,
    WorkspaceFile, WorkspaceFileContents, WorkspaceFileRename, WorkspaceFileWrite,
    WorkspaceFilesChanged, WorkspaceIdeRequest, WorkspacePath, WorkspaceSummary,
};
