# Save Starter source and reopen it after restart

Bind an empty Workspace, edit the built-in Starter in Monaco and verify the saved text both on disk and after restarting CodeLab Studio.

## Sub-features

- Materialize the Starter in a folder selected through the native picker.
- Open and save `task-api/main.go` in Monaco.
- Reopen the same Workspace after a full app exit and inspect the same source file.

## How to get to it (user POV)

Open the [built-in course](builtin-course.md) from the welcome page, then choose a Workspace in its workbench. Open `main.go` under `task-api` in **文件** (**Files**). After restarting, open the same folder from the welcome page's recent Workspaces.

## Driving it with the desktop window

1. Complete [Launch and Doctor](../SKILL.md) and the [built-in course steps](builtin-course.md). Keep the launch terminal open. Use a second shell in the repo root for readback commands, with `VERIFY_TMP` and `VERIFY_EVIDENCE` set to the values printed by setup. Run the following and copy the printed folder path for the picker.

   ```sh
   VERIFY_WORKSPACE=$(cd "$VERIFY_TMP/workspace" && pwd -P)
   VERIFY_FILE="$VERIFY_WORKSPACE/task-api/main.go"
   VERIFY_LINE="package main // codelab-verify-${VERIFY_TMP##*.}"
   test -z "$(ls -A "$VERIFY_WORKSPACE")" &&
   printf 'folder=%s\nexpected=%s\n' "$VERIFY_WORKSPACE" "$VERIFY_LINE"
   ```

2. Click **选择 Workspace** (**Choose Workspace**). In the native folder picker, navigate to the printed folder and select it. On macOS, press `⌘⇧G`, enter the absolute path, press Return to go there, then confirm the selection in the picker. Capture `03-picker.png` with the temporary folder selected before confirming. Check that the workbench displays the same Workspace path after binding.
3. Click **文件** (**Files**), expand **task-api** and click **main.go**. Confirm the status path reads `task-api/main.go` and Monaco line 1 reads `package main`. Capture `04-starter.png`. Establish the disk baseline before typing.

   ```sh
   if cmp -s builtin-courses/go-goframe-practice/starter/task-api/main.go "$VERIFY_FILE"; then
     printf 'PASS Starter matches checkout before edit\n'
   else
     printf 'FAIL Starter missing or different before edit\n'
     false
   fi > "$VERIFY_EVIDENCE/file-assertions.log" 2>&1
   cat "$VERIFY_EVIDENCE/file-assertions.log"
   ```

4. In Monaco, place the caret at the end of line 1 (`⌘→` on macOS or End on Linux/Windows), then type the suffix of `VERIFY_LINE`, including the space before `//`. Confirm the editor shows **有未保存的修改** (**Unsaved changes**) and the new comment. Capture `05-edited.png`. Use **保存** (**Save**) or `⌘S` / `Ctrl+S`, wait for **所有修改已保存** (**All changes saved**), then capture `06-saved.png`. Run the disk assertion from the second shell and require a PASS.

   ```sh
   verify_saved_file() {
     if rg -nxF -- "$VERIFY_LINE" "$VERIFY_FILE"; then
       printf 'PASS %s %s\n' "$1" "$VERIFY_FILE"
     else
       printf 'FAIL %s expected line %s in %s\n' "$1" "$VERIFY_LINE" "$VERIFY_FILE"
       return 1
     fi
   }
   verify_saved_file after-save >> "$VERIFY_EVIDENCE/file-assertions.log" 2>&1
   cat "$VERIFY_EVIDENCE/file-assertions.log"
   ```

5. Quit the app and confirm the first app and listener have exited using [Cleanup](../SKILL.md#cleanup). Keep both scratch directories at this point. In the launch shell, run `launch_verify 2`, or repeat the launch command with the same `CODELAB_DATA_DIR` and `launch-2.log`. Run Doctor again and record the new PID. Follow [Resume a Workspace](recent-workspace.md) to reopen the file and capture the editor after restart. Assert the file on disk again.

   ```sh
   verify_saved_file after-restart >> "$VERIFY_EVIDENCE/file-assertions.log" 2>&1
   cat "$VERIFY_EVIDENCE/file-assertions.log"
   ```

6. Require both disk assertions and the marker visibly present in Monaco after restart. Record the result in `run.txt`. Complete [Cleanup](../SKILL.md#cleanup) and verify the evidence survives.

## Gotchas

- The Starter has two `main.go` files. Use the `task-api` branch of **Files** and verify the path in the status bar.
- If the file tree clips the code, toggle **文件** (**Files**) closed after opening `main.go` so the first line fits in the editor screenshot.
- Start from an empty directory so the app materializes the Starter. The shell checks read the source independently of the editor. Type the marker in Monaco so the app is the only writer during the check.
- Check that the first PID exited and the new launch used the same data directory.
