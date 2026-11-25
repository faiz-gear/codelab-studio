# Recheck and install a Managed Runtime

Use the same isolated `CODELAB_DATA_DIR` as the desktop run. Runtime status reflects the host's installed tools. The install check needs network access to the bundled artifact URLs.

1. Open **环境体检** in the top bar. Click **重新探测** and capture the resulting status. Click **管理运行时** and confirm Settings shows the same four language runtimes.
2. In Preflight, choose **一键安装** for a missing runtime with a bundled artifact for this host. Capture the download or install state. Python installs with a bundled uv.
3. Capture the completion or error state. On success, confirm the language row identifies a **Managed Runtime** and the installed files are under `$VERIFY_TMP/data/runtimes`. On failure, record the displayed error and whether the install offers a retry.
