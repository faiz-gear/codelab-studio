mod daemon;
pub(crate) mod model;
mod output;
pub(crate) mod preparation;
mod process;
pub(crate) mod risk;
#[cfg(unix)]
mod unix;
pub(crate) mod verification;
#[cfg(windows)]
mod windows;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::runtime::{PreflightReport, RuntimeSource};
use crate::workspace::MetricEventDetail;
use crate::{CommandBlock, CommandMode, CourseLanguage, DomainError, ErrorCode};

use self::model::*;
use self::output::Capture;
use self::process::{ErrorLog, TerminalProcess, lock};
#[cfg(unix)]
pub(crate) use self::unix::Process;
#[cfg(windows)]
pub(crate) use self::windows::Process;

type Ports = (Box<dyn Read + Send>, Box<dyn Write + Send>);

struct Spawn {
    program: PathBuf,
    args: Vec<OsString>,
    env: BTreeMap<OsString, OsString>,
    directory: Directory,
}

pub(crate) fn spawn_piped(
    program: PathBuf,
    args: Vec<OsString>,
    directory: Directory,
) -> io::Result<(Process, Ports)> {
    let program = resolve_piped_program(program);
    let env = piped_environment(&program);
    Process::spawn_piped(Spawn {
        program,
        args,
        env,
        directory,
    })
}

fn piped_environment(program: &Path) -> BTreeMap<OsString, OsString> {
    let mut dirs: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    if let Some(parent) = program.parent().filter(|p| p.is_absolute() && p.is_dir())
        && !dirs.iter().any(|existing| existing == parent)
    {
        dirs.push(parent.to_path_buf());
    }
    for dir in crate::runtime::host_search_directories() {
        if dir.is_dir() && !dirs.iter().any(|existing| existing == &dir) {
            dirs.push(dir);
        }
    }
    let mut env = BTreeMap::new();
    if let Ok(joined) = std::env::join_paths(dirs) {
        env.insert(OsString::from("PATH"), joined);
    }
    env
}

fn resolve_piped_program(program: PathBuf) -> PathBuf {
    let Some(name) = program.to_str() else {
        return program;
    };
    if name.chars().any(std::path::is_separator) {
        return program;
    }
    let mut components = program.components();
    let is_bare = matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    );
    if !is_bare {
        return program;
    }
    crate::runtime::find_host_executable(name).unwrap_or(program)
}

#[derive(PartialEq, Eq)]
pub(crate) struct CommandLaunch {
    pub block: CommandBlock,
    pub cwd: PathBuf,
    pub script: String,
    pub restart_terminal_id: Option<u64>,
    pub cwd_key: String,
}

pub(crate) struct Directory {
    pub path: PathBuf,
    pub handle: cap_std::fs::Dir,
}

struct Terminal {
    summary: TerminalSummary,
    started_at: DateTime<Utc>,
    capture: Arc<Mutex<Capture>>,
    process: Option<TerminalProcess>,
    launch: Option<CommandLaunch>,
}

impl Terminal {
    fn summary(&self) -> TerminalSummary {
        let capture = lock(&self.capture);
        TerminalSummary {
            port_conflict: capture.port_conflict,
            port: capture.observed_port.or(self.summary.port),
            ..self.summary.clone()
        }
    }

    fn reap(&mut self) -> Option<MetricEventDetail> {
        let code = self.process.as_mut().and_then(TerminalProcess::exit_code)?;
        self.exited(code, false)
    }

    fn stop(&mut self) -> Option<MetricEventDetail> {
        let code = self.process.as_mut()?.stop();
        self.exited(code, true)
    }

    fn exited(&mut self, code: u32, stopped: bool) -> Option<MetricEventDetail> {
        self.process = None;
        self.summary.running = false;
        self.summary.exit_code = Some(code);
        self.launch.as_ref().map(|_| {
            let duration_ms = Utc::now()
                .signed_duration_since(self.started_at)
                .num_milliseconds()
                .max(0) as u64;
            MetricEventDetail::CmdFinished {
                mode: self.summary.mode,
                exit_code: code,
                stopped,
                duration_ms,
            }
        })
    }
}

#[derive(Default)]
pub(crate) struct Execution {
    terminals: BTreeMap<u64, Terminal>,
    next_id: u64,
    confirmation: Option<(u64, preparation::ConfirmationTarget)>,
    errors: ErrorLog,
    finished_metrics: Vec<MetricEventDetail>,
}

impl Execution {
    pub fn clear(&mut self) {
        self.terminals.clear();
        self.confirmation = None;
        self.errors = ErrorLog::default();
        self.finished_metrics.clear();
    }

    pub fn take_finished_metrics(&mut self) -> Vec<MetricEventDetail> {
        std::mem::take(&mut self.finished_metrics)
    }

    pub fn invalidate_confirmation(&mut self) {
        self.confirmation = None;
    }

