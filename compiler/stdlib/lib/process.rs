//! A module for working with processes.

use crate::collections::BTreeMap;
use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::fs::File;
use crate::intrinsics;
use crate::io::{self, IoSlice, IoSliceMut, Read, Write};
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::path::Path;
use crate::sys;

/// Terminates the current process with the specified exit code (stdout is flushed).
/// `argc` / `argv` for the C `main` of a `#![no_main]` crate.
#[doc(hidden)]
pub fn __rg_argc() -> i32 { intrinsics::args_count() as i32 }
#[doc(hidden)]
pub fn __rg_argv() -> *const u8 { intrinsics::argv_ptr() }

pub fn exit(code: i32) -> ! {
    unsafe {
        intrinsics::flush_stdout();
        intrinsics::exit(code)
    }
}

/// Terminates the process in an abnormal fashion.
pub fn abort() -> ! {
    unsafe {
        intrinsics::flush_stdout();
        intrinsics::abort()
    }
}

/// Returns the OS-assigned process identifier associated with this process.
pub fn id() -> u32 {
    unsafe { intrinsics::process_id() }
}

// ---------------------------------------------------------------------------
// ExitCode / Termination

/// This type represents the status code the current process can return to its parent.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(u8);

impl fmt::Debug for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Raw(u8);
        impl fmt::Debug for Raw {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("unix_exit_status").field(&self.0).finish()
            }
        }
        f.debug_tuple("ExitCode").field(&Raw(self.0)).finish()
    }
}

impl ExitCode {
    /// The canonical `ExitCode` for successful termination.
    pub const SUCCESS: ExitCode = ExitCode(0);
    /// The canonical `ExitCode` for unsuccessful termination.
    pub const FAILURE: ExitCode = ExitCode(1);

    /// Exit the current process with the given `ExitCode`.
    pub fn exit_process(self) -> ! {
        exit(self.to_i32())
    }

    /// Converts the exit code to an `i32` process status.
    #[inline]
    pub fn to_i32(self) -> i32 {
        self.0 as i32
    }
}

impl Default for ExitCode {
    fn default() -> Self {
        ExitCode::SUCCESS
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        ExitCode(code)
    }
}

/// A trait for implementing arbitrary return types in the `main` function.
pub trait Termination {
    /// Is called to get the representation of the value as status code.
    fn report(self) -> ExitCode;
}

