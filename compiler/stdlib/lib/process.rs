//! A module for working with processes.

use crate::fmt;
use crate::intrinsics;

/// Terminates the current process with the specified exit code (stdout is flushed).
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

/// Returns the OS-assigned process identifier associated with this process (not available: 0).
pub fn id() -> u32 {
    0
}

/// This type represents the status code the current process can return to its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCode(u8);

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
