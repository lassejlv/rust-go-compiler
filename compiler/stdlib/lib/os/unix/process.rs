//! Unix-specific extensions to primitives in the `std::process` module.

use crate::ffi::OsStr;
use crate::io;
use crate::process;

/// Unix-specific extensions to the `process::Command` builder.
pub trait CommandExt: Sized {
    /// Sets the child process's user ID.
    fn uid(&mut self, id: u32) -> &mut process::Command;

    /// Similar to `uid`, but sets the group ID of the child process.
    fn gid(&mut self, id: u32) -> &mut process::Command;

    /// Schedules a closure to be run just before the `exec` function is invoked.
    unsafe fn pre_exec<F>(&mut self, f: F) -> &mut process::Command
    where
        F: FnMut() -> io::Result<()> + Send + Sync + 'static;

    /// Schedules a closure to be run just before the `exec` function is invoked (deprecated).
    unsafe fn before_exec<F>(&mut self, f: F) -> &mut process::Command
    where
        F: FnMut() -> io::Result<()> + Send + Sync + 'static,
    {
        self.pre_exec(f)
    }

    /// Performs all the required setup by this `Command`, followed by calling the `execvp` syscall.
    fn exec(&mut self) -> io::Error;

    /// Set executable argument (`argv[0]`).
    fn arg0<S>(&mut self, arg: S) -> &mut process::Command
    where
        S: AsRef<OsStr>;

    /// Sets the process group ID (PGID) of the child process.
    fn process_group(&mut self, pgroup: i32) -> &mut process::Command;
}

impl CommandExt for process::Command {
    fn uid(&mut self, id: u32) -> &mut process::Command {
        self.set_uid(id);
        self
    }

    fn gid(&mut self, id: u32) -> &mut process::Command {
        self.set_gid(id);
        self
    }

    unsafe fn pre_exec<F>(&mut self, f: F) -> &mut process::Command
    where
        F: FnMut() -> io::Result<()> + Send + Sync + 'static,
    {
        self.push_closure(Box::new(f));
        self
    }

    fn exec(&mut self) -> io::Error {
        self.exec_replace()
    }

    fn arg0<S>(&mut self, arg: S) -> &mut process::Command
    where
        S: AsRef<OsStr>,
    {
        self.set_arg0(arg.as_ref());
        self
    }

    fn process_group(&mut self, pgroup: i32) -> &mut process::Command {
        self.set_pgroup(pgroup);
        self
    }
}

/// Unix-specific extensions to `process::ExitStatus` and `ExitStatusError`.
pub trait ExitStatusExt: Sized {
    /// Creates a new `ExitStatus` from the raw underlying integer status value from `wait`.
    fn from_raw(raw: i32) -> Self;

    /// If the process was terminated by a signal, returns that signal.
    fn signal(&self) -> Option<i32>;

    /// If the process was terminated by a signal, says whether it dumped core.
    fn core_dumped(&self) -> bool;

    /// If the process was stopped by a signal, returns that signal.
    fn stopped_signal(&self) -> Option<i32>;

    /// Whether the process was continued from a stopped status.
    fn continued(&self) -> bool;

    /// Returns the underlying raw `wait` status.
    fn into_raw(self) -> i32;
}

impl ExitStatusExt for process::ExitStatus {
    fn from_raw(raw: i32) -> Self {
        process::ExitStatus::from_raw(raw)
    }
    fn signal(&self) -> Option<i32> {
        self.signal_raw()
    }
    fn core_dumped(&self) -> bool {
        self.core_dumped_raw()
    }
    fn stopped_signal(&self) -> Option<i32> {
        self.stopped_signal_raw()
    }
    fn continued(&self) -> bool {
        self.continued_raw()
    }
    fn into_raw(self) -> i32 {
        process::ExitStatus::into_raw(&self)
    }
}

/// Returns the OS-assigned process identifier associated with this process's parent.
pub fn parent_id() -> u32 {
    unsafe { crate::sys::c_getppid() as u32 }
}
