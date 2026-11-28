use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use url::Url;

use super::{HostTool, ProbeProblem, ProbeStatus, ToolCheck};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
pub enum HostProbeResult {
    Found {
        output: String,
        executable: Option<PathBuf>,
    },
    Missing,
    Failed,
    TimedOut,
}

pub trait HostToolchainProbe: Send + Sync {
    fn probe(&self, tool: HostTool, timeout: Duration) -> HostProbeResult;

    fn locate_language_server(&self, _tool: HostTool) -> Option<PathBuf> {
        None
    }

    fn probe_go_proxy(&self, _managed_go: Option<&Path>, timeout: Duration) -> HostProbeResult {
        self.probe(HostTool::GoProxy, timeout)
    }
}

pub(super) struct SystemToolchainProbe;

impl HostToolchainProbe for SystemToolchainProbe {
    fn probe(&self, tool: HostTool, timeout: Duration) -> HostProbeResult {
        if tool == HostTool::GoProxy {
            return self.probe_go_proxy(None, timeout);
        }
        let deadline = Instant::now() + timeout;
        if tool == HostTool::Javac {
            return probe_jdk(deadline);
        }
        let Some(executable) = find_on_path(tool).next() else {
            return HostProbeResult::Missing;
        };
        // Preflight reports optional server installation without running the server.
        if tool.is_lsp() {
            return HostProbeResult::Found {
                output: String::new(),
                executable: Some(executable),
            };
        }
        let args: &[&str] = match tool {
            HostTool::Go => &["version"],
            HostTool::UvPython => &[
                "--no-config",
                "python",
                "find",
                "--no-project",
                "--no-python-downloads",
                ">=3.11",
            ],
            _ => &["--version"],
        };
        let mut result = run_probe(&executable, args, deadline);
        if tool == HostTool::UvPython
            && let HostProbeResult::Found { output, .. } = &result
        {
            // uv can find an existing interpreter outside PATH without downloading one.
            let python = Path::new(output.trim());
            result = if python.is_absolute() {
                run_probe(python, &["--version"], deadline)
            } else {
                HostProbeResult::Failed
            };
        }
        result
    }

    fn locate_language_server(&self, tool: HostTool) -> Option<PathBuf> {
        find_on_path(tool).next()
    }

    fn probe_go_proxy(&self, managed_go: Option<&Path>, timeout: Duration) -> HostProbeResult {
        if let Some(value) = env::var_os("GOPROXY").filter(|value| !value.is_empty()) {
            return HostProbeResult::Found {
                output: value.to_string_lossy().into_owned(),
                executable: None,
            };
        }
        let deadline = Instant::now() + timeout;
        // Both Go installations use the same host environment and GOENV settings.
        let executable = managed_go
            .map(Path::to_path_buf)
            .or_else(|| find_on_path(HostTool::Go).next());
        match executable {
            Some(go) => run_probe(&go, &["env", "GOPROXY"], deadline),
            None => HostProbeResult::Missing,
        }
    }
}

pub(super) fn probe_all(
    host: &Arc<dyn HostToolchainProbe>,
    managed_go: Option<&Path>,
    tools: impl Iterator<Item = HostTool>,
) -> HashMap<HostTool, ToolCheck> {
    let (tx, rx) = mpsc::channel();
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let tools: Vec<_> = tools.collect();
    for &tool in &tools {
        let host = Arc::clone(host);
        let tx = tx.clone();
        let managed_go = managed_go.map(Path::to_path_buf);
        let _ = thread::Builder::new()
            .name(format!("preflight-{tool:?}"))
            .spawn(move || {
                let result = if tool == HostTool::GoProxy {
                    host.probe_go_proxy(managed_go.as_deref(), PROBE_TIMEOUT)
                } else {
                    host.probe(tool, PROBE_TIMEOUT)
                };
                let _ = tx.send((tool, result));
            });
    }
    drop(tx);

    let mut checks = HashMap::new();
    while let Ok((tool, result)) =
        rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
        checks.insert(tool, check_from_result(tool, result));
    }
    for tool in tools {
        checks
            .entry(tool)
            .or_insert_with(|| check_from_result(tool, HostProbeResult::TimedOut));
    }
    checks
}

