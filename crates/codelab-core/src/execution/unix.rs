#[cfg(target_os = "macos")]
use std::collections::BTreeSet;
use std::fs::File;
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::OwnedFd;
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
                    || libc::fchdir(spec.directory.handle.as_raw_fd()) < 0
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
        signal_jobs(self.session, signal);
        // The leader remains waitable here, so this group ID cannot be reused.
        unsafe { libc::kill(-self.session, signal) };
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
fn signal_jobs(session: libc::pid_t, signal: libc::c_int) {
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
    let mut groups = BTreeSet::new();
    for pid in pids.into_iter().filter(|&pid| pid > 0) {
        // macOS only exposes numeric IDs for signals here. Recheck both the
        // membership and the group immediately before signaling a job.
        let group = unsafe { libc::getpgid(pid) };
        if group <= 0 || group == session || groups.contains(&group) {
            continue;
        }
        if unsafe {
            libc::getsid(pid) == session
                && libc::getpgid(pid) == group
                && libc::getsid(pid) == session
        } {
            unsafe { libc::kill(-group, signal) };
            groups.insert(group);
        }
    }
}

#[cfg(target_os = "linux")]
fn signal_jobs(session: libc::pid_t, signal: libc::c_int) {
    let pids = std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<libc::pid_t>().ok());
    // Gather handles before signaling so a killed parent cannot hide a detached child.
    let jobs: Vec<_> = pids
        .filter(|&pid| pid != session)
        .filter_map(|pid| {
            let handle = open_pid(pid)?;
            (unsafe { libc::getsid(pid) } == session || child_of_session(pid, &handle, session))
                .then_some(handle)
        })
        .collect();
    for handle in jobs {
        // A pidfd never redirects a signal to a new process after PID reuse.
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                handle.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
    }
}

#[cfg(target_os = "linux")]
fn open_pid(pid: libc::pid_t) -> Option<OwnedFd> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    (fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd as libc::c_int) })
}

#[cfg(target_os = "linux")]
fn child_of_session(pid: libc::pid_t, handle: &OwnedFd, session: libc::pid_t) -> bool {
    fn parent(pid: libc::pid_t) -> Option<libc::pid_t> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // The command name can contain spaces and parentheses.
        let (_, fields) = stat.rsplit_once(')')?;
        fields.split_whitespace().nth(1)?.parse().ok()
    }

    fn alive(handle: &OwnedFd) -> bool {
        let mut entry = libc::pollfd {
            fd: handle.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // A pidfd becomes readable when its process exits.
        unsafe { libc::poll(&mut entry, 1, 0) == 0 }
    }

    // A child that created a new session is still identifiable while its parent
    // remains in our terminal session. Verify the relationship through live handles.
    let Some(parent_pid) = parent(pid).filter(|&parent| parent > 1) else {
        return false;
    };
    let Some(parent_handle) = open_pid(parent_pid) else {
        return false;
    };
    (unsafe { libc::getsid(parent_pid) == session })
        && parent(pid) == Some(parent_pid)
        && alive(handle)
        && alive(&parent_handle)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn signal_jobs(_session: libc::pid_t, _signal: libc::c_int) {}
