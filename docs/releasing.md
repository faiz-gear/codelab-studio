# Releasing CodeLab Studio

The [Desktop Release workflow](../.github/workflows/release.yml) packages a macOS universal DMG, a Windows x64 NSIS installer, and a Linux x64 AppImage built on Ubuntu 22.04. Each Desktop Release is prepared as a GitHub Release draft for the maintainer to review and publish as a Public Preview.

## Build and prepare the draft

1. Set the same `X.Y.Z` version in `package.json`, `src-tauri/tauri.conf.json`, and `[workspace.package]` in `Cargo.toml`. Keep `Cargo.lock` in sync. CodeLab Studio's initial version is already `0.1.0`.
2. Use the **Desktop Release** workflow's **Run workflow** button to check packaging before creating a tag. Pull requests changing the release workflow or its notes also build installers. Download the packages from the workflow run's **Artifacts** section.
3. Push a matching `vX.Y.Z` tag for the release commit. For the first version, from that commit run `git tag -a v0.1.0 -m 'CodeLab Studio v0.1.0 Public Preview'` and `git push origin v0.1.0`.
4. Wait for all packages and the **Assemble Release draft** job to succeed. Open [GitHub Releases](https://github.com/faiz-gear/codelab-studio/releases) while signed in with repository write access and find the draft. If a job fails, rerun the workflow for the same tag after resolving any transient failure. For a source fix, cut a new version from the corrected commit.

## Verify and publish

1. Check that the draft has the DMG, EXE, AppImage, and `SHA256SUMS.txt`. Compare each downloaded file's SHA-256 to the listed value. On macOS use `shasum -a 256 FILE`, on Windows use `Get-FileHash FILE -Algorithm SHA256`, and on Linux use `sha256sum FILE`.
2. Install or run the downloaded packages on the supported platforms, including both Mac architectures. Verify that the app opens, a built-in Course appears, and it can create and reopen a Workspace. Confirm that installation behavior matches the draft's notes.
3. Add version-specific changes to the notes and publish the draft with **Set as a pre-release** selected.
4. Open the Releases sidebar link on the repository homepage while signed out and download the published assets. During the preview, link directly to `releases/tag/vX.Y.Z` or the Releases list. GitHub's **Latest** release link selects from published full releases. [GitHub release links](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases) · [Latest release API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)