impl Termination for () {
    #[inline]
    fn report(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

impl Termination for crate::convert::Infallible {
    fn report(self) -> ExitCode {
        match self {}
    }
}

impl Termination for ExitCode {
    #[inline]
    fn report(self) -> ExitCode {
        self
    }
}

impl<T: Termination, E: fmt::Debug> Termination for Result<T, E> {
    fn report(self) -> ExitCode {
        match self {
            Ok(val) => val.report(),
            Err(err) => {
                crate::io::_eprint(format_args!("Error: {err:?}\n"));
                ExitCode::FAILURE
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stdio

enum StdioInner {
    Inherit,
    Null,
    MakePipe,
    Fd(OwnedFd),
}

/// Describes what to do with a standard I/O stream for a child process.
pub struct Stdio(StdioInner);

impl Stdio {
    /// A new pipe should be arranged to connect the parent and child processes.
    pub fn piped() -> Stdio {
        Stdio(StdioInner::MakePipe)
    }

    /// The child inherits from the corresponding parent descriptor.
    pub fn inherit() -> Stdio {
        Stdio(StdioInner::Inherit)
    }

    /// This stream will be ignored (connected to `/dev/null`).
    pub fn null() -> Stdio {
        Stdio(StdioInner::Null)
    }

    /// Returns `true` if this requires `Command` to create a new pipe.
    pub fn makes_pipe(&self) -> bool {
        match self.0 {
            StdioInner::MakePipe => true,
            _ => false,
        }
    }

    fn dup(&self) -> io::Result<Stdio> {
        Ok(Stdio(match self.0 {
            StdioInner::Inherit => StdioInner::Inherit,
            StdioInner::Null => StdioInner::Null,
            StdioInner::MakePipe => StdioInner::MakePipe,
            StdioInner::Fd(ref fd) => StdioInner::Fd(fd.try_clone()?),
        }))
    }
}

impl fmt::Debug for Stdio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdio").finish_non_exhaustive()
    }
}

impl From<ChildStdin> for Stdio {
    fn from(child: ChildStdin) -> Stdio {
        Stdio(StdioInner::Fd(child.inner))
    }
}

impl From<ChildStdout> for Stdio {
    fn from(child: ChildStdout) -> Stdio {
        Stdio(StdioInner::Fd(child.inner))
    }
}

impl From<ChildStderr> for Stdio {
    fn from(child: ChildStderr) -> Stdio {
        Stdio(StdioInner::Fd(child.inner))
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Stdio {
        Stdio(StdioInner::Fd(OwnedFd::from(file)))
    }
}

impl From<OwnedFd> for Stdio {
    fn from(fd: OwnedFd) -> Stdio {
        Stdio(StdioInner::Fd(fd))
    }
}

impl From<io::Stdout> for Stdio {
    fn from(_: io::Stdout) -> Stdio {
        sys::flush_stdout();
        match unsafe { BorrowedFd::borrow_raw(1) }.try_clone_to_owned() {
            Ok(fd) => Stdio(StdioInner::Fd(fd)),
            Err(_) => Stdio(StdioInner::Inherit),
        }
    }
}

impl From<io::Stderr> for Stdio {
    fn from(_: io::Stderr) -> Stdio {
        match unsafe { BorrowedFd::borrow_raw(2) }.try_clone_to_owned() {
            Ok(fd) => Stdio(StdioInner::Fd(fd)),
            Err(_) => Stdio(StdioInner::Inherit),
        }
    }
}

impl FromRawFd for Stdio {
    unsafe fn from_raw_fd(fd: RawFd) -> Stdio {
        Stdio(StdioInner::Fd(OwnedFd::from_raw_fd(fd)))
    }
}

// ---------------------------------------------------------------------------
// Child pipes

/// A handle to a child process's standard input (stdin).
pub struct ChildStdin {
    inner: OwnedFd,
}

/// A handle to a child process's standard output (stdout).
pub struct ChildStdout {
    inner: OwnedFd,
}

/// A handle to a child process's stderr.
pub struct ChildStderr {
    inner: OwnedFd,
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }
    fn is_write_vectored(&self) -> bool {
        true
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for &ChildStdin {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for ChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool {
        true
    }
}

impl Read for ChildStderr {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool {
        true
    }
}

macro_rules! child_pipe_impls {
    ($($t:ident $name:expr),*) => {
        $(
            impl fmt::Debug for $t {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_struct($name).finish_non_exhaustive()
                }
            }

            impl AsRawFd for $t {
                #[inline]
                fn as_raw_fd(&self) -> RawFd {
                    self.inner.raw()
                }
            }

            impl AsFd for $t {
                #[inline]
                fn as_fd(&self) -> BorrowedFd<'_> {
                    self.inner.as_fd()
                }
            }

            impl IntoRawFd for $t {
                #[inline]
                fn into_raw_fd(self) -> RawFd {
                    self.inner.into_raw_fd()
                }
            }

            impl From<$t> for OwnedFd {
                #[inline]
                fn from(c: $t) -> OwnedFd {
                    c.inner
                }
            }

            impl From<OwnedFd> for $t {
                #[inline]
                fn from(fd: OwnedFd) -> $t {
                    $t { inner: fd }
                }
            }
        )*
    };
}

child_pipe_impls!(ChildStdin "ChildStdin", ChildStdout "ChildStdout", ChildStderr "ChildStderr");

// ---------------------------------------------------------------------------
// ExitStatus

/// Describes the result of a process after it has terminated (a raw wait status).
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ExitStatus(i32);

/// Describes the result of a process after it has failed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitStatusError(ExitStatus);

impl ExitStatus {
    /// Was termination successful? Signal termination is not considered a success.
    pub fn success(&self) -> bool {
        self.0 == 0
    }

    /// Returns the exit code of the process, if any.
    pub fn code(&self) -> Option<i32> {
        if self.exited() { Some((self.0 >> 8) & 0xff) } else { None }
    }

    /// Was termination successful? Returns a `Result`.
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        if self.success() { Ok(()) } else { Err(ExitStatusError(*self)) }
    }

    fn exited(&self) -> bool {
        self.0 & 0x7f == 0
    }

    pub(crate) fn signal_raw(&self) -> Option<i32> {
        let sig = self.0 & 0x7f;
        if sig != 0 && sig != 0x7f { Some(sig) } else { None }
    }

    pub(crate) fn core_dumped_raw(&self) -> bool {
        self.signal_raw().is_some() && (self.0 & 0x80) != 0
    }

    pub(crate) fn stopped_signal_raw(&self) -> Option<i32> {
        if self.0 & 0xff == 0x7f { Some((self.0 >> 8) & 0xff) } else { None }
    }

    pub(crate) fn continued_raw(&self) -> bool {
        self.0 == 0xffff
    }

    pub(crate) fn into_raw(&self) -> i32 {
        self.0
    }

    pub(crate) fn from_raw(raw: i32) -> ExitStatus {
        ExitStatus(raw)
    }
}

impl Default for ExitStatus {
    fn default() -> Self {
        ExitStatus(0)
    }
}

impl fmt::Debug for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Raw(i32);
        impl fmt::Debug for Raw {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("unix_wait_status").field(&self.0).finish()
            }
        }
        f.debug_tuple("ExitStatus").field(&Raw(self.0)).finish()
    }
}