fn probe_jdk(deadline: Instant) -> HostProbeResult {
    let java_home = env::var_os("JAVA_HOME").map(|home| {
        PathBuf::from(home)
            .join("bin")
            .join(executable_name("javac"))
    });
    let candidates = java_home.into_iter().chain(find_on_path(HostTool::Javac));
    let mut fallback = HostProbeResult::Missing;
    for compiler in candidates.filter(|path| {
        path.is_absolute()
            && is_executable(path)
            && is_executable(&path.with_file_name(executable_name("java")))
    }) {
        let result = run_probe(&compiler, &["-version"], deadline);
        match &result {
            HostProbeResult::Found { output, .. } => {
                if version_status(HostTool::Javac, output).1.is_none() {
                    return result;
                }
                fallback = result;
            }
            HostProbeResult::TimedOut => return result,
            _ if matches!(fallback, HostProbeResult::Missing) => fallback = result,
            _ => {}
        }
    }
    fallback
}

fn find_on_path(tool: HostTool) -> impl Iterator<Item = PathBuf> {
    find_on_path_command(tool.command())
}

fn find_on_path_command(name: &str) -> impl Iterator<Item = PathBuf> {
    let directories = host_search_directories();

    let extensions: &[&str] = if cfg!(windows) {
        let has_executable_ext = Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("exe")
                    || ext.eq_ignore_ascii_case("cmd")
                    || ext.eq_ignore_ascii_case("bat")
            });
        if has_executable_ext {
            &[""]
        } else {
            &[".exe", ".cmd", ".bat"]
        }
    } else {
        &[""]
    };

    directories
        .into_iter()
        .filter(|directory| directory.is_absolute())
        .flat_map(move |directory| {
            extensions
                .iter()
                .map(move |extension| directory.join(format!("{name}{extension}")))
        })
        .filter(|path| is_executable(path))
}

pub(crate) fn host_search_directories() -> Vec<PathBuf> {
    collect_search_directories(
        env::var_os("PATH").as_deref(),
        env::home_dir().as_deref(),
        |key| env::var_os(key),
    )
}

fn collect_search_directories(
    path: Option<&OsStr>,
    home: Option<&Path>,
    env_var: impl Fn(&str) -> Option<OsString>,
) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let mut seen = HashSet::new();
    let mut push_dir = |dir: PathBuf| {
        if dir.is_absolute() && seen.insert(dir.clone()) {
            directories.push(dir);
        }
    };

    // 1. Always prioritize explicit PATH entries.
    if let Some(path) = path {
        for dir in env::split_paths(path) {
            push_dir(dir);
        }
    }

    // 2. Desktop launches often have a smaller PATH than interactive shells; check standard bins next.
    if cfg!(unix) {
        push_dir(PathBuf::from("/usr/local/bin"));
    }
    if cfg!(target_os = "macos") {
        push_dir(PathBuf::from("/opt/homebrew/bin"));
    }
    if let Some(home) = home {
        for bin in [".local/bin", ".bun/bin", "go/bin"] {
            push_dir(home.join(bin));
        }
    }

    // 3. Common Node version managers and global npm/pnpm bin paths for GUI-launched apps.
    if let Some(volta_home) = env_var("VOLTA_HOME").filter(|v| !v.is_empty()) {
        push_dir(PathBuf::from(volta_home).join("bin"));
    }
    if let Some(home) = home {
        push_dir(home.join(".volta/bin"));
    }

    if let Some(prefix) = env_var("NPM_CONFIG_PREFIX").filter(|v| !v.is_empty()) {
        let prefix = PathBuf::from(prefix);
        push_dir(if cfg!(windows) {
            prefix
        } else {
            prefix.join("bin")
        });
    }
    if let Some(home) = home {
        push_dir(home.join(".npm-global/bin"));
        push_dir(home.join(".npm-packages/bin"));
    }
    if cfg!(windows)
        && let Some(appdata) = env_var("APPDATA").filter(|v| !v.is_empty())
    {
        push_dir(PathBuf::from(appdata).join("npm"));
    }

    if let Some(pnpm_home) = env_var("PNPM_HOME").filter(|v| !v.is_empty()) {
        push_dir(PathBuf::from(pnpm_home));
    }
    if cfg!(target_os = "macos")
        && let Some(home) = home
    {
        push_dir(home.join("Library/pnpm"));
    }
    if cfg!(unix) && !cfg!(target_os = "macos") {
        if let Some(xdg_data) = env_var("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            push_dir(PathBuf::from(xdg_data).join("pnpm"));
        }
        if let Some(home) = home {
            push_dir(home.join(".local/share/pnpm"));
        }
    }
    if cfg!(windows)
        && let Some(local_appdata) = env_var("LOCALAPPDATA").filter(|v| !v.is_empty())
    {
        push_dir(PathBuf::from(local_appdata).join("pnpm"));
    }

    // nvm (POSIX & Windows)
    if let Some(nvm_bin) = env_var("NVM_BIN").filter(|v| !v.is_empty()) {
        push_dir(PathBuf::from(nvm_bin));
    }
    if cfg!(windows)
        && let Some(nvm_symlink) = env_var("NVM_SYMLINK").filter(|v| !v.is_empty())
    {
        push_dir(PathBuf::from(nvm_symlink));
    }
    let nvm_root = env_var("NVM_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".nvm")));
    if let Some(nvm_root) = nvm_root {
        for dir in nvm_bin_directories(&nvm_root) {
            push_dir(dir);
        }
    }

    // fnm (Fast Node Manager)
    let mut fnm_roots = Vec::new();
    if let Some(fnm_dir) = env_var("FNM_DIR").filter(|v| !v.is_empty()) {
        fnm_roots.push(PathBuf::from(fnm_dir));
    }
    if let Some(xdg_data) = env_var("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        fnm_roots.push(PathBuf::from(xdg_data).join("fnm"));
    }
    if cfg!(target_os = "macos")
        && let Some(home) = home
    {
        fnm_roots.push(home.join("Library/Application Support/fnm"));
    }
    if cfg!(windows) {
        if let Some(appdata) = env_var("APPDATA").filter(|v| !v.is_empty()) {
            fnm_roots.push(PathBuf::from(appdata).join("fnm"));
        }
        if let Some(local_appdata) = env_var("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            fnm_roots.push(PathBuf::from(local_appdata).join("fnm"));
        }
    }
    if let Some(home) = home {
        fnm_roots.push(home.join(".local/share/fnm"));
        fnm_roots.push(home.join(".fnm"));
    }
    for fnm_root in fnm_roots {
        for dir in fnm_bin_directories(&fnm_root) {
            push_dir(dir);
        }
    }

    // asdf / mise shims
    if let Some(home) = home {
        push_dir(home.join(".asdf/shims"));
        push_dir(home.join(".local/share/mise/shims"));
    }
    if let Some(xdg_data) = env_var("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        push_dir(PathBuf::from(xdg_data).join("mise/shims"));
    }

    directories
}

