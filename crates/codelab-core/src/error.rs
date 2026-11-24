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
    StorageReadFailed,
    StorageWriteFailed,
    StorageCorrupted,
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
