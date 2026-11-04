//! The I/O error type.

use crate::error;
use crate::fmt;
use crate::sys;

/// A specialized `Result` type for I/O operations.
pub type Result<T> = crate::result::Result<T, Error>;

/// A list specifying general categories of I/O error.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    InProgress,
    /// An I/O error (EIO); unstable in std (`io_error_input_output_error`).
    #[doc(hidden)]
    InputOutputError,
    Other,
    #[doc(hidden)]
    Uncategorized,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::AddrInUse => "address in use",
            ErrorKind::AddrNotAvailable => "address not available",
            ErrorKind::AlreadyExists => "entity already exists",
            ErrorKind::ArgumentListTooLong => "argument list too long",
            ErrorKind::BrokenPipe => "broken pipe",
            ErrorKind::ConnectionAborted => "connection aborted",
            ErrorKind::ConnectionRefused => "connection refused",
            ErrorKind::ConnectionReset => "connection reset",
            ErrorKind::CrossesDevices => "cross-device link or rename",
            ErrorKind::Deadlock => "deadlock",
            ErrorKind::DirectoryNotEmpty => "directory not empty",
            ErrorKind::ExecutableFileBusy => "executable file busy",
            ErrorKind::FileTooLarge => "file too large",
            ErrorKind::FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            ErrorKind::QuotaExceeded => "quota exceeded",
            ErrorKind::HostUnreachable => "host unreachable",
            ErrorKind::Interrupted => "operation interrupted",
            ErrorKind::InProgress => "in progress",
            ErrorKind::InputOutputError => "input/output error",
            ErrorKind::InvalidData => "invalid data",
            ErrorKind::InvalidFilename => "invalid filename",
            ErrorKind::InvalidInput => "invalid input parameter",
            ErrorKind::IsADirectory => "is a directory",
            ErrorKind::NetworkDown => "network down",
            ErrorKind::NetworkUnreachable => "network unreachable",
            ErrorKind::NotADirectory => "not a directory",
            ErrorKind::NotConnected => "not connected",
            ErrorKind::NotFound => "entity not found",
            ErrorKind::NotSeekable => "seek on unseekable file",
            ErrorKind::Other => "other error",
            ErrorKind::OutOfMemory => "out of memory",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::ReadOnlyFilesystem => "read-only filesystem or storage medium",
            ErrorKind::ResourceBusy => "resource busy",
            ErrorKind::StaleNetworkFileHandle => "stale network file handle",
            ErrorKind::StorageFull => "no storage space",
            ErrorKind::TimedOut => "timed out",
            ErrorKind::TooManyLinks => "too many links",
            ErrorKind::Uncategorized => "uncategorized error",
            ErrorKind::UnexpectedEof => "unexpected end of file",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::WouldBlock => "operation would block",
            ErrorKind::WriteZero => "write zero",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Maps an `errno` value to its `ErrorKind`.
pub(crate) fn decode_error_kind(errno: i32) -> ErrorKind {
    match errno {
        7 => ErrorKind::ArgumentListTooLong,
        98 => ErrorKind::AddrInUse,
        99 => ErrorKind::AddrNotAvailable,
        16 => ErrorKind::ResourceBusy,
        103 => ErrorKind::ConnectionAborted,
        111 => ErrorKind::ConnectionRefused,
        104 => ErrorKind::ConnectionReset,
        35 => ErrorKind::Deadlock,
        122 => ErrorKind::QuotaExceeded,
        17 => ErrorKind::AlreadyExists,
        27 => ErrorKind::FileTooLarge,
        113 => ErrorKind::HostUnreachable,
        4 => ErrorKind::Interrupted,
        22 => ErrorKind::InvalidInput,
        21 => ErrorKind::IsADirectory,
        40 => ErrorKind::FilesystemLoop,
        2 => ErrorKind::NotFound,
        12 => ErrorKind::OutOfMemory,
        28 => ErrorKind::StorageFull,
        38 => ErrorKind::Unsupported,
        95 => ErrorKind::Unsupported,
        31 => ErrorKind::TooManyLinks,
        36 => ErrorKind::InvalidFilename,
        100 => ErrorKind::NetworkDown,
        101 => ErrorKind::NetworkUnreachable,
        107 => ErrorKind::NotConnected,
        20 => ErrorKind::NotADirectory,
        39 => ErrorKind::DirectoryNotEmpty,
        32 => ErrorKind::BrokenPipe,
        30 => ErrorKind::ReadOnlyFilesystem,
        29 => ErrorKind::NotSeekable,
        116 => ErrorKind::StaleNetworkFileHandle,
        110 => ErrorKind::TimedOut,
        26 => ErrorKind::ExecutableFileBusy,
        18 => ErrorKind::CrossesDevices,
        115 => ErrorKind::InProgress,
        5 => ErrorKind::InputOutputError,
        1 | 13 => ErrorKind::PermissionDenied,
        11 => ErrorKind::WouldBlock,
        _ => ErrorKind::Uncategorized,
    }
}

