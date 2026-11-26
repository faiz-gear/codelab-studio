use std::collections::HashMap;
use std::env;
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
        let args: &[&str] = match tool {
            HostTool::Go => &["version"],
            HostTool::Gopls => &["version"],
            HostTool::Jdtls => &["--help"],
            HostTool::Pyright => {
                let Some(cli) = find_on_path_command("pyright").next() else {
                    // Some distributions ship only pyright-langserver.
                    return run_probe(&executable, &["--stdio"], deadline);
                };
                return run_probe(&cli, &["--version"], deadline);
            }
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
    let path = env::var_os("PATH").unwrap_or_default();
    let mut directories: Vec<_> = env::split_paths(&path).collect();
    // Desktop launches often have a smaller PATH than interactive shells.
    if cfg!(unix) {
        directories.push(PathBuf::from("/usr/local/bin"));
    }
    if cfg!(target_os = "macos") {
        directories.push(PathBuf::from("/opt/homebrew/bin"));
    }
    if let Some(home) = env::home_dir() {
        directories.extend(
            [".local/bin", ".bun/bin", "go/bin"]
                .into_iter()
                .map(|bin| home.join(bin)),
        );
    }

    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
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