fn signal_string(signal: i32) -> &'static str {
    match signal {
        1 => " (SIGHUP)",
        2 => " (SIGINT)",
        3 => " (SIGQUIT)",
        4 => " (SIGILL)",
        5 => " (SIGTRAP)",
        6 => " (SIGABRT)",
        7 => " (SIGBUS)",
        8 => " (SIGFPE)",
        9 => " (SIGKILL)",
        10 => " (SIGUSR1)",
        11 => " (SIGSEGV)",
        12 => " (SIGUSR2)",
        13 => " (SIGPIPE)",
        14 => " (SIGALRM)",
        15 => " (SIGTERM)",
        16 => " (SIGSTKFLT)",
        17 => " (SIGCHLD)",
        18 => " (SIGCONT)",
        19 => " (SIGSTOP)",
        20 => " (SIGTSTP)",
        21 => " (SIGTTIN)",
        22 => " (SIGTTOU)",
        23 => " (SIGURG)",
        24 => " (SIGXCPU)",
        25 => " (SIGXFSZ)",
        26 => " (SIGVTALRM)",
        27 => " (SIGPROF)",
        28 => " (SIGWINCH)",
        29 => " (SIGIO)",
        30 => " (SIGPWR)",
        31 => " (SIGSYS)",
        _ => "",
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.code() {
            write!(f, "exit status: {}", code)
        } else if let Some(signal) = self.signal_raw() {
            let signal_string = signal_string(signal);
            if self.core_dumped_raw() {
                write!(f, "signal: {}{} (core dumped)", signal, signal_string)
            } else {
                write!(f, "signal: {}{}", signal, signal_string)
            }
        } else if let Some(signal) = self.stopped_signal_raw() {
            write!(f, "stopped (not terminated) by signal: {}{}", signal, signal_string(signal))
        } else if self.continued_raw() {
            write!(f, "continued (WIFCONTINUED)")
        } else {
            write!(f, "unrecognised wait status: {} {:#x}", self.0, self.0)
        }
    }
}

impl ExitStatusError {
    /// Reports the exit code, if applicable, from an `ExitStatusError`.
    pub fn code(&self) -> Option<i32> {
        self.0.code()
    }

    /// Converts an `ExitStatusError` (back) to an `ExitStatus`.
    pub fn into_status(&self) -> ExitStatus {
        self.0
    }
}

impl fmt::Display for ExitStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "process exited unsuccessfully: {}", self.0)
    }
}

impl crate::error::Error for ExitStatusError {}

impl From<ExitStatusError> for ExitStatus {
    fn from(error: ExitStatusError) -> Self {
        error.0
    }
}

// ---------------------------------------------------------------------------
// Output

/// The output of a finished process.
#[derive(PartialEq, Eq, Clone)]
pub struct Output {
    /// The status (exit code) of the process.
    pub status: ExitStatus,
    /// The data that the process wrote to stdout.
    pub stdout: Vec<u8>,
    /// The data that the process wrote to stderr.
    pub stderr: Vec<u8>,
}

impl fmt::Debug for Output {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stdout_utf8 = crate::str::from_utf8(&self.stdout);
        let stdout_debug: &dyn fmt::Debug = match stdout_utf8 {
            Ok(ref s) => s,
            Err(_) => &self.stdout,
        };
        let stderr_utf8 = crate::str::from_utf8(&self.stderr);
        let stderr_debug: &dyn fmt::Debug = match stderr_utf8 {
            Ok(ref s) => s,
            Err(_) => &self.stderr,
        };
        fmt.debug_struct("Output")
            .field("status", &self.status)
            .field("stdout", stdout_debug)
            .field("stderr", stderr_debug)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Command

/// A process builder, providing fine-grained control over how a new process should be spawned.
pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    env_clear: bool,
    env: BTreeMap<OsString, Option<OsString>>,
    cwd: Option<OsString>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
    uid: Option<u32>,
    gid: Option<u32>,
    pgroup: Option<i32>,
    closures: Vec<Box<dyn FnMut() -> io::Result<()> + Send + Sync>>,
}