enum Repr {
    Os(i32),
    Simple(ErrorKind),
    SimpleMessage(ErrorKind, &'static str),
    Custom(Box<Custom>),
}

#[derive(Debug)]
struct Custom {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

/// The error type for I/O operations of the `Read`, `Write`, `Seek`, and
/// associated traits.
pub struct Error {
    repr: Repr,
}

/// Creates an error with a static message (no allocation).
#[inline]
pub(crate) const fn const_io_error(kind: ErrorKind, msg: &'static str) -> Error {
    Error { repr: Repr::SimpleMessage(kind, msg) }
}

impl Error {
    /// Creates a new I/O error from a known kind of error as well as an
    /// arbitrary error payload.
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error { repr: Repr::Custom(Box::new(Custom { kind, error: error.into() })) }
    }

    /// Creates a new I/O error from an arbitrary error payload (kind `Other`).
    pub fn other<E>(error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error::new(ErrorKind::Other, error)
    }

    /// Returns an error representing the last OS error which occurred.
    pub fn last_os_error() -> Error {
        Error::from_raw_os_error(sys::errno())
    }

    /// Creates a new instance of an `Error` from a particular OS error code.
    #[inline]
    pub fn from_raw_os_error(code: i32) -> Error {
        Error { repr: Repr::Os(code) }
    }

    /// Returns the OS error that this error represents (if any).
    #[inline]
    pub fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Os(code) => Some(code),
            _ => None,
        }
    }

    /// Returns a reference to the inner error wrapped by this error (if any).
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Custom(ref c) => Some(&*c.error),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner error wrapped by this error (if any).
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Custom(ref mut c) => Some(&mut *c.error),
            _ => None,
        }
    }

    /// Consumes the `Error`, returning its inner error (if any).
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self.repr {
            Repr::Custom(c) => {
                let c = *c;
                Some(c.error)
            }
            _ => None,
        }
    }

    /// Attempts to downcast the custom boxed error to `E`.
    pub fn downcast<E>(self) -> crate::result::Result<E, Error>
    where
        E: error::Error + Send + Sync + 'static,
    {
        match self.repr {
            Repr::Custom(c) => {
                if c.error.is::<E>() {
                    let c = *c;
                    let raw = Box::into_raw(c.error) as *mut E;
                    Ok(unsafe { *Box::from_raw(raw) })
                } else {
                    Err(Error { repr: Repr::Custom(c) })
                }
            }
            repr => Err(Error { repr }),
        }
    }

    /// Returns the corresponding `ErrorKind` for this error.
    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            Repr::Os(code) => decode_error_kind(code),
            Repr::Simple(kind) => kind,
            Repr::SimpleMessage(kind, _) => kind,
            Repr::Custom(ref c) => c.kind,
        }
    }

    pub(crate) fn is_interrupted(&self) -> bool {
        match self.repr {
            Repr::Os(code) => code == sys::EINTR,
            Repr::Simple(kind) => kind == ErrorKind::Interrupted,
            Repr::SimpleMessage(kind, _) => kind == ErrorKind::Interrupted,
            Repr::Custom(ref c) => c.kind == ErrorKind::Interrupted,
        }
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error { repr: Repr::Simple(kind) }
    }
}

impl From<crate::ffi::NulError> for Error {
    fn from(_: crate::ffi::NulError) -> Error {
        const_io_error(ErrorKind::InvalidInput, "data provided contains a nul byte")
    }
}

impl From<crate::collections::TryReserveError> for Error {
    fn from(_: crate::collections::TryReserveError) -> Error {
        Error::from(ErrorKind::OutOfMemory)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.repr {
            Repr::Os(code) => f
                .debug_struct("Os")
                .field("code", &code)
                .field("kind", &decode_error_kind(code))
                .field("message", &sys::error_string(code))
                .finish(),
            Repr::Custom(ref c) => fmt::Debug::fmt(&**c, f),
            Repr::Simple(kind) => f.debug_tuple("Kind").field(&kind).finish(),
            Repr::SimpleMessage(kind, msg) => f.debug_struct("Error").field("kind", &kind).field("message", &msg).finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.repr {
            Repr::Os(code) => {
                let detail = sys::error_string(code);
                write!(f, "{} (os error {})", detail, code)
            }
            Repr::Custom(ref c) => fmt::Display::fmt(&*c.error, f),
            Repr::Simple(kind) => f.write_str(kind.as_str()),
            Repr::SimpleMessage(_, msg) => f.write_str(msg),
        }
    }
}

impl error::Error for Error {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match self.repr {
            Repr::Os(..) | Repr::Simple(..) => self.kind().as_str(),
            Repr::SimpleMessage(_, msg) => msg,
            Repr::Custom(ref c) => c.error.description(),
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.repr {
            Repr::Custom(ref c) => c.error.source(),
            _ => None,
        }
    }
}
