use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    WorkspaceNotOpen,
    SessionEpochStale,
    PathOutsideSandbox,
    FileTooLargeOrBinary,
    ExternalEditConflict,
    ExternalPackUntrusted,
    RiskConfirmationRequired,
    RiskTokenInvalidOrExpired,
    MarkdownLimitExceeded,
    SchemaValidationFailed,
    RuntimeMissing,
    RuntimeInstallFailed,
    RuntimeChecksumMismatch,
    PtySpawnFailed,
    DaemonNotRunning,
    GradeTimeout,
    LspUnavailable,
    KeychainUnavailable,
    AiProviderError,
    AcpHandshakeFailed,
    AcpPermissionDenied,
    StorageCorruptedRecovered,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS, Error)]
#[serde(rename_all = "camelCase")]
#[error("{code:?}: {message_key}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message_key: String,
    pub params: BTreeMap<String, String>,
    pub detail: Option<String>,
    pub retryable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_has_all_23_variants_in_screaming_snake_case() {
        let all = [
            (ErrorCode::WorkspaceNotOpen, "WORKSPACE_NOT_OPEN"),
            (ErrorCode::SessionEpochStale, "SESSION_EPOCH_STALE"),
            (ErrorCode::PathOutsideSandbox, "PATH_OUTSIDE_SANDBOX"),
            (ErrorCode::FileTooLargeOrBinary, "FILE_TOO_LARGE_OR_BINARY"),
            (ErrorCode::ExternalEditConflict, "EXTERNAL_EDIT_CONFLICT"),
            (ErrorCode::ExternalPackUntrusted, "EXTERNAL_PACK_UNTRUSTED"),
            (
                ErrorCode::RiskConfirmationRequired,
                "RISK_CONFIRMATION_REQUIRED",
            ),
            (
                ErrorCode::RiskTokenInvalidOrExpired,
                "RISK_TOKEN_INVALID_OR_EXPIRED",
            ),
            (ErrorCode::MarkdownLimitExceeded, "MARKDOWN_LIMIT_EXCEEDED"),
            (
                ErrorCode::SchemaValidationFailed,
                "SCHEMA_VALIDATION_FAILED",
            ),
            (ErrorCode::RuntimeMissing, "RUNTIME_MISSING"),
            (ErrorCode::RuntimeInstallFailed, "RUNTIME_INSTALL_FAILED"),
            (
                ErrorCode::RuntimeChecksumMismatch,
                "RUNTIME_CHECKSUM_MISMATCH",
            ),
            (ErrorCode::PtySpawnFailed, "PTY_SPAWN_FAILED"),
            (ErrorCode::DaemonNotRunning, "DAEMON_NOT_RUNNING"),
            (ErrorCode::GradeTimeout, "GRADE_TIMEOUT"),
            (ErrorCode::LspUnavailable, "LSP_UNAVAILABLE"),
            (ErrorCode::KeychainUnavailable, "KEYCHAIN_UNAVAILABLE"),
            (ErrorCode::AiProviderError, "AI_PROVIDER_ERROR"),
            (ErrorCode::AcpHandshakeFailed, "ACP_HANDSHAKE_FAILED"),
            (ErrorCode::AcpPermissionDenied, "ACP_PERMISSION_DENIED"),
            (
                ErrorCode::StorageCorruptedRecovered,
                "STORAGE_CORRUPTED_RECOVERED",
            ),
            (ErrorCode::Cancelled, "CANCELLED"),
        ];

        assert_eq!(all.len(), 23);
        for (variant, expected) in all {
            assert_eq!(serde_json::to_value(variant).unwrap(), expected);
        }
    }
}
