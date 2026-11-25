use std::path::{Path, PathBuf};

use crate::workspace::Workspace;
use crate::{DomainError, WorkspaceFile};

use super::model::{CommandConfirmation, CommandDraft, CommandFailed, CommandSaveConflict};
use super::{CommandLaunch, Execution, risk};

pub(crate) enum CommandPreparation {
    ConfirmationRequired(CommandConfirmation),
    SaveConflict(CommandSaveConflict),
    Failed(CommandFailed),
    Ready {
        target: ConfirmationTarget,
        saved: Vec<WorkspaceFile>,
    },
}

#[derive(PartialEq, Eq)]
pub(crate) enum ConfirmationTarget {
    Block(Box<CommandLaunch>),
    Checks {
        chapter_id: String,
        workspace_root: PathBuf,
        commands: Vec<CommandInvocation>,
    },
}

#[derive(PartialEq, Eq)]
pub(crate) struct CommandInvocation {
    script: String,
    cwd: PathBuf,
    shell: String,
    source_script: Option<String>,
}

impl CommandInvocation {
    pub fn new(script: &str, cwd: &Path, shell: &str) -> Self {
        Self {
            script: script.into(),
            cwd: cwd.into(),
            shell: shell.into(),
            source_script: None,
        }
    }

    pub fn from_block(launch: &CommandLaunch) -> Self {
        let mut invocation = Self::new(&launch.script, &launch.cwd, &launch.block.shell);
        if launch.script != launch.block.command {
            invocation.source_script = Some(launch.block.command.clone());
        }
        invocation
    }

    fn display_script(&self) -> &str {
        self.source_script.as_deref().unwrap_or(&self.script)
    }

    fn high_risk(&self) -> bool {
        high_risk(&self.script, self.display_script())
    }
}

impl ConfirmationTarget {
    fn high_risk(&self) -> bool {
        match self {
            Self::Block(launch) => high_risk(&launch.script, &launch.block.command),
            Self::Checks { commands, .. } => commands.iter().any(CommandInvocation::high_risk),
        }
    }

    fn prompt(&self, token: u64) -> CommandConfirmation {
        let (cwd, command) = match self {
            Self::Block(launch) => (launch.cwd.as_path(), launch.block.command.clone()),
            Self::Checks {
                workspace_root,
                commands,
                ..
            } => (
                workspace_root.as_path(),
                commands
                    .iter()
                    .map(CommandInvocation::display_script)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        };
        CommandConfirmation {
            token,
            cwd: super::label(cwd),
            command,
        }
    }
}

impl Execution {
    pub fn prepare_command(
        &mut self,
        workspace: &Workspace,
        target: ConfirmationTarget,
        confirmation: Option<u64>,
        drafts: Vec<CommandDraft>,
    ) -> CommandPreparation {
        let pending = self.confirmation.take();
        let authorized = pending
            .is_some_and(|(token, expected)| confirmation == Some(token) && expected == target)
            || (confirmation.is_none() && !target.high_risk());
        if !authorized {
            self.next_id += 1;
            let prompt = target.prompt(self.next_id);
            self.confirmation = Some((prompt.token, target));
            return CommandPreparation::ConfirmationRequired(prompt);
        }

        let mut saved = Vec::new();
        match save_drafts(workspace, drafts, &mut saved) {
            Ok(Some(file)) => CommandPreparation::SaveConflict(CommandSaveConflict { file, saved }),
            Ok(None) => CommandPreparation::Ready { target, saved },
            Err(error) => CommandPreparation::Failed(CommandFailed { error, saved }),
        }
    }
}

fn high_risk(script: &str, source: &str) -> bool {
    risk::is_high_risk(script) || (source != script && risk::is_high_risk(source))
}

fn save_drafts(
    workspace: &Workspace,
    drafts: Vec<CommandDraft>,
    saved: &mut Vec<WorkspaceFile>,
) -> Result<Option<WorkspaceFile>, DomainError> {
    // Check every revision before writing, then retain successful writes if a later one fails.
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
