# Recheck and install a Managed Runtime

Use the same isolated `CODELAB_DATA_DIR` as the desktop run. Runtime status reflects the host's installed tools. The install check needs network access to the bundled artifact URLs.

1. Open **环境体检** in the top bar. Click **重新探测** and capture the resulting status. Click **管理运行时** and confirm Settings opens on **四语言运行时** with the same status for JS/TS, Go, Java, and Python.
2. Choose **一键安装** for a missing runtime with a bundled artifact for this host. If system runtimes already cover all four languages, turn on **优先托管版** for a language without a managed copy to reveal the install action. Capture the download or install state. The Managed Python package includes uv and pytest support. Managed Java includes JUnit.
3. Capture the completion or error state. On success, confirm the language row identifies a **Managed Runtime** and the installed files are under `$VERIFY_TMP/data/runtimes`. On failure, record the displayed error and whether the install offers a retry.