fn node_bin_subdir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.to_path_buf()
    } else {
        root.join("bin")
    }
}

fn parse_semver_key(raw: &str) -> Option<(u32, u32, u32)> {
    let trimmed = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    let mut parts = trimmed.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().unwrap_or("0").parse().ok()?;
    let patch_raw = parts.next().unwrap_or("0");
    let patch_digits: String = patch_raw.chars().take_while(char::is_ascii_digit).collect();
    let patch: u32 = if patch_digits.is_empty() {
        0
    } else {
        patch_digits.parse().ok()?
    };
    Some((major, minor, patch))
}

struct InstalledNodeBin {
    name: String,
    version: Option<(u32, u32, u32)>,
    bin: PathBuf,
}

fn nvm_bin_directories(nvm_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let current = node_bin_subdir(&nvm_root.join("current"));
    if current.is_dir() {
        out.push(current);
    }

    let versions_dir = nvm_root.join("versions/node");
    let Ok(entries) = fs::read_dir(&versions_dir) else {
        return out;
    };
    let mut installed: Vec<InstalledNodeBin> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            let bin = node_bin_subdir(&entry.path());
            bin.is_dir().then(|| {
                let version = parse_semver_key(&name);
                InstalledNodeBin { name, version, bin }
            })
        })
        .collect();
    // Highest semver version first.
    installed.sort_by(|a, b| b.version.cmp(&a.version).then_with(|| b.name.cmp(&a.name)));

    if let Some(default_bin) = resolve_nvm_default_alias(nvm_root, &installed) {
        out.push(default_bin);
    }
    for entry in installed {
        out.push(entry.bin);
    }
    out
}

fn resolve_nvm_default_alias(nvm_root: &Path, installed: &[InstalledNodeBin]) -> Option<PathBuf> {
    let mut alias = fs::read_to_string(nvm_root.join("alias/default"))
        .ok()?
        .trim()
        .to_owned();
    for _ in 0..5 {
        if alias.is_empty() {
            return None;
        }
        let target = alias.strip_prefix('v').unwrap_or(&alias);
        if target.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let exact_v = format!("v{target}");
            let prefix_dot = format!("v{target}.");
            return installed
                .iter()
                .find(|entry| {
                    entry.name == exact_v
                        || entry.name == target
                        || entry.name.starts_with(&prefix_dot)
                })
                .map(|entry| entry.bin.clone());
        }
        // Follow alias chain inside nvm_root/alias/<alias> (e.g. "lts/*" -> "lts/jod" -> "v22...").
        if Path::new(&alias)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            break;
        }
        let next_path = nvm_root.join("alias").join(&alias);
        if let Ok(next) = fs::read_to_string(next_path) {
            let trimmed = next.trim().to_owned();
            if trimmed == alias {
                break;
            }
            alias = trimmed;
            continue;
        }
        break;
    }
    None
}

