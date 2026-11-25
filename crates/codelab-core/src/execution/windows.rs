use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, Instant};

use cap_fs_ext::MetadataExt;
use cap_std::fs::Dir;
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
};
use windows_sys::Win32::System::Console::{
    COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON, ResizePseudoConsole,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
    ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::{Directory, Ports, Spawn};

struct Console(HPCON);

impl Drop for Console {
    fn drop(&mut self) {
        // The output reader remains active while ConPTY flushes on shutdown.
        unsafe { ClosePseudoConsole(self.0) };
    }
}

struct Attributes(Vec<usize>);

impl Attributes {
    fn new(console: &Console) -> io::Result<Self> {
        let mut size = 0;
        // SAFETY: a null list requests the required allocation size.
        unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size) };
        let mut storage = vec![0; size.div_ceil(std::mem::size_of::<usize>())];
        check(unsafe {
            InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 1, 0, &mut size)
        })?;
        let attributes = Self(storage);
        check(unsafe {
            UpdateProcThreadAttribute(
                attributes.ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console.0 as _,
                std::mem::size_of::<HPCON>(),
                null_mut(),
                null(),
            )
        })?;
        Ok(attributes)
    }

    fn ptr(&self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_ptr().cast_mut().cast()
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: the allocation holds an initialized attribute list.
        unsafe { DeleteProcThreadAttributeList(self.ptr()) };
    }
}

pub(super) struct Process {
    finished: bool,
    owns_console: bool,
    job: OwnedHandle,
    child: OwnedHandle,
    console: Option<Console>,
}

impl Process {
    pub fn spawn(spec: Spawn) -> io::Result<(Self, Ports)> {
        let cwd_guards = pin_cwd(&spec.directory)?;
        let cwd_path = dunce::simplified(&spec.directory.path);
        if spec
            .program
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("cmd.exe"))
            && cwd_path.as_os_str().as_encoded_bytes().starts_with(b"\\\\")
        {
            // cmd would change to the Windows directory instead of using this path.
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // Own the job before creating a child. No breakaway flags are enabled.
        let job = owned(unsafe { CreateJobObjectW(null(), null()) })?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        check(unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        })?;

        let (input, writer) = pipe()?;
        let (reader, output) = pipe()?;
        let mut console = 0;
        hresult(unsafe {
            CreatePseudoConsole(
                size(80, 24),
                input.as_raw_handle(),
                output.as_raw_handle(),
                0,
                &mut console,
            )
        })?;
        let console = Console(console);

