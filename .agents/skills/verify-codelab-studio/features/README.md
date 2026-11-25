# CodeLab Studio desktop verification map

Start from the [launch and doctor instructions](../SKILL.md). Use fresh app data and a separate empty local Workspace for each run. Keep both directories for the restart within that run.

| Feature | Entry point | Proof |
| --- | --- | --- |
| [Built-in Go / GoFrame course](builtin-course.md) | Welcome page | The built-in manual opens and offers Workspace selection. |
| [Workspace and Monaco save](workspace-save.md) | Native folder picker in the workbench | The Starter opens in Monaco, the editor shows the saved state, and the actual file contains the run marker. This recipe covers the complete desktop journey. |
| [Resume a Workspace](recent-workspace.md) | Welcome page after restart | The same Workspace opens and Monaco displays the saved marker. |
| [Reference Solution and Starter restore](reference-solution.md) | Exercise 1 and the Workspace editor toolbar | The review leaves disk unchanged, apply writes the solution, and restore brings back the Starter. |
| [Runtime Preflight and Managed Runtime](runtime-preflight.md) | Top-bar Preflight and Settings | Recheck shows host runtime status. Installing an absent runtime reports progress and writes it under the isolated app data directory. |
| [Command execution](command-execution.md) | Exercise 1 and an authored Trial Workspace | A Managed Daemon serves the exercise and stops. A guarded command waits for confirmation before producing terminal output. |
| [Git Course Pack and Workspace files](git-course-pack.md) | Course import modal, Git repository mode | Read-only preview precedes trust. Refresh preserves the bound Workspace, file actions update disk, and revocation returns to preview. |
| [Authored Course Pack trial](authored-trial.md) | Welcome page, create a new Course Pack | Trial saves the draft, starts a temporary Workspace and can return to the source editor. |

Keep the save marker until restart proof, then run Reference Solution and Starter restore. Use separate scratch directories for Git import and authoring. The guarded command check can share the authored draft. Prefer the desktop driver's accessible names where available. Record the entry points you used and the last observed step for any blocked route.
