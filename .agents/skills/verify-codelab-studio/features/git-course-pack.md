# Import a Git Course Pack and work in its Workspace

Use a test Git repository with a valid `codelab.json` at its root and a Starter. The [loopback fixture](#loopback-fixture) uses a disposable copy of the built-in Go / GoFrame pack.

1. Click **导入本地 Markdown / Course Pack** on the welcome page or **导入本地 Markdown / Git 或创作课程包…** in the Course menu. Select **Git 仓库**, enter the fixture URL and click **打开只读手册**. Capture the **Git Course Pack** and **只读预览** badges and normalized source URL. The displayed URL drops the `.git` suffix.
2. Click **信任此课程** and verify the normalized URL in the trust dialog before granting trust. Choose the empty Workspace through the native picker. Confirm the bound path and compare `task-api/main.go` against the fixture's Starter.
3. Change the fixture's Exercise 0 title in `codelab.json`, keeping the chapter ID and matcher, and commit the change. Click **拉取上游更新**. Capture the new title and the same bound Workspace path. Compare the Starter again and retain the server log.
4. In the editor, use **新建文件** to create `verification-note.txt` and **重命名文件** to rename it to `verification-renamed.txt`. Confirm the new path exists and the old one is gone. Capture the renamed editor tab.
5. Open **设置** → **隐私与本地数据**. Match the fixture's normalized URL in the Trust State list and click **撤销信任**. Close Settings and confirm the course returns to **只读预览**.

## Loopback fixture

From the checkout root, create the fixture and Workspace. Python 3 and Git must be available.

```sh
mkdir "$VERIFY_TMP/git" "$VERIFY_TMP/git-workspace" &&
cp -R builtin-courses/go-goframe-practice "$VERIFY_TMP/git/course.git" &&
git -C "$VERIFY_TMP/git/course.git" init -q &&
git -C "$VERIFY_TMP/git/course.git" add . &&
git -C "$VERIFY_TMP/git/course.git" -c user.name=Fixture \
  -c user.email=fixture@example.test commit -qm 'Initial Course Pack'
```

Run the server in a separate terminal with the same scratch and evidence paths. Read the first line of the log for the import URL. Stage and commit the updated manifest before refreshing. Stop the fixture server with Ctrl+C before removing scratch state.

```sh
.agents/skills/verify-codelab-studio/scripts/serve-git-fixture.py "$VERIFY_TMP/git" \
  > "$VERIFY_EVIDENCE/git-http.log" 2>&1
```