    pub fn recent_errors(&self) -> Vec<String> {
        lock(&self.errors).iter().cloned().collect()
    }

    pub fn start(
        &mut self,
        launch: Option<CommandLaunch>,
        dir: Directory,
        report: &PreflightReport,
        language: Option<CourseLanguage>,
    ) -> Result<TerminalSummary, DomainError> {
        let (program, args) = shell(launch.as_ref())?;
        let env = environment(report, language)?;
        let block = launch.as_ref().map(|launch| &launch.block);
        let mode = block.map(|block| block.mode);
        let restart = launch
            .as_ref()
            .and_then(|launch| launch.restart_terminal_id);
        if let Some(id) = restart
            && let Some(metric) = self.terminal_mut(id)?.stop()
        {
            self.finished_metrics.push(metric);
        }
        self.reap();
        let reusable = if mode == Some(CommandMode::Oneshot) {
            self.terminals
                .values()
                .find(|terminal| {
                    terminal.summary.mode == Some(CommandMode::Oneshot)
                        && terminal.process.is_none()
                })
                .map(|terminal| terminal.summary.id)
        } else {
            None
        };
        let id = restart.or(reusable).unwrap_or_else(|| {
            self.next_id += 1;
            self.next_id
        });
        let cwd = label(&dir.path);
        let capture = self.terminals.get(&id).map_or_else(
            || Arc::new(Mutex::new(Capture::default())),
            |terminal| Arc::clone(&terminal.capture),
        );
        {
            let mut output = lock(&capture);
            output.port_conflict = false;
            output.observed_port = None;
        }
        let started_at = Utc::now();
        let process = TerminalProcess::open(
            Spawn {
                program,
                args,
                env,
                directory: dir,
            },
            Arc::clone(&capture),
            Arc::clone(&self.errors),
        )
        .map_err(execution_error)?;
        let summary = TerminalSummary {
            id,
            command_id: block.map(|block| block.id.clone()),
            label: block.and_then(|block| block.label.clone()),
            mode,
            cwd,
            started_at: started_at.to_rfc3339(),
            port: block.and_then(|block| block.port),
            running: true,
            exit_code: None,
            port_conflict: false,
        };
        self.terminals.insert(
            id,
            Terminal {
                summary: summary.clone(),
                started_at,
                capture,
                process: Some(process),
                launch,
            },
        );
        Ok(summary)
    }

    pub fn restart_launch(
        &self,
        terminal_id: u64,
        command_id: &str,
    ) -> Result<&CommandLaunch, DomainError> {
        self.terminals
            .get(&terminal_id)
            .and_then(|terminal| terminal.launch.as_ref())
            .filter(|launch| {
                launch.block.mode == CommandMode::Daemon && launch.block.id == command_id
            })
            .ok_or_else(|| DomainError::new(ErrorCode::TerminalNotFound))
    }

    pub fn read(&mut self, request: TerminalRead) -> TerminalUpdates {
        self.reap();
        let output = request
            .terminal_id
            .and_then(|id| self.terminals.get(&id))
            .map(|terminal| lock(&terminal.capture).output(request.cursor));
        TerminalUpdates {
            terminals: self.terminals.values().map(Terminal::summary).collect(),
            output,
            recent_errors: if request.include_recent_errors.unwrap_or(true) {
                lock(&self.errors).iter().cloned().collect()
            } else {
                Vec::new()
            },
        }
    }

    pub fn input(&mut self, request: TerminalInput) -> Result<(), DomainError> {
        if let Some(process) = &self.terminal_mut(request.target.terminal_id)?.process {
            process.send(request.text).map_err(execution_error)?;
        }
        Ok(())
    }

    pub fn resize(&mut self, request: TerminalSize) -> Result<(), DomainError> {
        if let Some(process) = &self.terminal_mut(request.target.terminal_id)?.process {
            process
                .resize(request.cols, request.rows)
                .map_err(execution_error)?;
        }
        Ok(())
    }

    pub fn stop(&mut self, id: u64) -> Result<(), DomainError> {
        if let Some(metric) = self.terminal_mut(id)?.stop() {
            self.finished_metrics.push(metric);
        }
        Ok(())
    }

    pub fn close(&mut self, id: u64) -> Result<(), DomainError> {
        let mut terminal = self
            .terminals
            .remove(&id)
            .ok_or_else(|| DomainError::new(ErrorCode::TerminalNotFound))?;
        if let Some(metric) = terminal.stop() {
            self.finished_metrics.push(metric);
        }
        Ok(())
    }

    fn terminal_mut(&mut self, id: u64) -> Result<&mut Terminal, DomainError> {
        self.terminals
            .get_mut(&id)
            .ok_or_else(|| DomainError::new(ErrorCode::TerminalNotFound))
    }

    fn reap(&mut self) {
        for terminal in self.terminals.values_mut() {
            if let Some(metric) = terminal.reap() {
                self.finished_metrics.push(metric);
            }
        }
    }
}

