//! Support for capturing a stack backtrace of an OS thread.
//!
//! rustgo cannot walk the stack: captured backtraces are always reported as
//! disabled (or unsupported for `force_capture`), which is a valid outcome of
//! the real API.

use crate::fmt;

/// A captured OS thread stack backtrace.
pub struct Backtrace {
    status: BacktraceStatus,
}

/// The current status of a backtrace.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BacktraceStatus {
    /// Capturing a backtrace is not supported, likely because it's not
    /// implemented for the current platform.
    Unsupported,
    /// Capturing a backtrace has been disabled through either the
    /// `RUST_LIB_BACKTRACE` or `RUST_BACKTRACE` environment variables.
    Disabled,
    /// A backtrace has been captured and the `Backtrace` should print
    /// reasonable information when rendered.
    Captured,
}

/// A single frame of a backtrace.
pub struct BacktraceFrame {
    _private: (),
}

impl fmt::Debug for BacktraceFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().finish()
    }
}

impl Backtrace {
    /// Captures a stack backtrace of the current thread (if enabled).
    #[inline(never)]
    pub fn capture() -> Backtrace {
        let enabled = match crate::env::var_os("RUST_LIB_BACKTRACE") {
            Some(v) => v != "0",
            None => match crate::env::var_os("RUST_BACKTRACE") {
                Some(v) => v != "0",
                None => false,
            },
        };
        if enabled {
            Backtrace { status: BacktraceStatus::Unsupported }
        } else {
            Backtrace { status: BacktraceStatus::Disabled }
        }
    }

    /// Forcibly captures a full backtrace, regardless of environment variable configuration.
    #[inline(never)]
    pub fn force_capture() -> Backtrace {
        Backtrace { status: BacktraceStatus::Unsupported }
    }

    /// Forcibly captures a disabled backtrace, regardless of environment variable configuration.
    pub const fn disabled() -> Backtrace {
        Backtrace { status: BacktraceStatus::Disabled }
    }

    /// Returns the status of this backtrace.
    pub fn status(&self) -> BacktraceStatus {
        self.status
    }

    /// Returns an iterator over the backtrace frames.
    pub fn frames(&self) -> &[BacktraceFrame] {
        &[]
    }
}

impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            BacktraceStatus::Unsupported => f.write_str("unsupported backtrace"),
            BacktraceStatus::Disabled => f.write_str("disabled backtrace"),
            BacktraceStatus::Captured => Ok(()),
        }
    }
}

impl fmt::Debug for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            BacktraceStatus::Unsupported => f.write_str("<unsupported>"),
            BacktraceStatus::Disabled => f.write_str("<disabled>"),
            BacktraceStatus::Captured => f.write_str("[]"),
        }
    }
}
