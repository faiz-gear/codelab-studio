use std::fmt;
use std::path::{Path, PathBuf};

use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject, StringValidation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    TS,
)]
pub struct SessionEpoch(#[ts(type = "number")] pub u64);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct ChapterId(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct TerminalId(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct CommandBlockId(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct RunId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
pub struct RiskConfirmationToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RelWorkspacePathError {
    #[error("relative workspace path must not be empty")]
    Empty,
    #[error("relative workspace path must not contain NUL bytes")]
    NulByte,
    #[error("relative workspace path must use POSIX '/' separators, not '\\\\'")]
    Backslash,
    #[error("relative workspace path must not start with '/'")]
    LeadingSlash,
    #[error("relative workspace path must not end with '/'")]
    TrailingSlash,
    #[error("relative workspace path must not contain empty path segments")]
    EmptySegment,
    #[error("relative workspace path must not contain '.' or '..' segments")]
    DotSegment,
    #[error("relative workspace path segment '{0}' is a reserved Windows device name")]
    ReservedWindowsName(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
pub struct RelWorkspacePath(#[ts(type = "string")] String);

impl RelWorkspacePath {
    pub fn new(raw: impl Into<String>) -> Result<Self, RelWorkspacePathError> {
        let value = raw.into();
        validate_rel_workspace_path(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for RelWorkspacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RelWorkspacePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RelWorkspacePath {
    type Error = RelWorkspacePathError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RelWorkspacePath> for String {
    fn from(value: RelWorkspacePath) -> Self {
        value.0
    }
}

impl JsonSchema for RelWorkspacePath {
    fn schema_name() -> String {
        "RelWorkspacePath".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            string: Some(Box::new(StringValidation {
                min_length: Some(1),
                pattern: Some(
                    r"^(?!/)(?!.*/$)(?!.*\\)(?!.*//)(?!\.{1,2}(?:/|$))(?!.*/\.{1,2}(?:/|$)).+$"
                        .to_owned(),
                ),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

fn validate_rel_workspace_path(value: &str) -> Result<(), RelWorkspacePathError> {
    if value.is_empty() {
        return Err(RelWorkspacePathError::Empty);
    }
    if value.contains('\0') {
        return Err(RelWorkspacePathError::NulByte);
    }
    if value.contains('\\') {
        return Err(RelWorkspacePathError::Backslash);
    }
    if value.starts_with('/') {
        return Err(RelWorkspacePathError::LeadingSlash);
    }
    if value.ends_with('/') {
        return Err(RelWorkspacePathError::TrailingSlash);
    }

    for segment in value.split('/') {
        if segment.is_empty() {
            return Err(RelWorkspacePathError::EmptySegment);
        }
        if segment == "." || segment == ".." {
            return Err(RelWorkspacePathError::DotSegment);
        }
        let stem = segment.split('.').next().unwrap_or("").trim_end();
        if is_windows_reserved_name(stem) {
            return Err(RelWorkspacePathError::ReservedWindowsName(
                segment.to_owned(),
            ));
        }
    }

    Ok(())
}

fn is_windows_reserved_name(stem: &str) -> bool {
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// Verified absolute path anchored inside the active workspace root.
///
/// Must not implement `Deserialize` so IPC callers cannot forge canonical paths:
///
/// ```compile_fail
/// use codelab_studio_lib::domain::ids::CanonicalWorkspacePath;
/// let _: CanonicalWorkspacePath = serde_json::from_str(r#""task-api/main.go""#).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalWorkspacePath {
    abs: PathBuf,
    rel: RelWorkspacePath,
}

impl CanonicalWorkspacePath {
    #[allow(dead_code)]
    pub(crate) fn from_verified_parts(abs: PathBuf, rel: RelWorkspacePath) -> Self {
        Self { abs, rel }
    }

    pub fn abs(&self) -> &Path {
        &self.abs
    }

    pub fn rel(&self) -> &RelWorkspacePath {
        &self.rel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_workspace_path_accepts_valid_paths() {
        let valid = [
            "README.md",
            "task-api/main.go",
            "task-api/internal/handler.go",
            ".codelab-checks.json",
            "src/console.ts",
            "src/nullable.rs",
            "src/com10.txt",
        ];

        for raw in valid {
            let parsed: RelWorkspacePath =
                serde_json::from_str(&serde_json::to_string(raw).unwrap()).unwrap();
            assert_eq!(parsed.as_str(), raw);
        }
    }

    #[test]
    fn rel_workspace_path_rejects_unsafe_paths() {
        let invalid = [
            "",
            ".",
            "..",
            "../",
            "../secret.txt",
            "a/../b",
            "a/./b",
            "/etc/passwd",
            "a/b/",
            "a//b",
            "a\\b",
            "a\0b",
            "CON",
            "PRN",
            "AUX",
            "NUL",
            "COM1",
            "COM9",
            "LPT1",
            "LPT9",
            "con.txt",
            "dir/NUL.json",
            "dir/Com1.rs",
            "dir/lpt9.tar.gz",
        ];

        for raw in invalid {
            let json = serde_json::to_string(raw).unwrap();
            let err = serde_json::from_str::<RelWorkspacePath>(&json);
            assert!(err.is_err(), "expected '{raw:?}' to be rejected");
        }
    }

    #[test]
    fn canonical_workspace_path_exposes_verified_parts() {
        let rel = RelWorkspacePath::new("task-api/main.go").unwrap();
        let canonical = CanonicalWorkspacePath::from_verified_parts(
            PathBuf::from("/workspace/task-api/main.go"),
            rel.clone(),
        );
        assert_eq!(canonical.abs(), Path::new("/workspace/task-api/main.go"));
        assert_eq!(canonical.rel(), &rel);
    }
}