/// An iterator over the command arguments.
pub struct CommandArgs<'a> {
    inner: crate::slice::Iter<'a, OsString>,
}

impl<'a> Iterator for CommandArgs<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        self.inner.next().map(|s| s.as_os_str())
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for CommandArgs<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> fmt::Debug for CommandArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.clone()).finish()
    }
}

/// An iterator over the command environment variables.
pub struct CommandEnvs<'a> {
    inner: crate::collections::btree_map::Iter<'a, OsString, Option<OsString>>,
}

impl<'a> Iterator for CommandEnvs<'a> {
    type Item = (&'a OsStr, Option<&'a OsStr>);
    fn next(&mut self) -> Option<(&'a OsStr, Option<&'a OsStr>)> {
        self.inner.next().map(|(k, v)| (k.as_os_str(), v.as_ref().map(|v| v.as_os_str())))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for CommandEnvs<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> fmt::Debug for CommandEnvs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandEnvs").finish_non_exhaustive()
    }
}

// Everything the forked child needs, prepared before `fork` (the child must not allocate).
struct Prepared {
    path: Vec<u8>,
    argv_store: Vec<Vec<u8>>,
    argv: Vec<*const u8>,
    envp_store: Vec<Vec<u8>>,
    envp: Vec<*const u8>,
    cwd: Option<Vec<u8>>,
}

// Pipes and descriptors for one standard stream: (child side, parent side).
struct StdioPair {
    child: Option<OwnedFd>,
    parent: Option<OwnedFd>,
}

fn open_null(write: bool) -> io::Result<OwnedFd> {
    let flags = if write { sys::O_WRONLY } else { sys::O_RDONLY };
    let fd = sys::cvt_r(|| unsafe { sys::c_open(b"/dev/null\0".as_ptr(), flags | sys::O_CLOEXEC) })?;
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn make_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0i32; 2];
    sys::cvt(unsafe { sys::c_pipe2(fds.as_mut_ptr(), sys::O_CLOEXEC) })?;
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

fn setup_stdio(s: &Stdio, readable: bool) -> io::Result<StdioPair> {
    Ok(match s.0 {
        StdioInner::Inherit => StdioPair { child: None, parent: None },
        StdioInner::Null => StdioPair { child: Some(open_null(!readable)?), parent: None },
        StdioInner::MakePipe => {
            let (r, w) = make_pipe()?;
            if readable {
                // the child reads (stdin)
                StdioPair { child: Some(r), parent: Some(w) }
            } else {
                StdioPair { child: Some(w), parent: Some(r) }
            }
        }
        StdioInner::Fd(ref fd) => StdioPair { child: Some(fd.try_clone()?), parent: None },
    })
}

/// A NUL-terminated copy of a program / argument / directory of a `Command`
/// (std reports interior NULs with this message when spawning).
fn cmd_cstr(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.contains(&0) {
        return Err(io::const_io_error(io::ErrorKind::InvalidInput, "nul byte found in provided data"));
    }
    sys::cstr(bytes)
}

