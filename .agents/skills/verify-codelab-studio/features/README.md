# CodeLab Studio desktop verification map

Start from the [launch and doctor instructions](../SKILL.md). Use fresh app data and a separate empty local Workspace for each run. Keep both directories for the restart within that run.

| Feature | Entry point | Proof |
| --- | --- | --- |
| [Built-in courses](builtin-course.md) | Welcome page and Course menu | Go opens from three entry points. The JS/TS, Python, and Java filters open their matching manuals. |
| [Workspace and Monaco save](workspace-save.md) | Native folder picker in the workbench | The Starter opens in Monaco, the editor shows the saved state, and the file on disk contains the run marker across restart. |
| [Resume a Workspace](recent-workspace.md) | Welcome page after restart | The same Workspace opens and Monaco displays the saved marker. |
| [Reference Solution and Starter restore](reference-solution.md) | Exercise 1 and the Workspace editor toolbar | The review leaves disk unchanged, apply writes the solution, and restore brings back the Starter. |
| [Command execution](command-execution.md) | Exercise 1 and an authored Trial Workspace | A Managed Daemon serves the exercise and stops. A guarded command waits for confirmation before producing terminal output. |
| [Exercise Checks and progress](exercise-checks.md) | Exercise 1 in the built-in courses | Go Checks start the daemon. JS/TS, Python, and Java show per-case results and complete after applying the Reference Solution. |
| [Module Root language servers](language-server.md) | Monaco status bar | Go completions, hover, and diagnostics follow the active module. A missing server leaves Monaco editable. |
| [ACP Tutor and Context Snapshot](acp-tutor.md) | Settings, selections, and workbench Ask AI buttons | The local agent handshakes. Ask AI quotes the selected context and removing a chip reduces the token estimate. |
| [Runtime Preflight and Managed Runtime](runtime-preflight.md) | Top-bar Preflight and Settings | Recheck shows host runtime status. Installation reports progress and writes into the isolated app data directory. |
| [Settings and local data](settings-local-data.md) | Top-bar Settings | Four tabs respond to keyboard navigation. Editor preferences persist and local metrics match the Workspace file. |
| [Git Course Pack and Workspace files](git-course-pack.md) | Course import modal, Git repository mode | Read-only preview precedes trust. Refresh preserves the bound Workspace, file actions update disk, and revocation returns to preview. |
| [Authored Course Pack trial](authored-trial.md) | Welcome page, create a new Course Pack | Trial saves the draft, starts a temporary Workspace and can return to the source editor. |

Keep the save marker until restart proof, then review and restore the Starter. Run Go commands, Checks, language server and Tutor checks in that Workspace. Check runtime prerequisites before binding the JS/TS, Python, and Java courses. Use the TypeScript failure for the Tutor Check Results actions. Run the generated command in the authored trial before reusing the draft for command confirmation. Use separate scratch directories for each Course Pack Workspace and for the draft. Record the entry points and the last observed step for any blocked route.
