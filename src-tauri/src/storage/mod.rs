pub mod atomic_json;
pub mod global_store;
pub mod metrics_writer;
pub mod workspace_store;

pub use atomic_json::{
    cleanup_stale_tmp_files, AtomicJsonFile, AtomicJsonStore, LoadOutcome, StorageError,
};
pub use global_store::{
    AcpAgentEntry, AiGlobalConfig, AiProtocolKind, AppearanceConfig, ByokProfileRef, EditorConfig,
    GlobalConfigFile, GlobalStore, PrivacyConfig, TrustedPackEntry, TrustedPacksFile,
    MAX_RECENT_WORKSPACES,
};
pub use metrics_writer::{
    metric_event_to_record, MetricsShutdownStats, MetricsWriter, MetricsWriterConfig,
    MetricsWriterHandle, DEFAULT_METRICS_BATCH_SIZE, DEFAULT_METRICS_CHANNEL_CAPACITY,
    DEFAULT_METRICS_FLUSH_INTERVAL, MAX_METRICS_EVENTS,
};
pub use workspace_store::{
    WorkspaceInitSpec, WorkspaceOpenReport, WorkspaceStore, MAX_CHAT_MESSAGES_PER_CHAPTER,
    MAX_CHAT_TURNS_PER_CHAPTER,
};