fn is_executable_file(path: &[u8]) -> bool {
    let c = match sys::cstr(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let mut st = sys::stat_t::zeroed();
    unsafe {
        if sys::c_stat(c.as_ptr(), &mut st) != 0 {
            return false;
        }
        if st.st_mode & sys::S_IFMT != sys::S_IFREG {
            return false;
        }
        sys::c_access(c.as_ptr(), 1) == 0
    }
}

impl Command {
    /// Constructs a new `Command` for launching the program at path `program`.
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        let program = program.as_ref().to_os_string();
        Command {
            args: vec![program.clone()],
            program,
            env_clear: false,
            env: BTreeMap::new(),
            cwd: None,
            stdin: None,
            stdout: None,
            stderr: None,
            uid: None,
            gid: None,
            pgroup: None,
            closures: Vec::new(),
        }
    }

    /// Adds an argument to pass to the program.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Adds multiple arguments to pass to the program.
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg.as_ref());
        }
        self
    }

    /// Inserts or updates an explicit environment variable mapping.
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env.insert(key.as_ref().to_os_string(), Some(val.as_ref().to_os_string()));
        self
    }

    /// Inserts or updates multiple explicit environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (ref key, ref val) in vars {
            self.env.insert(key.as_ref().to_os_string(), Some(val.as_ref().to_os_string()));
        }
        self
    }

    /// Removes an explicitly set environment variable and prevents inheriting it from a parent process.
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        let key = key.as_ref().to_os_string();
        if self.env_clear {
            self.env.remove(&key);
        } else {
            self.env.insert(key, None);
        }
        self
    }

    /// Clears all explicitly set environment variables and prevents inheriting any parent process environment variables.
    pub fn env_clear(&mut self) -> &mut Command {
        self.env_clear = true;
        self.env.clear();
        self
    }

    /// Sets the working directory for the child process.
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd = Some(dir.as_ref().as_os_str().to_os_string());
        self
    }

    /// Configuration for the child process's standard input (stdin) handle.
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdin = Some(cfg.into());
        self
    }

    /// Configuration for the child process's standard output (stdout) handle.
    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdout = Some(cfg.into());
        self
    }

    /// Configuration for the child process's standard error (stderr) handle.
    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stderr = Some(cfg.into());
        self
    }

    /// Returns the path to the program that was given to `Command::new`.
    pub fn get_program(&self) -> &OsStr {
        &self.program
    }

    /// Returns an iterator of the arguments that will be passed to the program.
    pub fn get_args(&self) -> CommandArgs<'_> {
        CommandArgs { inner: self.args[1..].iter() }
    }

    /// Returns an iterator of the environment variables explicitly set for the child process.
    pub fn get_envs(&self) -> CommandEnvs<'_> {
        CommandEnvs { inner: self.env.iter() }
    }

    /// Returns the working directory for the child process.
    pub fn get_current_dir(&self) -> Option<&Path> {
        self.cwd.as_ref().map(|cs| Path::new(cs))
    }

    pub(crate) fn set_arg0(&mut self, arg: &OsStr) {
        self.args[0] = arg.to_os_string();
    }

    pub(crate) fn set_uid(&mut self, uid: u32) {
        self.uid = Some(uid);
    }

    pub(crate) fn set_gid(&mut self, gid: u32) {
        self.gid = Some(gid);
    }

    pub(crate) fn set_pgroup(&mut self, pgroup: i32) {
        self.pgroup = Some(pgroup);
    }

    pub(crate) fn push_closure(&mut self, f: Box<dyn FnMut() -> io::Result<()> + Send + Sync>) {
        self.closures.push(f);
    }

    // The environment of the child (None: unchanged).
    fn capture_env(&self) -> Option<Vec<Vec<u8>>> {
        if !self.env_clear && self.env.is_empty() {
            return None;
        }
        let mut result: BTreeMap<OsString, OsString> = BTreeMap::new();
        if !self.env_clear {
            for (k, v) in crate::env::vars_os() {
                result.insert(k, v);
            }
        }
        for (k, v) in self.env.iter() {
            match v {
                Some(v) => {
                    result.insert(k.clone(), v.clone());
                }
                None => {
                    result.remove(k);
                }
            }
        }
        let mut out = Vec::new();
        for (k, v) in result {
            let mut e = k.into_encoded_bytes();
            e.push(b'=');
            e.extend_from_slice(v.as_encoded_bytes());
            e.push(0);
            out.push(e);
        }
        Some(out)
    }

    fn resolve_program(&self, env: &Option<Vec<Vec<u8>>>) -> Vec<u8> {
        let prog = self.program.as_encoded_bytes();
        if prog.contains(&b'/') || prog.is_empty() {
            return prog.to_vec();
        }
        // the PATH of the child's environment
        let mut path_var: Option<Vec<u8>> = None;
        match env {
            Some(entries) => {
                for e in entries {
                    if e.starts_with(b"PATH=") {
                        path_var = Some(e[5..e.len() - 1].to_vec());
                    }
                }
            }
            None => {
                path_var = crate::env::var_os("PATH").map(|p| p.into_encoded_bytes());
            }
        }
        let path_var = match path_var {
            Some(p) => p,
            None => b"/bin:/usr/bin".to_vec(),
        };
        for dir in path_var.split(|&b| b == b':') {
            let mut cand = if dir.is_empty() { b".".to_vec() } else { dir.to_vec() };
            cand.push(b'/');
            cand.extend_from_slice(prog);
            if is_executable_file(&cand) {
                return cand;
            }
        }
        prog.to_vec()
    }

    fn prepare(&self) -> io::Result<Prepared> {
        let env = self.capture_env();
        let path = cmd_cstr(&self.resolve_program(&env))?;
        let mut argv_store = Vec::with_capacity(self.args.len());
        for a in self.args.iter() {
            argv_store.push(cmd_cstr(a.as_encoded_bytes())?);
        }
        let mut argv: Vec<*const u8> = Vec::with_capacity(argv_store.len() + 1);
        for a in argv_store.iter() {
            argv.push(a.as_ptr());
        }
        argv.push(crate::ptr::null());
        let envp_store = match env {
            Some(e) => e,
            None => {
                let n = unsafe { intrinsics::env_count() };
                let mut v = Vec::with_capacity(n);
                let mut i = 0;
                while i < n {
                    let mut e = unsafe { sys::from_cstr(intrinsics::env_entry(i)) };
                    e.push(0);
                    v.push(e);
                    i += 1;
                }
                v
            }
        };
        let mut envp: Vec<*const u8> = Vec::with_capacity(envp_store.len() + 1);
        for e in envp_store.iter() {
            envp.push(e.as_ptr());
        }
        envp.push(crate::ptr::null());
        let cwd = match self.cwd {
            Some(ref c) => Some(cmd_cstr(c.as_encoded_bytes())?),
            None => None,
        };
        Ok(Prepared { path, argv_store, argv, envp_store, envp, cwd })
    }

    fn do_spawn(&mut self, default: StdioInner, needs_stdin: bool) -> io::Result<Child> {
        let default_stdio = Stdio(default);
        let stdin_cfg = match self.stdin {
            Some(ref s) => s.dup()?,
            None => {
                if needs_stdin { default_stdio.dup()? } else { Stdio(StdioInner::Null) }
            }
        };
        let stdout_cfg = match self.stdout {
            Some(ref s) => s.dup()?,
            None => default_stdio.dup()?,
        };
        let stderr_cfg = match self.stderr {
            Some(ref s) => s.dup()?,
            None => default_stdio.dup()?,
        };
        let stdin = setup_stdio(&stdin_cfg, true)?;
        let stdout = setup_stdio(&stdout_cfg, false)?;
        let stderr = setup_stdio(&stderr_cfg, false)?;
        let prep = self.prepare()?;
        let (err_r, err_w) = make_pipe()?;

        sys::flush_stdout();
        let pid = unsafe { sys::c_fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            // child
            let errno = self.child_exec(&prep, &stdin, &stdout, &stderr);
            let bytes = [
                (errno >> 24) as u8,
                (errno >> 16) as u8,
                (errno >> 8) as u8,
                errno as u8,
                b'N',
                b'O',
                b'E',
                b'X',
            ];
            unsafe {
                sys::c_write(err_w.raw(), bytes.as_ptr(), 8);
                sys::c_exit_now(1)
            }
        }
        // parent
        drop(err_w);
        drop(stdin.child);
        drop(stdout.child);
        drop(stderr.child);
        let mut buf = [0u8; 8];
        let mut n = 0;
        loop {
            match sys::read_fd(err_r.raw(), &mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => {
                    n += k;
                    if n == 8 {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        drop(err_r);
        let _ = (&prep.argv_store, &prep.envp_store);
        if n == 8 {
            let errno = ((buf[0] as i32) << 24) | ((buf[1] as i32) << 16) | ((buf[2] as i32) << 8) | (buf[3] as i32);
            let mut status = 0;
            unsafe {
                sys::c_waitpid(pid, &mut status, 0);
            }
            return Err(io::Error::from_raw_os_error(errno));
        }
        Ok(Child {
            pid,
            status: None,
            stdin: stdin.parent.map(|fd| ChildStdin { inner: fd }),
            stdout: stdout.parent.map(|fd| ChildStdout { inner: fd }),
            stderr: stderr.parent.map(|fd| ChildStderr { inner: fd }),
        })
    }

    // Runs in the forked child; returns the errno of the failing step.
    fn child_exec(&mut self, prep: &Prepared, stdin: &StdioPair, stdout: &StdioPair, stderr: &StdioPair) -> i32 {
        unsafe {
            if let Some(ref fd) = stdin.child {
                if sys::c_dup2(fd.raw(), 0) < 0 {
                    return sys::errno();
                }
            }
            if let Some(ref fd) = stdout.child {
                if sys::c_dup2(fd.raw(), 1) < 0 {
                    return sys::errno();
                }
            }
            if let Some(ref fd) = stderr.child {
                if sys::c_dup2(fd.raw(), 2) < 0 {
                    return sys::errno();
                }
            }
            if let Some(gid) = self.gid {
                if sys::c_setgid(gid) != 0 {
                    return sys::errno();
                }
            }
            if let Some(uid) = self.uid {
                if sys::c_setuid(uid) != 0 {
                    return sys::errno();
                }
            }
            if let Some(ref cwd) = prep.cwd {
                if sys::c_chdir(cwd.as_ptr()) != 0 {
                    return sys::errno();
                }
            }
            if let Some(pg) = self.pgroup {
                if sys::c_setpgid(0, pg) != 0 {
                    return sys::errno();
                }
            }
            // restore the default SIGPIPE disposition and stack limit
            sys::c_signal(sys::SIGPIPE, 0);
            unsafe { crate::intrinsics::restore_stack_limit() };
            for c in self.closures.iter_mut() {
                if let Err(e) = c() {
                    return match e.raw_os_error() {
                        Some(n) => n,
                        None => sys::EINVAL,
                    };
                }
            }
            c_execve(prep.path.as_ptr(), prep.argv.as_ptr(), prep.envp.as_ptr());
            sys::errno()
        }
    }

    /// Executes the command as a child process, returning a handle to it.
    pub fn spawn(&mut self) -> io::Result<Child> {
        self.do_spawn(StdioInner::Inherit, true)
    }

    /// Executes the command as a child process, waiting for it to finish and collecting all of its output.
    pub fn output(&mut self) -> io::Result<Output> {
        let child = self.do_spawn(StdioInner::MakePipe, false)?;
        child.wait_with_output()
    }

    /// Executes a command as a child process, waiting for it to finish and collecting its status.
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        let mut child = self.do_spawn(StdioInner::Inherit, true)?;
        child.stdin.take();
        child.wait()
    }

    pub(crate) fn exec_replace(&mut self) -> io::Error {
        let stdin_cfg = match self.stdin {
            Some(ref s) => s.dup(),
            None => Ok(Stdio::inherit()),
        };
        let stdout_cfg = match self.stdout {
            Some(ref s) => s.dup(),
            None => Ok(Stdio::inherit()),
        };
        let stderr_cfg = match self.stderr {
            Some(ref s) => s.dup(),
            None => Ok(Stdio::inherit()),
        };
        let (stdin_cfg, stdout_cfg, stderr_cfg) = match (stdin_cfg, stdout_cfg, stderr_cfg) {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            _ => return io::Error::last_os_error(),
        };
        let stdin = match setup_stdio(&stdin_cfg, true) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let stdout = match setup_stdio(&stdout_cfg, false) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let stderr = match setup_stdio(&stderr_cfg, false) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let prep = match self.prepare() {
            Ok(p) => p,
            Err(e) => return e,
        };
        sys::flush_stdout();
        let errno = self.child_exec(&prep, &stdin, &stdout, &stderr);
        io::Error::from_raw_os_error(errno)
    }
}

extern "C" {
    #[link_name = "execve"]
    fn c_execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> i32;
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let mut debug_command = f.debug_struct("Command");
            debug_command.field("program", &self.program).field("args", &self.args);
            if self.env_clear || !self.env.is_empty() {
                struct Env<'a>(&'a Command);
                impl<'a> fmt::Debug for Env<'a> {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.debug_struct("CommandEnv")
                            .field("clear", &self.0.env_clear)
                            .field("vars", &self.0.env)
                            .finish()
                    }
                }
                debug_command.field("env", &Env(self));
            }
            if let Some(ref cwd) = self.cwd {
                debug_command.field("cwd", cwd);
            }
            if let Some(ref stdin) = self.stdin {
                debug_command.field("stdin", stdin);
            }
            if let Some(ref stdout) = self.stdout {
                debug_command.field("stdout", stdout);
            }
            if let Some(ref stderr) = self.stderr {
                debug_command.field("stderr", stderr);
            }
            if let Some(ref uid) = self.uid {
                debug_command.field("uid", uid);
            }
            if let Some(ref gid) = self.gid {
                debug_command.field("gid", gid);
            }
            if let Some(ref pgroup) = self.pgroup {
                debug_command.field("pgroup", pgroup);
            }
            debug_command.finish()
        } else {
            if let Some(ref cwd) = self.cwd {
                write!(f, "cd {:?} && ", cwd)?;
            }
            if self.env_clear {
                write!(f, "env -i ")?;
            } else {
                let mut any_removed = false;
                for (key, value_opt) in self.get_envs() {
                    if value_opt.is_none() {
                        if !any_removed {
                            write!(f, "env ")?;
                            any_removed = true;
                        }
                        write!(f, "-u {} ", key.to_string_lossy())?;
                    }
                }
            }
            for (key, value_opt) in self.get_envs() {
                if let Some(value) = value_opt {
                    write!(f, "{}={:?} ", key.to_string_lossy(), value)?;
                }
            }
            if self.program != self.args[0] {
                write!(f, "[{:?}] ", self.program)?;
            }
            write!(f, "{:?}", self.args[0])?;
            for arg in &self.args[1..] {
                write!(f, " {:?}", arg)?;
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Child

/// Representation of a running or exited child process.
pub struct Child {
    pid: i32,
    status: Option<ExitStatus>,
    /// The handle for writing to the child's standard input (stdin), if it has been captured.
    pub stdin: Option<ChildStdin>,
    /// The handle for reading from the child's standard output (stdout), if it has been captured.
    pub stdout: Option<ChildStdout>,
    /// The handle for reading from the child's standard error (stderr), if it has been captured.
    pub stderr: Option<ChildStderr>,
}

impl Child {
    /// Forces the child process to exit (SIGKILL).
    pub fn kill(&mut self) -> io::Result<()> {
        if self.status.is_some() {
            return Ok(());
        }
        sys::cvt(unsafe { sys::c_kill(self.pid, sys::SIGKILL) })?;
        Ok(())
    }

    /// Returns the OS-assigned process identifier associated with this child.
    pub fn id(&self) -> u32 {
        self.pid as u32
    }

    /// Waits for the child to exit completely, returning the status that it exited with.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        drop(self.stdin.take());
        if let Some(status) = self.status {
            return Ok(status);
        }
        let mut status: i32 = 0;
        let pid = self.pid;
        sys::cvt_r(|| unsafe { sys::c_waitpid(pid, &mut status, 0) })?;
        self.status = Some(ExitStatus(status));
        Ok(ExitStatus(status))
    }

    /// Attempts to collect the exit status of the child if it has already exited.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        let mut status: i32 = 0;
        let pid = sys::cvt(unsafe { sys::c_waitpid(self.pid, &mut status, sys::WNOHANG) })?;
        if pid == 0 {
            Ok(None)
        } else {
            self.status = Some(ExitStatus(status));
            Ok(Some(ExitStatus(status)))
        }
    }

    /// Simultaneously waits for the child to exit and collect all remaining output on the stdout/stderr handles.
    pub fn wait_with_output(mut self) -> io::Result<Output> {
        drop(self.stdin.take());
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        match (self.stdout.take(), self.stderr.take()) {
            (None, None) => {}
            (Some(mut out), None) => {
                out.read_to_end(&mut stdout)?;
            }
            (None, Some(mut err)) => {
                err.read_to_end(&mut stderr)?;
            }
            (Some(out), Some(err)) => {
                read2(out.inner, &mut stdout, err.inner, &mut stderr)?;
            }
        }
        let status = self.wait()?;
        Ok(Output { status, stdout, stderr })
    }
}

