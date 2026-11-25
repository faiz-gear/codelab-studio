use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{Ports, Spawn};

pub(super) struct Process {
    child: Child,
    master: File,
    session: libc::pid_t,
    reaped: bool,
}

impl Process {
    pub fn spawn(spec: Spawn) -> io::Result<(Self, Ports)> {
        let mut master = -1;
        let mut slave = -1;
        let mut size = winsize(80, 24);
        // SAFETY: openpty initializes two owned descriptors on success.
        if unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut size,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openpty returned distinct owned descriptors.
        let (master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
        for fd in [&master, &slave] {
            if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        let ports = (
            Box::new(master.try_clone()?) as _,
            Box::new(master.try_clone()?) as _,
        );
        let mut command = Command::new(spec.program);
        command
            .args(spec.args)
            .envs(spec.env)
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        // SAFETY: Only async-signal-safe operations run between fork and exec. The
        // validated Workspace directory handle stays open across fork.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0
                    || libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as _, 0) < 0
                    || libc::fchdir(spec.directory.as_raw_fd()) < 0
                {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        let session = child.id() as libc::pid_t;
        Ok((
            Self {
                child,
                master,
                session,
                reaped: false,
            },
            ports,
        ))
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        let size = winsize(cols, rows);
        // SAFETY: ioctl reads a valid winsize and the master descriptor is open.
        if unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &size) } < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn exited(&self) -> bool {
        // Leave the leader waitable until cleanup has finished so its PID stays reserved.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.session as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        result == 0 && unsafe { info.si_pid() } != 0
    }

    pub fn reap_if_exited(&mut self) -> Option<u32> {
        self.exited().then(|| self.finish())
    }

    pub fn interrupt(&mut self) -> u32 {
        self.signal_session(libc::SIGINT);
        let until = Instant::now() + Duration::from_millis(400);
        while !self.exited() && Instant::now() < until {
            thread::sleep(Duration::from_millis(10));
        }
        self.finish()
    }

    fn finish(&mut self) -> u32 {
        self.signal_session(libc::SIGSTOP);
        self.signal_session(libc::SIGKILL);
        self.reaped = true;
        self.child.wait().ok().map_or(1, |status| {
            use std::os::unix::process::ExitStatusExt;
            status
                .code()
                .or_else(|| status.signal().map(|signal| 128 + signal))
                .unwrap_or(1) as u32
        })
    }

    fn signal_session(&self, signal: libc::c_int) {
        // Interactive shells create new process groups for jobs. Signal every
        // process group in this PTY session, including background jobs.
        let groups: BTreeSet<_> = process_ids()
            .into_iter()
            .filter_map(|pid| {
                // SAFETY: these syscalls query numeric process IDs and can race exit.
                if unsafe { libc::getsid(pid) } != self.session {
                    return None;
                }
                let group = unsafe { libc::getpgid(pid) };
                (group > 0).then_some(group)
            })
            .chain(std::iter::once(self.session))
            .collect();
        for group in groups {
            // SAFETY: a negative PID limits delivery to the selected process group.
            unsafe { libc::kill(-group, signal) };
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.reaped {
            self.finish();
        }
    }
}

fn winsize(cols: u16, rows: u16) -> libc::winsize {
    libc::winsize {
        ws_col: cols,
        ws_row: rows,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

#[cfg(target_os = "macos")]
fn process_ids() -> Vec<libc::pid_t> {
    // SAFETY: the first call measures the buffer and the second fills it.
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    let mut pids = vec![0; count.max(0) as usize + 128];
    let count = unsafe {
        libc::proc_listallpids(
            pids.as_mut_ptr().cast(),
            std::mem::size_of_val(&pids[..]) as libc::c_int,
        )
    };
    pids.truncate(count.max(0) as usize);
    pids.retain(|pid| *pid > 0);
    pids
}

#[cfg(not(target_os = "macos"))]
fn process_ids() -> Vec<libc::pid_t> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .collect()
}