fn execution_error(error: io::Error) -> DomainError {
    DomainError::new(ErrorCode::ExecutionFailed).with_detail("reason", error.to_string())
}

fn label(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn shell(launch: Option<&CommandLaunch>) -> Result<(PathBuf, Vec<OsString>), DomainError> {
    if let Some(launch) = launch {
        return shell_command(&launch.block.shell, &launch.script);
    }

    #[cfg(unix)]
    let (program, args) = (
        std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .unwrap_or_else(|| "/bin/sh".into()),
        vec!["-i".into()],
    );
    #[cfg(windows)]
    let (program, args) = (windows_command_shell(), vec!["/D".into(), "/Q".into()]);
    Ok((program, args))
}

#[cfg(windows)]
fn windows_command_shell() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join(r"System32\cmd.exe")
}

fn shell_command(name: &str, script: &str) -> Result<(PathBuf, Vec<OsString>), DomainError> {
    let program = shell_path(name).ok_or_else(|| {
        DomainError::new(ErrorCode::CommandShellUnavailable).with_detail("shell", name)
    })?;
    Ok((program, vec![OsString::from("-c"), OsString::from(script)]))
}

fn shell_path(name: &str) -> Option<PathBuf> {
    #[cfg(unix)]
    let standard = ["/bin", "/usr/bin"]
        .into_iter()
        .map(|dir| Path::new(dir).join(name));
    #[cfg(windows)]
    let standard = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .flat_map(|dir| {
            let dir = PathBuf::from(dir);
            [dir.join("Git/bin"), dir.join("Programs/Git/bin")]
        })
        .map(|dir| dir.join(format!("{name}.exe")));

    let path = std::env::var_os("PATH").unwrap_or_default();
    standard
        .chain(
            std::env::split_paths(&path)
                .filter(|path| path.is_absolute())
                .map(|dir| {
                    if cfg!(windows) {
                        dir.join(format!("{name}.exe"))
                    } else {
                        dir.join(name)
                    }
                }),
        )
        .find(|candidate| {
            #[cfg(windows)]
            if candidate
                .parent()
                .and_then(|path| path.file_name())
                .is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.eq_ignore_ascii_case("System32")
                        || name.eq_ignore_ascii_case("WindowsApps")
                })
            {
                return false;
            }
            candidate.is_file()
        })
}

fn environment(
    report: &PreflightReport,
    language: Option<CourseLanguage>,
) -> Result<BTreeMap<OsString, OsString>, DomainError> {
    let mut env = BTreeMap::from([
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
    ]);
    let mut dirs = Vec::new();
    let rows = report
        .languages
        .iter()
        .filter(|row| Some(row.language) == language)
        .chain(
            report
                .languages
                .iter()
                .filter(|row| Some(row.language) != language),
        );
    for row in rows {
        let executable = match row.selected {
            Some(RuntimeSource::System { tool }) => row
                .system
                .iter()
                .find(|check| check.tool == tool)
                .and_then(|check| check.executable.as_ref()),
            Some(RuntimeSource::Managed) => row.managed.as_ref().map(|runtime| &runtime.executable),
            None => None,
        };
        let Some(bin) = executable
            .filter(|path| path.is_absolute())
            .and_then(|path| path.parent())
        else {
            continue;
        };
        if !dirs.iter().any(|present| present == bin) {
            dirs.push(bin.to_owned());
        }
        if row.selected == Some(RuntimeSource::Managed)
            && let Some(uv_bin) = row
                .managed
                .as_ref()
                .and_then(|runtime| runtime.uv.as_ref())
                .and_then(|uv| uv.parent())
            && !dirs.iter().any(|present| present == uv_bin)
        {
            dirs.push(uv_bin.to_owned());
        }
        let managed = row.selected == Some(RuntimeSource::Managed);
        if row.language == CourseLanguage::Java
            && let Some(runtime) = &row.managed
            && runtime.test_executor.is_file()
        {
            env.insert(
                "CODELAB_JUNIT_JAR".into(),
                runtime.test_executor.as_os_str().to_owned(),
            );
        }
        if let Some(home) = bin.parent() {
            let variable = match row.language {
                CourseLanguage::Java => Some("JAVA_HOME"),
                CourseLanguage::Go if managed => Some("GOROOT"),
                _ => None,
            };
            if let Some(variable) = variable {
                env.insert(variable.into(), home.as_os_str().to_owned());
            }
        }
    }
    if language == Some(CourseLanguage::Python) {
        env.insert("PYTHONUTF8".into(), "1".into());
    }
    if !dirs.is_empty() {
        dirs.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let path =
            std::env::join_paths(dirs).map_err(|_| DomainError::new(ErrorCode::ExecutionFailed))?;
        env.insert("PATH".into(), path);
    }
    Ok(env)
}
