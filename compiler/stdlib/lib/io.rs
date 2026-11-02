//! Traits, helpers, and type definitions for core I/O functionality.

use crate::fmt;
use crate::intrinsics;

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
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::NotFound => "entity not found",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::ConnectionRefused => "connection refused",
            ErrorKind::ConnectionReset => "connection reset",
            ErrorKind::ConnectionAborted => "connection aborted",
            ErrorKind::NotConnected => "not connected",
            ErrorKind::AddrInUse => "address in use",
            ErrorKind::AddrNotAvailable => "address not available",
            ErrorKind::BrokenPipe => "broken pipe",
            ErrorKind::AlreadyExists => "entity already exists",
            ErrorKind::WouldBlock => "operation would block",
            ErrorKind::InvalidInput => "invalid input parameter",
            ErrorKind::InvalidData => "invalid data",
            ErrorKind::TimedOut => "timed out",
            ErrorKind::WriteZero => "write zero",
            ErrorKind::Interrupted => "operation interrupted",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::UnexpectedEof => "unexpected end of file",
            ErrorKind::OutOfMemory => "out of memory",
            ErrorKind::Other => "other error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The error type for I/O operations.
pub struct Error {
    kind: ErrorKind,
    msg: Option<String>,
}

impl Error {
    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    pub fn new<E: fmt::Display>(kind: ErrorKind, error: E) -> Error {
        Error { kind, msg: Some(error.to_string()) }
    }

    /// Creates a new I/O error from an arbitrary error payload.
    pub fn other<E: fmt::Display>(error: E) -> Error {
        Error::new(ErrorKind::Other, error)
    }

    /// Returns the corresponding `ErrorKind` for this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the OS error that this error represents (if any).
    pub fn raw_os_error(&self) -> Option<i32> {
        None
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind, msg: None }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.msg {
            Some(ref m) => f.debug_struct("Custom").field("kind", &self.kind).field("error", m).finish(),
            None => f.debug_tuple("Kind").field(&self.kind).finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.msg {
            Some(ref m) => f.write_str(m.as_str()),
            None => f.write_str(self.kind.as_str()),
        }
    }
}

impl crate::error::Error for Error {}

/// A trait for objects which are byte-oriented sinks.
pub trait Write {
    /// Writes a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flushes this output stream.
    fn flush(&mut self) -> Result<()>;

    /// Attempts to write an entire buffer into this writer.
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let mut rest = buf;
        while !rest.is_empty() {
            match self.write(rest) {
                Ok(0) => return Err(Error::new(ErrorKind::WriteZero, "failed to write whole buffer")),
                Ok(n) => rest = &rest[n..],
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes a formatted string into this writer.
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        self.write_all(args.as_formatted_str().as_bytes())
    }

    /// Creates a "by reference" adapter for this instance of `Write`.
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }
    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(args)
    }
}

impl<W: Write + ?Sized> Write for Box<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

impl Write for Vec<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// The `Read` trait allows for reading bytes from a source.
pub trait Read {
    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Read all bytes until EOF in this source, placing them into `buf`.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let mut chunk = [0u8; 256];
        let mut total = 0;
        loop {
            let n = self.read(&mut chunk)?;
            if n == 0 {
                return Ok(total);
            }
            buf.extend_from_slice(&chunk[..n]);
            total += n;
        }
    }

    /// Read all bytes until EOF in this source, appending them to `buf`.
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        let mut bytes = Vec::new();
        let n = self.read_to_end(&mut bytes)?;
        match crate::str::from_utf8(&bytes) {
            Ok(s) => {
                buf.push_str(s);
                Ok(n)
            }
            Err(_) => Err(Error::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
        }
    }

    /// Read the exact number of bytes required to fill `buf`.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut filled = 0;
        while filled < buf.len() {
            let n = self.read(&mut buf[filled..])?;
            if n == 0 {
                return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
            }
            filled += n;
        }
        Ok(())
    }
}

impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = if buf.len() < self.len() { buf.len() } else { self.len() };
        buf[..n].copy_from_slice(&self[..n]);
        *self = &self[n..];
        Ok(n)
    }
}

/// A `BufRead` is a type of `Read`er which has an internal buffer.
pub trait BufRead: Read {
    /// Read all bytes until a newline (the `0xA` byte) is reached, and append them to the provided `String` buffer.
    fn read_line(&mut self, buf: &mut String) -> Result<usize>;

    /// Returns an iterator over the lines of this reader.
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { buf: self }
    }
}

