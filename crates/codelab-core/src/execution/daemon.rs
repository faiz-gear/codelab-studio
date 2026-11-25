use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use super::{CommandLaunch, Directory, Execution, Terminal, lock};
use crate::runtime::PreflightReport;
use crate::{CommandMode, CourseLanguage, DomainError, ErrorCode};

pub(crate) struct DaemonStartup {
    terminal_id: u64,
    started_at: DateTime<Utc>,
    port: u16,
}

impl DaemonStartup {
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn port_open(&self) -> bool {
        port_open(self.port)
    }

    pub(crate) fn wait_until_ready(
        &self,
        mut probe: impl FnMut(&Self) -> Option<Result<bool, DomainError>>,
    ) -> Result<(), DomainError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match probe(self).transpose()? {
                Some(true) => return Ok(()),
                Some(false) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(25));
                }
                _ => return Err(port_error(ErrorCode::DaemonNotReady, self.port)),
            }
        }
    }
}

impl Execution {
    pub(crate) fn start_daemon(
        &mut self,
        mut launch: CommandLaunch,
        directory: Directory,
        report: &PreflightReport,
        language: Option<CourseLanguage>,
        port: u16,
        last_saved_at: DateTime<Utc>,
    ) -> Result<DaemonStartup, DomainError> {
        self.reap();
        let previous = self
            .terminals
            .values()
            .filter(|terminal| {
                terminal.summary.mode == Some(CommandMode::Daemon)
                    && terminal.summary().port == Some(port)
            })
            .max_by_key(|terminal| (terminal.process.is_some(), terminal.started_at));
        if let Some(previous) = previous {
            if previous.process.is_some()
                && port_open(port)
                && previous.started_at >= last_saved_at
                && previous.launch.as_ref().is_some_and(|running| {
                    running.cwd == launch.cwd
                        && running.script == launch.script
                        && running.block.shell == launch.block.shell
                })
            {
                return Ok(previous.daemon_startup(port));
            }
            // A port indicates readiness or conflict. Only an owned terminal can be stopped.
            let id = previous.summary.id;
            self.stop(id)?;
            launch.restart_terminal_id = Some(id);
        }
        if port_open(port) {
            return Err(port_error(ErrorCode::DaemonPortConflict, port));
        }
        let terminal = self.start(Some(launch), directory, report, language)?;
        Ok(self.terminals[&terminal.id].daemon_startup(port))
    }

    pub(crate) fn daemon_running(&mut self, startup: &DaemonStartup) -> Result<bool, DomainError> {
        let terminal = self
            .terminals
            .get_mut(&startup.terminal_id)
            .filter(|terminal| terminal.started_at == startup.started_at)
            .ok_or_else(|| port_error(ErrorCode::DaemonNotReady, startup.port))?;
        terminal.reap();
        if lock(&terminal.capture).port_conflict {
            return Err(port_error(ErrorCode::DaemonPortConflict, startup.port));
        }
        if terminal.process.is_none() {
            return Ok(false);
        }
        Ok(true)
    }
}

impl Terminal {
    fn daemon_startup(&self, port: u16) -> DaemonStartup {
        DaemonStartup {
            terminal_id: self.summary.id,
            started_at: self.started_at,
            port,
        }
    }
}

fn port_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(50),
    )
    .is_ok()
}

fn port_error(code: ErrorCode, port: u16) -> DomainError {
    DomainError::new(code).with_detail("port", port.to_string())
}