fn fnm_bin_directories(fnm_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let default_alias = node_bin_subdir(&fnm_root.join("aliases/default"));
    if default_alias.is_dir() {
        out.push(default_alias);
    }
    let versions_dir = fnm_root.join("node-versions");
    let Ok(entries) = fs::read_dir(&versions_dir) else {
        return out;
    };
    let mut installed: Vec<InstalledNodeBin> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            let bin = node_bin_subdir(&entry.path().join("installation"));
            bin.is_dir().then(|| {
                let version = parse_semver_key(&name);
                InstalledNodeBin { name, version, bin }
            })
        })
        .collect();
    installed.sort_by(|a, b| b.version.cmp(&a.version).then_with(|| b.name.cmp(&a.name)));
    for entry in installed {
        out.push(entry.bin);
    }
    out
}

pub(crate) fn find_host_executable(name: &str) -> Option<PathBuf> {
    find_on_path_command(name).next()
}

pub(crate) fn executable_name(name: &str) -> String {
    format!("{name}{}", env::consts::EXE_SUFFIX)
}

pub(crate) fn probe_executable_output(
    executable: &Path,
    args: &[&str],
    timeout: Duration,
) -> Option<String> {
    match run_probe(executable, args, Instant::now() + timeout) {
        HostProbeResult::Found { output, .. } => Some(output),
        _ => None,
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    metadata.is_file()
}

fn run_probe(executable: &Path, args: &[&str], deadline: Instant) -> HostProbeResult {
    if Instant::now() >= deadline {
        return HostProbeResult::TimedOut;
    }
    let result = (|| -> std::io::Result<HostProbeResult> {
        // Sharing a file avoids blocked pipes when a child times out and survives in a descendant.
        let mut output = tempfile::tempfile()?;
        let mut command = Command::new(executable);
        command
            .args(args)
            .current_dir(env::temp_dir())
            .env("GOTOOLCHAIN", "local")
            .env("GOWORK", "off")
            .stdin(Stdio::null())
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::from(output.try_clone()?));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn()?;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) => return Ok(HostProbeResult::Failed),
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return result.map(|_| HostProbeResult::TimedOut);
                }
            }
        }
        output.rewind()?;
        let mut bytes = Vec::new();
        output.take(8192).read_to_end(&mut bytes)?;
        Ok(HostProbeResult::Found {
            output: String::from_utf8_lossy(&bytes).into_owned(),
            executable: Some(executable.to_path_buf()),
        })
    })();
    result.unwrap_or(HostProbeResult::Failed)
}

fn check_from_result(tool: HostTool, result: HostProbeResult) -> ToolCheck {
    let mut check = ToolCheck {
        tool,
        status: ProbeStatus::Warn,
        value: None,
        executable: None,
        problem: None,
    };
    match result {
        HostProbeResult::Missing => check.status = ProbeStatus::Missing,
        HostProbeResult::Failed => check.problem = Some(ProbeProblem::Failed),
        HostProbeResult::TimedOut => check.problem = Some(ProbeProblem::TimedOut),
        HostProbeResult::Found { output, executable } => {
            if tool == HostTool::GoProxy {
                (check.value, check.problem) = proxy_status(&output);
            } else {
                check.executable = executable;
                if !tool.is_lsp() {
                    (check.value, check.problem) = version_status(tool, &output);
                }
            }
            if check.problem.is_none() {
                check.status = ProbeStatus::Ok;
            }
        }
    }
    check
}

fn version_status(tool: HostTool, output: &str) -> (Option<String>, Option<ProbeProblem>) {
    let minimum = match tool {
        HostTool::Node => (20, 0),
        HostTool::Go => (1, 22),
        HostTool::Javac => (21, 0),
        HostTool::Python3 | HostTool::Python | HostTool::UvPython => (3, 11),
        _ => (0, 0),
    };

    let version = output.split_whitespace().find_map(|word| {
        let word = word.trim_matches('"');
        let word = word
            .strip_prefix("go")
            .or_else(|| word.strip_prefix('v'))
            .unwrap_or(word);
        let end = word
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(word.len());
        let (digits, suffix) = word.split_at(end);
        let mut parts = digits.split('.');
        let major: u32 = parts.next()?.parse().ok()?;
        let minor = match parts.next() {
            Some(value) => value.parse().ok()?,
            None if tool == HostTool::Javac => 0,
            None => return None,
        };
        Some((word, (major, minor), suffix))
    });

    let Some((version, number, suffix)) = version else {
        return (None, Some(ProbeProblem::Failed));
    };
    let below_minimum =
        number < minimum || (number == minimum && !suffix.is_empty() && !suffix.starts_with('+'));
    let problem = below_minimum.then_some(ProbeProblem::VersionTooOld);
    (Some(version.to_owned()), problem)
}