// Reads two pipes to EOF concurrently (with poll).
fn read2(p1: OwnedFd, v1: &mut Vec<u8>, p2: OwnedFd, v2: &mut Vec<u8>) -> io::Result<()> {
    sys::set_nonblocking(p1.raw(), true)?;
    sys::set_nonblocking(p2.raw(), true)?;
    let mut fds = [
        sys::pollfd_t { fd: p1.raw(), events: sys::POLLIN, revents: 0 },
        sys::pollfd_t { fd: p2.raw(), events: sys::POLLIN, revents: 0 },
    ];
    let mut done = [false, false];
    let mut buf = [0u8; 8192];
    while !done[0] || !done[1] {
        let nfds: u64 = if done[0] || done[1] { 1 } else { 2 };
        let start = if done[0] { 1 } else { 0 };
        let r = unsafe { sys::c_poll(fds.as_mut_ptr().add(start), nfds, -1) };
        if r < 0 {
            let e = sys::errno();
            if e == sys::EINTR {
                continue;
            }
            return Err(io::Error::from_raw_os_error(e));
        }
        let mut i = 0;
        while i < 2 {
            if !done[i] && fds[i].revents != 0 {
                loop {
                    let n = unsafe { sys::c_read(fds[i].fd, buf.as_mut_ptr(), buf.len()) };
                    if n > 0 {
                        if i == 0 {
                            v1.extend_from_slice(&buf[..n as usize]);
                        } else {
                            v2.extend_from_slice(&buf[..n as usize]);
                        }
                    } else if n == 0 {
                        done[i] = true;
                        break;
                    } else {
                        let e = sys::errno();
                        if e == sys::EAGAIN || e == sys::EINTR {
                            break;
                        }
                        return Err(io::Error::from_raw_os_error(e));
                    }
                }
            }
            i += 1;
        }
    }
    Ok(())
}

impl fmt::Debug for Child {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Child")
            .field("stdin", &self.stdin)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish_non_exhaustive()
    }
}