/// An iterator over the lines of an instance of `BufRead`.
pub struct Lines<B> {
    buf: B,
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = Result<String>;
    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Standard streams

/// Prints the formatted text to standard output (used by `print!`).
pub fn _print(args: fmt::Arguments<'_>) {
    let s = args.as_formatted_str();
    unsafe { intrinsics::write_stdout(s.as_ptr(), s.len()) }
}

/// Prints the formatted text to standard error (used by `eprint!`).
pub fn _eprint(args: fmt::Arguments<'_>) {
    let s = args.as_formatted_str();
    unsafe { intrinsics::write_stderr(s.as_ptr(), s.len()) }
}

/// A handle to the global standard output stream of the current process.
pub struct Stdout {
    _priv: (),
}

/// A locked reference to the `Stdout` handle.
pub struct StdoutLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

/// Constructs a new handle to the standard output of the current process.
pub fn stdout() -> Stdout {
    Stdout { _priv: () }
}

impl Stdout {
    /// Locks this handle to the standard output stream.
    pub fn lock(&self) -> StdoutLock<'static> {
        StdoutLock { _marker: crate::marker::PhantomData }
    }
}

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe { intrinsics::write_stdout(buf.as_ptr(), buf.len()) };
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        unsafe { intrinsics::flush_stdout() };
        Ok(())
    }
}

impl<'a> Write for StdoutLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe { intrinsics::write_stdout(buf.as_ptr(), buf.len()) };
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        unsafe { intrinsics::flush_stdout() };
        Ok(())
    }
}

impl fmt::Debug for Stdout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdout").finish_non_exhaustive()
    }
}

/// A handle to the standard error stream of a process.
pub struct Stderr {
    _priv: (),
}

/// A locked reference to the `Stderr` handle.
pub struct StderrLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

/// Constructs a new handle to the standard error of the current process.
pub fn stderr() -> Stderr {
    Stderr { _priv: () }
}

impl Stderr {
    /// Locks this handle to the standard error stream.
    pub fn lock(&self) -> StderrLock<'static> {
        StderrLock { _marker: crate::marker::PhantomData }
    }
}

impl Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe { intrinsics::write_stderr(buf.as_ptr(), buf.len()) };
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<'a> Write for StderrLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe { intrinsics::write_stderr(buf.as_ptr(), buf.len()) };
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A handle to the standard input stream of a process. Input is not
/// supported by this runtime: reads always report end of file.
pub struct Stdin {
    _priv: (),
}

/// A locked reference to the `Stdin` handle.
pub struct StdinLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

/// Constructs a new handle to the standard input of the current process.
pub fn stdin() -> Stdin {
    Stdin { _priv: () }
}

impl Stdin {
    /// Locks this handle to the standard input stream.
    pub fn lock(&self) -> StdinLock<'static> {
        StdinLock { _marker: crate::marker::PhantomData }
    }

    /// Locks this handle and reads a line of input, appending it to the specified buffer.
    pub fn read_line(&self, buf: &mut String) -> Result<usize> {
        let _ = buf;
        Ok(0)
    }

    /// Consumes this handle and returns an iterator over input lines.
    pub fn lines(self) -> Lines<StdinLock<'static>> {
        self.lock().lines()
    }
}

impl Read for Stdin {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl<'a> Read for StdinLock<'a> {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl<'a> BufRead for StdinLock<'a> {
    fn read_line(&mut self, _buf: &mut String) -> Result<usize> {
        Ok(0)
    }
}

/// Wraps a writer and buffers its output.
pub struct BufWriter<W: Write> {
    inner: Option<W>,
    buf: Vec<u8>,
}

impl<W: Write> BufWriter<W> {
    /// Creates a new `BufWriter<W>` with a default buffer capacity.
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(8192, inner)
    }

    /// Creates a new `BufWriter<W>` with at least the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter { inner: Some(inner), buf: Vec::with_capacity(capacity) }
    }

    fn flush_buf(&mut self) -> Result<()> {
        if !self.buf.is_empty() {
            if let Some(w) = self.inner.as_mut() {
                w.write_all(&self.buf)?;
            }
            self.buf.clear();
        }
        Ok(())
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.as_ref().unwrap()
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.as_mut().unwrap()
    }

    /// Returns a reference to the internally buffered data.
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    /// Unwraps this `BufWriter<W>`, returning the underlying writer (after flushing the buffer).
    pub fn into_inner(self) -> crate::result::Result<W, Error> {
        let mut me = self;
        me.flush_buf()?;
        Ok(me.inner.take().unwrap())
    }
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.inner.as_mut().unwrap().write(buf)
        } else {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> Result<()> {
        self.flush_buf()?;
        self.inner.as_mut().unwrap().flush()
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            let _ = self.flush_buf();
        }
    }
}

/// Wraps a writer and buffers output to it, flushing whenever a newline is written.
pub type LineWriter<W> = BufWriter<W>;

/// A writer which will move data into the void.
pub struct Sink {
    _priv: (),
}

/// Creates an instance of a writer which will successfully consume all data.
pub const fn sink() -> Sink {
    Sink { _priv: () }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Copies the entire contents of a reader into a writer.
pub fn copy<R: Read + ?Sized, W: Write + ?Sized>(reader: &mut R, writer: &mut W) -> Result<u64> {
    let mut buf = [0u8; 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        writer.write_all(&buf[..n])?;
        total += n as u64;
    }
}

/// Prelude of commonly used I/O traits.
pub mod prelude {
    pub use super::{BufRead, Read, Write};
}
