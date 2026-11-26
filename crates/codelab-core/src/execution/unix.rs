#[cfg(target_os = "macos")]
use std::collections::{BTreeMap, BTreeSet};
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

#[cfg(target_os = "linux")]
const LSP_PROCESS_OWNER: &str = "CODELAB_LSP_PROCESS_OWNER";

pub(crate) struct Process {
    child: Child,
    master: Option<File>,
    session: libc::pid_t,
    reaped: bool,
    #[cfg(target_os = "macos")]
    lsp_identity: Option<u64>,
    #[cfg(target_os = "linux")]
    lsp_owner: Option<String>,
}

impl Process {
    pub(super) fn spawn(spec: Spawn) -> io::Result<(Self, Ports)> {
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
                master: Some(master),
                session,
                reaped: false,
                #[cfg(target_os = "macos")]
                lsp_identity: None,
                #[cfg(target_os = "linux")]
                lsp_owner: None,
            },
            ports,
        ))
    }

    pub(super) fn spawn_headless(spec: Spawn, output: File) -> io::Result<Self> {
        let mut command = Command::new(spec.program);
        command
            .args(spec.args)
            .envs(spec.env)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::from(output));
        Self::spawn_stdio(command, spec.directory)
    }

    pub(super) fn spawn_piped(spec: Spawn) -> io::Result<(Self, Ports)> {
        let mut command = Command::new(spec.program);
        command
            .args(spec.args)
            .envs(spec.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(target_os = "linux")]
        let owner = {
            use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
            use std::io::Read;

            let mut bytes = [0; 16];
            File::open("/dev/urandom")?.read_exact(&mut bytes)?;
            let owner = URL_SAFE_NO_PAD.encode(bytes);
            command.env(LSP_PROCESS_OWNER, &owner);
            owner
        };
        let mut process = Self::spawn_stdio(command, spec.directory)?;
        #[cfg(target_os = "macos")]
        {
            process.lsp_identity = process_identity(process.session).map(|identity| identity.id);
        }
        #[cfg(target_os = "linux")]
        {
            process.lsp_owner = Some(owner);
        }
        let writer = process.child.stdin.take().expect("piped stdin");
        let reader = process.child.stdout.take().expect("piped stdout");
        Ok((process, (Box::new(reader), Box::new(writer))))
    }

    fn spawn_stdio(mut command: Command, directory: super::Directory) -> io::Result<Self> {
        // SAFETY: the validated directory handle is retained across fork; only
        // async-signal-safe syscalls run before exec.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 || libc::fchdir(directory.handle.as_raw_fd()) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        let session = child.id() as libc::pid_t;
        Ok(Self {
            child,
            master: None,
            session,
            reaped: false,
            #[cfg(target_os = "macos")]
            lsp_identity: None,
            #[cfg(target_os = "linux")]
            lsp_owner: None,
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        let size = winsize(cols, rows);
        // SAFETY: ioctl reads a valid winsize and the master descriptor is open.
        if unsafe {
            libc::ioctl(
                self.master
                    .as_ref()
                    .ok_or(io::ErrorKind::InvalidInput)?
                    .as_raw_fd(),
                libc::TIOCSWINSZ,
                &size,
            )
        } < 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub(crate) fn exited(&self) -> bool {
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
        signal_jobs(self, signal);
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
fn signal_jobs(process: &Process, signal: libc::c_int) {
    let session = process.session;
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

    if let Some(identity) = process.lsp_identity {
        signal_descendants(identity, &pids, signal);
    }

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

#[cfg(target_os = "macos")]
#[repr(C)]
struct ProcessIdentity {
    executable_uuid: [u8; 16],
    id: u64,
    original_parent_id: u64,
    pid_version: i32,
    original_parent_version: i32,
    reserved: [u64; 2],
}

#[cfg(target_os = "macos")]
const _: () = assert!(std::mem::size_of::<ProcessIdentity>() == 56);

#[cfg(target_os = "macos")]
fn process_identity(pid: libc::pid_t) -> Option<ProcessIdentity> {
    // XNU's proc_uniqidentifierinfo ABI. Query the server while it is still
    // waitable and follow original parent IDs, which survive reparenting.
    const PROC_PIDUNIQIDENTIFIERINFO: libc::c_int = 17;
    let mut info = std::mem::MaybeUninit::<ProcessIdentity>::uninit();
    let size = std::mem::size_of::<ProcessIdentity>() as libc::c_int;
    let returned = unsafe {
        libc::proc_pidinfo(
            pid,
            PROC_PIDUNIQIDENTIFIERINFO,
            1,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    (returned == size).then(|| unsafe { info.assume_init() })
}

#[cfg(target_os = "macos")]
fn signal_descendants(root: u64, pids: &[libc::pid_t], signal: libc::c_int) {
    let identities: BTreeMap<_, _> = pids
        .iter()
        .copied()
        .filter(|&pid| pid > 1)
        .filter_map(|pid| process_identity(pid).map(|identity| (identity.id, (pid, identity))))
        .collect();
    for (pid, identity) in identities.values() {
        let mut parent = identity.original_parent_id;
        while parent != root {
            let Some((_, ancestor)) = identities.get(&parent) else {
                break;
            };
            if ancestor.original_parent_id >= parent {
                break;
            }
            parent = ancestor.original_parent_id;
        }
        if parent == root {
            signal_process_identity(*pid, identity, signal);
        }
    }
}

#[cfg(target_os = "macos")]
fn signal_process_identity(pid: libc::pid_t, identity: &ProcessIdentity, signal: libc::c_int) {
    type SignalWithAuditToken = unsafe extern "C" fn(*const [u32; 8], libc::c_int) -> libc::c_int;
    static SIGNAL: std::sync::OnceLock<Option<SignalWithAuditToken>> = std::sync::OnceLock::new();
    let signal_with_audit_token = SIGNAL.get_or_init(|| {
        // Load at runtime for macOS releases that predate this libproc symbol.
        let symbol =
            unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"proc_signal_with_audittoken".as_ptr()) };
        if symbol.is_null() {
            None
        } else {
            // SAFETY: this symbol has the signature defined by libproc.
            Some(unsafe { std::mem::transmute::<*mut libc::c_void, SignalWithAuditToken>(symbol) })
        }
    });
    if let Some(send_signal) = signal_with_audit_token {
        // The kernel checks the PID version before delivering the signal.
        let mut token = [0; 8];
        token[5] = pid as u32;
        token[7] = identity.pid_version as u32;
        unsafe { send_signal(&token, signal) };
    } else if process_identity(pid).is_some_and(|current| current.id == identity.id) {
        unsafe { libc::kill(pid, signal) };
    }
}

#[cfg(target_os = "linux")]
fn signal_jobs(process: &Process, signal: libc::c_int) {
    let session = process.session;
    let owner = process
        .lsp_owner
        .as_ref()
        .map(|owner| format!("{LSP_PROCESS_OWNER}={owner}"));
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
            (unsafe { libc::getsid(pid) } == session
                || child_of_session(pid, &handle, session)
                || owner
                    .as_ref()
                    .is_some_and(|owner| has_lsp_owner(pid, owner)))
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
fn has_lsp_owner(pid: libc::pid_t, owner: &str) -> bool {
    use std::io::{BufRead, BufReader, Read};
    const MAX_ENVIRONMENT_BYTES: u64 = 8 * 1024 * 1024;
    let Ok(environment) = File::open(format!("/proc/{pid}/environ")) else {
        return false;
    };
    // Inspect only the exact inherited ownership entry. Never surface the
    // contents of the environment or signal through a recycled numeric PID.
    BufReader::new(environment.take(MAX_ENVIRONMENT_BYTES))
        .split(0)
        .any(|entry| entry.is_ok_and(|entry| entry == owner.as_bytes()))
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
fn signal_jobs(_process: &Process, _signal: libc::c_int) {}
