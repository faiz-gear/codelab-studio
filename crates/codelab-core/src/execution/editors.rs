use crate::workspace::Workspace;
use crate::{DomainError, WorkspaceFile};

use super::model::CommandDraft;

pub(crate) fn save(
    workspace: &Workspace,
    drafts: Vec<CommandDraft>,
    saved: &mut Vec<WorkspaceFile>,
) -> Result<Option<WorkspaceFile>, DomainError> {
    // Detect conflicts across all drafts before writing any of them. The write
    // itself still uses the revision guard in case an external editor races this pass.
    for draft in &drafts {
        let file = workspace.files.read(&draft.path)?;
        if file.revision != draft.expected_revision {
            return Ok(Some(file));
        }
    }
    for draft in drafts {
        match workspace.write_file(
            &draft.path,
            &draft.text,
            draft.expected_revision.as_deref(),
            false,
        )? {
            Ok(file) => saved.push(file),
            Err(file) => return Ok(Some(file)),
        }
    }
    Ok(None)
}
