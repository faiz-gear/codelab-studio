use crate::workspace::Workspace;
use crate::{DomainError, WorkspaceFile};

use super::model::{CommandDraft, CommandSaveConflict};

pub(crate) fn save(
    workspace: &Workspace,
    drafts: Vec<CommandDraft>,
) -> Result<Result<Vec<WorkspaceFile>, CommandSaveConflict>, DomainError> {
    let mut saved = Vec::new();
    // Detect conflicts across all drafts before writing any of them. The write
    // itself still uses the revision guard in case an external editor races this pass.
    for draft in &drafts {
        let file = workspace.files.read(&draft.path)?;
        if file.revision != draft.expected_revision {
            return Ok(Err(CommandSaveConflict { file, saved }));
        }
    }
    for draft in drafts {
        match workspace.files.write(
            &draft.path,
            &draft.text,
            draft.expected_revision.as_deref(),
            false,
        )? {
            Ok(file) => saved.push(file),
            Err(file) => return Ok(Err(CommandSaveConflict { file, saved })),
        }
    }
    Ok(Ok(saved))
}