        let attributes = Attributes::new(&console)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = std::mem::size_of_val(&startup) as u32;
        // Prevent redirected host streams (for example, under CI) from replacing the PTY streams.
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
        startup.lpAttributeList = attributes.ptr();
        let mut information = PROCESS_INFORMATION::default();
        let application = wide(spec.program.as_os_str());
        let mut command_line = Vec::new();
        for argument in std::iter::once(spec.program.into_os_string()).chain(spec.args) {
            if !command_line.is_empty() {
                command_line.push(b' ' as u16);
            }
            quote(&argument, &mut command_line);
        }
        command_line.push(0);
        let cwd = wide(cwd_path.as_os_str());
        let environment = environment(spec.env);
        // SAFETY: the UTF-16 buffers, startup attributes and output struct are alive
        // for the call. The primary thread starts suspended until the job owns it.
        check(unsafe {
            CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                null(),
                null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                environment.as_ptr().cast(),
                cwd.as_ptr(),
                &startup.StartupInfo,
                &mut information,
            )
        })?;
        drop((input, output, cwd_guards));
        let child = owned(information.hProcess)?;
        let primary_thread = owned(information.hThread)?;
        let assigned =
            check(unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) });
        if let Err(error) = assigned {
            unsafe {
                TerminateProcess(child.as_raw_handle(), 1);
                WaitForSingleObject(child.as_raw_handle(), INFINITE);
            }
            return Err(error);
        }
        if unsafe { ResumeThread(primary_thread.as_raw_handle()) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }

        let ports = (
            Box::new(File::from(reader)) as _,
            Box::new(File::from(writer)) as _,
        );
        Ok((
            Self {
                finished: false,
                owns_console: false,
                job,
                child,
                console: Some(console),
            },
            ports,
        ))
    }

    pub fn spawn_headless(spec: Spawn, output: File) -> io::Result<Self> {
        use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
        use windows_sys::Win32::System::Threading::{
            CREATE_NEW_PROCESS_GROUP, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
        };
        let _console_guard = CONSOLE_CONTROL.lock().unwrap_or_else(|p| p.into_inner());
        let owns_console = unsafe { windows_sys::Win32::System::Console::GetConsoleCP() } == 0;
        let guards = pin_cwd(&spec.directory)?;
        let input = File::open("NUL")?;
        let handles = [input.as_raw_handle(), output.as_raw_handle()];
        for handle in handles {
            check(unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT)
            })?;
        }
        let job = owned(unsafe { CreateJobObjectW(null(), null()) })?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        check(unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        })?;
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes);
        }
        let mut storage = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
        check(unsafe {
            InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 1, 0, &mut bytes)
        })?;
        let attributes = Attributes(storage);
        check(unsafe {
            UpdateProcThreadAttribute(
                attributes.ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast_mut().cast(),
                std::mem::size_of_val(&handles),
                null_mut(),
                null(),
            )
        })?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = std::mem::size_of_val(&startup) as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.dwFlags |= windows_sys::Win32::System::Threading::STARTF_USESHOWWINDOW;
        startup.StartupInfo.wShowWindow = 0; // SW_HIDE: the dedicated console stays invisible.
        startup.StartupInfo.hStdInput = input.as_raw_handle();
        startup.StartupInfo.hStdOutput = output.as_raw_handle();
        startup.StartupInfo.hStdError = output.as_raw_handle();
        startup.lpAttributeList = attributes.ptr();
        let application = wide(spec.program.as_os_str());
        let mut command = Vec::new();
        for argument in std::iter::once(spec.program.into_os_string()).chain(spec.args) {
            if !command.is_empty() {
                command.push(b' ' as u16);
            }
            quote(&argument, &mut command);
        }
        command.push(0);
        let cwd = wide(dunce::simplified(&spec.directory.path).as_os_str());
        let env = environment(spec.env);
        let mut info = PROCESS_INFORMATION::default();
        check(unsafe {
            CreateProcessW(
                application.as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                1,
                CREATE_SUSPENDED
                    | CREATE_UNICODE_ENVIRONMENT
                    | EXTENDED_STARTUPINFO_PRESENT
                    | if owns_console {
                        windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE
                    } else {
                        CREATE_NEW_PROCESS_GROUP
                    },
                env.as_ptr().cast(),
                cwd.as_ptr(),
                &startup.StartupInfo,
                &mut info,
            )
        })?;
        drop(guards);
        let child = owned(info.hProcess)?;
        let primary = owned(info.hThread)?;
        if let Err(error) =
            check(unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) })
        {
            unsafe {
                TerminateProcess(child.as_raw_handle(), 1);
                WaitForSingleObject(child.as_raw_handle(), INFINITE);
            }
            return Err(error);
        }
        if unsafe { ResumeThread(primary.as_raw_handle()) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            job,
            child,
            console: None,
            finished: false,
            owns_console,
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        match self.console.as_ref() {
            Some(console) => hresult(unsafe { ResizePseudoConsole(console.0, size(cols, rows)) }),
            None => Ok(()),
        }
    }

    fn exited(&self) -> bool {
        unsafe { WaitForSingleObject(self.child.as_raw_handle(), 0) == WAIT_OBJECT_0 }
    }

    pub fn reap_if_exited(&mut self) -> Option<u32> {
        self.exited().then(|| self.finish())
    }

    pub fn interrupt(&mut self) -> u32 {
        use windows_sys::Win32::System::Console::{
            AttachConsole, CTRL_BREAK_EVENT, FreeConsole, GenerateConsoleCtrlEvent,
            SetConsoleCtrlHandler,
        };
        let _console_guard = CONSOLE_CONTROL.lock().unwrap_or_else(|p| p.into_inner());
        let mut attached = false;
        if self.console.is_none() {
            let pid = unsafe {
                windows_sys::Win32::System::Threading::GetProcessId(self.child.as_raw_handle())
            };
            if self.owns_console {
                attached = unsafe { AttachConsole(pid) } != 0;
                if attached {
                    unsafe {
                        SetConsoleCtrlHandler(Some(ignore_break), 1);
                        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, 0);
                    }
                }
            } else {
                unsafe {
                    GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
                }
            }
        }
        // The caller sends Ctrl-C through ConPTY before waiting. Windows delivers
        // a console interrupt to console programs that support it.
        let until = Instant::now() + Duration::from_millis(400);
        while !self.exited() && Instant::now() < until {
            thread::sleep(Duration::from_millis(10));
        }
        if attached {
            unsafe {
                FreeConsole();
                SetConsoleCtrlHandler(Some(ignore_break), 0);
            }
        }
        self.finish()
    }

    fn finish(&mut self) -> u32 {
        self.finished = true;
        let mut code = 1;
        unsafe {
            TerminateJobObject(self.job.as_raw_handle(), 1);
            WaitForSingleObject(self.child.as_raw_handle(), INFINITE);
            GetExitCodeProcess(self.child.as_raw_handle(), &mut code);
        }
        drop(self.console.take());
        code
    }
}

