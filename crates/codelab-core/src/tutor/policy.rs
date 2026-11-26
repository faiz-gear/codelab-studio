use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::execution::risk::is_high_risk;
use crate::workspace::{WorkspacePaths, is_sensitive_path};

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
    workspace_files: &WorkspacePaths,
    policy: PermissionPolicy,
    kind: TutorPermissionKind,
    target_paths: &[String],
    command: Option<&str>,
) -> PermissionEvaluation {
    let canonical_root =
        fs::canonicalize(&workspace_files.root).unwrap_or_else(|_| workspace_files.root.clone());

    let cleaned_targets: Vec<&str> = target_paths
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .collect();

    let mut sensitive = false;
    let mut all_inside_workspace = !cleaned_targets.is_empty();
    let mut first_workspace_rel = None;
    let mut first_resolved_abs = None;

    for raw in &cleaned_targets {
        let single = inspect_target_path(workspace_files, &canonical_root, raw);
        if single.sensitive {
            sensitive = true;
        }
        if !single.inside_workspace {
            all_inside_workspace = false;
        }
        if first_workspace_rel.is_none() {
            first_workspace_rel = single.workspace_relative_path;
        }
        if first_resolved_abs.is_none() {
            first_resolved_abs = single.resolved_absolute_path;
        }
    }

    let outside_workspace = match kind {
        TutorPermissionKind::Execute => false,
        TutorPermissionKind::Read | TutorPermissionKind::Write => !all_inside_workspace,
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
        && all_inside_workspace
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
        workspace_relative_path: if all_inside_workspace {
            first_workspace_rel
        } else {
            None
        },
        resolved_absolute_path: first_resolved_abs,
    }
}

struct TargetInspection {
    inside_workspace: bool,
    sensitive: bool,
    workspace_relative_path: Option<String>,
    resolved_absolute_path: Option<PathBuf>,
}

fn inspect_target_path(
    workspace_files: &WorkspacePaths,
    canonical_root: &Path,
    raw: &str,
) -> TargetInspection {
    let raw_path = Path::new(raw);
    let mut sensitive = path_suffix_after_common_prefix(raw_path, canonical_root)
        .as_deref()
        .is_some_and(is_sensitive_path);

    let candidate = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        canonical_root.join(raw_path)
    };

    let Some(resolved) = resolve_without_lexical_symlink_collapse(&candidate) else {
        return TargetInspection {
            inside_workspace: false,
            sensitive,
            workspace_relative_path: None,
            resolved_absolute_path: None,
        };
    };

    // Inspect only the suffix diverging from canonical_root so an unrelated ancestor
    // directory (e.g. `/Users/mickey/...`) never triggers a false-positive `*key*` block.
    if let Some(suffix) = path_suffix_after_common_prefix(&resolved, canonical_root)
        && is_sensitive_path(&suffix)
    {
        sensitive = true;
    }

    let mut inside_workspace = false;
    let mut workspace_relative_path = None;
    if let Ok(rel) = resolved.strip_prefix(canonical_root) {
        if is_sensitive_path(rel) {
            sensitive = true;
        } else if !has_parent_dir(rel) && workspace_files.validate(rel).is_ok() {
            inside_workspace = true;
            workspace_relative_path = to_wire_relative(rel);
        }
    }

    TargetInspection {
        inside_workspace,
        sensitive,
        workspace_relative_path,
        resolved_absolute_path: Some(resolved),
    }
}

fn path_suffix_after_common_prefix(target: &Path, root: &Path) -> Option<PathBuf> {
    if !target.is_absolute() {
        return Some(target.to_path_buf());
    }
    let mut target_iter = target.components().peekable();
    let mut root_iter = root.components().peekable();
    while let (Some(a), Some(b)) = (target_iter.peek(), root_iter.peek()) {
        if a == b {
            target_iter.next();
            root_iter.next();
        } else {
            break;
        }
    }
    let rest: PathBuf = target_iter.collect();
    if rest.as_os_str().is_empty() {
        target.file_name().map(PathBuf::from)
    } else {
        Some(rest)
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

/// Resolves symlinks component-by-component BEFORE applying `..` so a non-existent
/// path like `outbound_symlink/../missing` can never collapse lexically back inside
/// the workspace root.
fn resolve_without_lexical_symlink_collapse(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical);
    }
    let mut current = PathBuf::new();
    let mut entered_missing = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                // Never lexically collapse `..` past a non-existent segment.
                if entered_missing {
                    return None;
                }
                current = fs::canonicalize(&current).ok()?;
                if !current.pop() {
                    return None;
                }
            }
            Component::Normal(part) => {
                current.push(part);
                if !entered_missing {
                    match fs::canonicalize(&current) {
                        Ok(canonical) => current = canonical,
                        Err(_) => entered_missing = true,
                    }
                }
            }
        }
    }
    if current.as_os_str().is_empty() {
        None
    } else {
        Some(current)
    }
}
