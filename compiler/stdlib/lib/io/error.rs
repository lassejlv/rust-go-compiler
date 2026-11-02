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
    Other,
    #[doc(hidden)]
    Uncategorized,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        match *self {
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            AlreadyExists => "entity already exists",
            ArgumentListTooLong => "argument list too long",
            BrokenPipe => "broken pipe",
            ConnectionAborted => "connection aborted",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            CrossesDevices => "cross-device link or rename",
            Deadlock => "deadlock",
            DirectoryNotEmpty => "directory not empty",
            ExecutableFileBusy => "executable file busy",
            FileTooLarge => "file too large",
            FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            QuotaExceeded => "quota exceeded",
            HostUnreachable => "host unreachable",
            Interrupted => "operation interrupted",
            InProgress => "in progress",
            InvalidData => "invalid data",
            InvalidFilename => "invalid filename",
            InvalidInput => "invalid input parameter",
            IsADirectory => "is a directory",
            NetworkDown => "network down",
            NetworkUnreachable => "network unreachable",
            NotADirectory => "not a directory",
            NotConnected => "not connected",
            NotFound => "entity not found",
            NotSeekable => "seek on unseekable file",
            Other => "other error",
            OutOfMemory => "out of memory",
            PermissionDenied => "permission denied",
            ReadOnlyFilesystem => "read-only filesystem or storage medium",
            ResourceBusy => "resource busy",
            StaleNetworkFileHandle => "stale network file handle",
            StorageFull => "no storage space",
            TimedOut => "timed out",
            TooManyLinks => "too many links",
            Uncategorized => "uncategorized error",
            UnexpectedEof => "unexpected end of file",
            Unsupported => "unsupported",
            WouldBlock => "operation would block",
            WriteZero => "write zero",
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
    use ErrorKind::*;
    match errno {
        7 => ArgumentListTooLong,
        98 => AddrInUse,
        99 => AddrNotAvailable,
        16 => ResourceBusy,
        103 => ConnectionAborted,
        111 => ConnectionRefused,
        104 => ConnectionReset,
        35 => Deadlock,
        122 => QuotaExceeded,
        17 => AlreadyExists,
        27 => FileTooLarge,
        113 => HostUnreachable,
        4 => Interrupted,
        22 => InvalidInput,
        21 => IsADirectory,
        40 => FilesystemLoop,
        2 => NotFound,
        12 => OutOfMemory,
        28 => StorageFull,
        38 => Unsupported,
        95 => Unsupported,
        31 => TooManyLinks,
        36 => InvalidFilename,
        100 => NetworkDown,
        101 => NetworkUnreachable,
        107 => NotConnected,
        20 => NotADirectory,
        39 => DirectoryNotEmpty,
        32 => BrokenPipe,
        30 => ReadOnlyFilesystem,
        29 => NotSeekable,
        116 => StaleNetworkFileHandle,
        110 => TimedOut,
        26 => ExecutableFileBusy,
        18 => CrossesDevices,
        115 => InProgress,
        1 | 13 => PermissionDenied,
        11 => WouldBlock,
        _ => Uncategorized,
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