fn proxy_status(raw: &str) -> (Option<String>, Option<ProbeProblem>) {
    let mut sanitized = String::new();
    let mut disabled = false;
    for part in raw.trim().split_inclusive([',', '|']) {
        let endpoint = part.trim_end_matches([',', '|']);
        let display = match endpoint {
            "direct" => endpoint.to_owned(),
            "off" => {
                disabled = sanitized.is_empty();
                endpoint.to_owned()
            }
            value => {
                let parsed = if value.contains("://") {
                    Url::parse(value)
                } else {
                    Url::parse(&format!("https://{value}"))
                };
                let Ok(mut url) = parsed else {
                    return (None, Some(ProbeProblem::InvalidProxy));
                };
                if !matches!(url.scheme(), "http" | "https" | "file") {
                    return (None, Some(ProbeProblem::InvalidProxy));
                }
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_query(None);
                url.set_fragment(None);
                url.as_str().trim_end_matches('/').to_owned()
            }
        };
        sanitized.push_str(&display);
        sanitized.push_str(part.strip_prefix(endpoint).unwrap_or_default());
    }
    if sanitized.is_empty() {
        return (None, Some(ProbeProblem::InvalidProxy));
    }
    let problem = disabled.then_some(ProbeProblem::ProxyDisabled);
    (Some(sanitized), problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_directories_prioritize_path_then_standard_bins_then_node_managers() {
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path().join("home");
        let path_dir = temp.path().join("custom-path-bin");
        let nvm_v9 = home.join(".nvm/versions/node/v9.11.2");
        let nvm_v20 = home.join(".nvm/versions/node/v20.11.0");
        let nvm_v22 = home.join(".nvm/versions/node/v22.14.0");
        let fnm_default = home.join(".fnm/aliases/default");
        let fnm_v24 = home.join(".fnm/node-versions/v24.1.0/installation");
        for dir in [
            path_dir.clone(),
            home.join(".local/bin"),
            home.join(".nvm/alias"),
            node_bin_subdir(&nvm_v9),
            node_bin_subdir(&nvm_v20),
            node_bin_subdir(&nvm_v22),
            node_bin_subdir(&fnm_default),
            node_bin_subdir(&fnm_v24),
            home.join(".volta/bin"),
            home.join(".npm-global/bin"),
        ] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(home.join(".nvm/alias/default"), "20\n").unwrap();

        let path_env = env::join_paths([&path_dir]).unwrap();
        let dirs = collect_search_directories(Some(&path_env), Some(&home), |_| None);

        let pos = |p: PathBuf| {
            dirs.iter()
                .position(|candidate| candidate == &p)
                .unwrap_or_else(|| panic!("expected {} in {:?}", p.display(), dirs))
        };

        let p_path = pos(path_dir);
        let p_local = pos(home.join(".local/bin"));
        let p_volta = pos(home.join(".volta/bin"));
        let p_npm_global = pos(home.join(".npm-global/bin"));
        let p_nvm_v20 = pos(node_bin_subdir(&nvm_v20));
        let p_nvm_v22 = pos(node_bin_subdir(&nvm_v22));
        let p_nvm_v9 = pos(node_bin_subdir(&nvm_v9));
        let p_fnm_default = pos(node_bin_subdir(&fnm_default));
        let p_fnm_v24 = pos(node_bin_subdir(&fnm_v24));

        assert!(p_path < p_local, "PATH must precede ~/.local/bin");
        assert!(p_local < p_volta, "standard bins must precede Volta");
        assert!(
            p_local < p_npm_global,
            "standard bins must precede npm-global"
        );
        assert!(p_local < p_nvm_v20, "standard bins must precede nvm");
        assert!(
            p_nvm_v20 < p_nvm_v22,
            "nvm alias/default (20) must precede other versions"
        );
        assert!(
            p_nvm_v22 < p_nvm_v9,
            "nvm versions must sort by numeric semver (v22 before v9)"
        );
        assert!(
            p_fnm_default < p_fnm_v24,
            "fnm default alias must precede version scan"
        );
    }
}
