use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::execution::risk::is_high_risk;
use crate::workspace::is_sensitive_path;

use super::model::{PermissionPolicy, TutorPermissionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyDecision {
    AutoAllow,
    AskUser,
    BlockSensitive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionEvaluation {
    pub decision: PolicyDecision,
    pub outside_workspace: bool,
    pub sensitive: bool,
    pub high_risk: bool,
    pub workspace_relative_path: Option<String>,
    pub resolved_absolute_path: Option<PathBuf>,
}

pub(crate) fn evaluate_permission(
    workspace_root: &Path,
    policy: PermissionPolicy,
    kind: TutorPermissionKind,
    target_path: Option<&str>,
    command: Option<&str>,
) -> PermissionEvaluation {
    let canonical_root =
        fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());

    let mut sensitive = false;
    let mut inside_workspace = false;
    let mut workspace_relative_path = None;
    let mut resolved_absolute_path = None;

    if let Some(raw) = target_path.map(str::trim).filter(|raw| !raw.is_empty()) {
        let raw_path = Path::new(raw);
        if is_sensitive_path(raw_path) {
            sensitive = true;
        }
        let candidate = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            canonical_root.join(raw_path)
        };
        if let Some(resolved) = resolve_canonical_or_ancestor(&candidate) {
            if is_sensitive_path(&resolved) {
                sensitive = true;
            }
            if let Ok(rel) = resolved.strip_prefix(&canonical_root) {
                if !is_sensitive_path(rel) && !has_parent_dir(rel) {
                    inside_workspace = true;
                    workspace_relative_path = to_wire_relative(rel);
                } else {
                    sensitive = sensitive || is_sensitive_path(rel);
                }
            }
            resolved_absolute_path = Some(resolved);
        } else {
            // Unresolvable path: keep conservative outside_workspace = true.
            if is_sensitive_path(&candidate) {
                sensitive = true;
            }
        }
    }

    let outside_workspace = match kind {
        TutorPermissionKind::Execute => false,
        TutorPermissionKind::Read | TutorPermissionKind::Write => !inside_workspace,
    };

    let high_risk = match kind {
        TutorPermissionKind::Execute => command
            .map(str::trim)
            .filter(|cmd| !cmd.is_empty())
            .is_none_or(is_high_risk),
        TutorPermissionKind::Read | TutorPermissionKind::Write => false,
    };

    let decision = if sensitive {
        PolicyDecision::BlockSensitive
    } else if policy == PermissionPolicy::GuidedReadonly
        && kind == TutorPermissionKind::Read
        && inside_workspace
    {
        PolicyDecision::AutoAllow
    } else {
        PolicyDecision::AskUser
    };

    PermissionEvaluation {
        decision,
        outside_workspace,
        sensitive,
        high_risk,
        workspace_relative_path,
        resolved_absolute_path,
    }
}

fn has_parent_dir(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn to_wire_relative(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

fn resolve_canonical_or_ancestor(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical);
    }
    // For not-yet-created files (e.g. Write targets), normalize lexical components
    // first so `..` segments are resolved, then canonicalize the deepest existing ancestor.
    let normalized = normalize_lexical(path)?;
    let mut tail = Vec::new();
    let mut cursor = normalized.as_path();
    loop {
        if let Ok(canonical) = fs::canonicalize(cursor) {
            let mut full = canonical;
            for part in tail.into_iter().rev() {
                full.push(part);
            }
            return Some(full);
        }
        let name = cursor.file_name()?;
        tail.push(name.to_os_string());
        cursor = cursor.parent()?;
    }
}

fn normalize_lexical(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}