fn pin_cwd(directory: &Directory) -> io::Result<Vec<Dir>> {
    // CreateProcessW takes a path. Open its components from the root down with
    // write and delete sharing denied, and keep them open until process creation.
    let mut chain: Vec<&Path> = directory.path.ancestors().collect();
    let mut guards = Vec::with_capacity(chain.len());
    while let Some(path) = chain.pop() {
        let file = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        guards.push(Dir::from_std_file(file));
    }
    let actual = guards
        .last()
        .ok_or(io::ErrorKind::InvalidInput)?
        .dir_metadata()?;
    let expected = directory.handle.dir_metadata()?;
    if (actual.dev(), actual.ino()) != (expected.dev(), expected.ino()) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(guards)
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.finished {
            self.finish();
        }
    }
}

fn size(cols: u16, rows: u16) -> COORD {
    COORD {
        X: cols as i16,
        Y: rows as i16,
    }
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: these APIs return new owned non-null handles.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}

fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let (mut read, mut write) = (null_mut(), null_mut());
    check(unsafe { CreatePipe(&mut read, &mut write, null(), 0) })?;
    Ok((owned(read)?, owned(write)?))
}

fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn hresult(result: i32) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::from_raw_os_error(result))
    } else {
        Ok(())
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain([0]).collect()
}

fn environment(overrides: BTreeMap<OsString, OsString>) -> Vec<u16> {
    let mut entries = BTreeMap::new();
    for (key, value) in std::env::vars_os().chain(overrides) {
        entries.insert(key.to_string_lossy().to_uppercase(), value);
    }
    let mut block = Vec::new();
    for (key, value) in entries {
        block.extend(key.encode_utf16());
        block.push(b'=' as u16);
        block.extend(wide(&value));
    }
    block.push(0);
    block
}

fn quote(argument: &OsStr, out: &mut Vec<u16>) {
    if !argument.is_empty()
        && !argument
            .encode_wide()
            .any(|unit| matches!(unit, 9 | 32 | 34))
    {
        out.extend(argument.encode_wide());
        return;
    }
    // Microsoft CRT argument quoting preserves literal backslashes before quotes
    // and at the end of a quoted argument.
    let slash = b'\\' as u16;
    let quote = b'"' as u16;
    let mut slashes = 0;
    out.push(quote);
    for character in argument.encode_wide() {
        if character == slash {
            slashes += 1;
            continue;
        }
        let count = if character == quote {
            slashes * 2 + 1
        } else {
            slashes
        };
        out.extend(std::iter::repeat_n(slash, count));
        out.push(character);
        slashes = 0;
    }
    out.extend(std::iter::repeat_n(slash, slashes * 2));
    out.push(quote);
}

// Console attachment belongs to the process, so serialize headless creation and
// interrupt delivery to avoid one task inheriting another task's temporary console.
static CONSOLE_CONTROL: std::sync::Mutex<()> = std::sync::Mutex::new(());
unsafe extern "system" fn ignore_break(event: u32) -> i32 {
    i32::from(event == windows_sys::Win32::System::Console::CTRL_BREAK_EVENT)
}
