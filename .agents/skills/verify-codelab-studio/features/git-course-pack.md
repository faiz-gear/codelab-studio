# Import a Git Course Pack and work in its Workspace

Use a known test Git repository with a valid `codelab.json` at its root and a Starter. A scratch copy of the built-in Go / GoFrame pack served by `git http-backend` on loopback works for a local fixture. Use a separate empty Workspace for the import.

1. Open **导入或创作课程包** from the Course menu or welcome page. Select **Git 仓库**, enter the fixture URL and click **打开只读手册**. Capture the **Git Course Pack** and **只读预览** badges and the source URL.
2. Click **信任此课程** and confirm the test URL in the trust dialog before granting trust. Choose the empty Workspace through the native picker. Confirm the bound path and compare a copied Starter file against the fixture.
3. Click **拉取上游更新**. Confirm the same Workspace stays bound and record evidence of the successful fetch when the fixture server logs requests.
4. In the editor, use **新建文件** to create `verification-note.txt` and **重命名文件** to rename it to `verification-renamed.txt`. Confirm the new path exists and the old one is gone. Capture the renamed editor tab.
5. Click **撤销信任** and confirm the course returns to **只读预览**.
