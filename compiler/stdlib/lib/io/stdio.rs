//! Standard input, output and error streams.

use super::{BufRead, IoSlice, Lines, Read, Result, Split, Write};
use crate::fmt;
use crate::intrinsics;

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

#[inline]
fn out_write(buf: &[u8]) -> Result<usize> {
    unsafe { intrinsics::write_stdout(buf.as_ptr(), buf.len()) };
    Ok(buf.len())
}

#[inline]
fn err_write(buf: &[u8]) -> Result<usize> {
    unsafe { intrinsics::write_stderr(buf.as_ptr(), buf.len()) };
    Ok(buf.len())
}

fn vectored(bufs: &[IoSlice<'_>], f: fn(&[u8]) -> Result<usize>) -> Result<usize> {
    let mut n = 0;
    for b in bufs {
        n += f(&**b)?;
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// Stdout

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
        out_write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        vectored(bufs, out_write)
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        out_write(buf).map(|_| ())
    }
    fn flush(&mut self) -> Result<()> {
        unsafe { intrinsics::flush_stdout() };
        Ok(())
    }
}

impl Write for &Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        out_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        unsafe { intrinsics::flush_stdout() };
        Ok(())
    }
}

impl<'a> Write for StdoutLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        out_write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        vectored(bufs, out_write)
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        out_write(buf).map(|_| ())
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

impl<'a> fmt::Debug for StdoutLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdoutLock").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Stderr

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
        err_write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        vectored(bufs, err_write)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        err_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<'a> Write for StderrLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        err_write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        vectored(bufs, err_write)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Stderr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stderr").finish_non_exhaustive()
    }
}

impl<'a> fmt::Debug for StderrLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StderrLock").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Stdin

/// A handle to the standard input stream of a process.
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

// The process-wide stdin buffer (in the runtime).
fn stdin_fill_buf<'a>() -> Result<&'a [u8]> {
    unsafe {
        let n = intrinsics::stdin_fill();
        if n < 0 {
            return Err(super::Error::last_os_error());
        }
        Ok(crate::slice::from_raw_parts(intrinsics::stdin_ptr(), n as usize))
    }
}

fn stdin_read(out: &mut [u8]) -> Result<usize> {
    let available = stdin_fill_buf()?;
    let n = if out.len() < available.len() { out.len() } else { available.len() };
    out[..n].copy_from_slice(&available[..n]);
    unsafe { intrinsics::stdin_consume(n) };
    Ok(n)
}

impl Stdin {
    /// Locks this handle to the standard input stream.
    pub fn lock(&self) -> StdinLock<'static> {
        StdinLock { _marker: crate::marker::PhantomData }
    }

    /// Locks this handle and reads a line of input, appending it to the specified buffer.
    pub fn read_line(&self, buf: &mut String) -> Result<usize> {
        self.lock().read_line(buf)
    }

    /// Consumes this handle and returns an iterator over input lines.
    pub fn lines(self) -> Lines<StdinLock<'static>> {
        self.lock().lines()
    }

    /// Consumes this handle and returns an iterator over input bytes split at the given byte.
    pub fn split(self, byte: u8) -> Split<StdinLock<'static>> {
        self.lock().split(byte)
    }

    /// Reads all bytes until EOF into `buf` (as a `String`).
    pub fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.lock().read_to_string(buf)
    }

    /// Reads all bytes until EOF into `buf`.
    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.lock().read_to_end(buf)
    }
}

impl Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        stdin_read(buf)
    }
}

impl Read for &Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        stdin_read(buf)
    }
}

impl<'a> Read for StdinLock<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        stdin_read(buf)
    }
}

impl<'a> BufRead for StdinLock<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        stdin_fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        unsafe { intrinsics::stdin_consume(amt) }
    }
}

impl fmt::Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdin").finish_non_exhaustive()
    }
}

impl<'a> fmt::Debug for StdinLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdinLock").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// IsTerminal

/// A trait to determine whether a descriptor/handle refers to a terminal/tty.
pub trait IsTerminal {
    /// Returns `true` if the descriptor/handle refers to a terminal/tty.
    fn is_terminal(&self) -> bool;
}

pub(crate) fn fd_is_terminal(fd: i32) -> bool {
    unsafe { crate::sys::c_isatty(fd) != 0 }
}

impl IsTerminal for Stdin {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(0)
    }
}

impl<'a> IsTerminal for StdinLock<'a> {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(0)
    }
}

impl IsTerminal for Stdout {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(1)
    }
}

impl<'a> IsTerminal for StdoutLock<'a> {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(1)
    }
}

impl IsTerminal for Stderr {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(2)
    }
}

impl<'a> IsTerminal for StderrLock<'a> {
    fn is_terminal(&self) -> bool {
        fd_is_terminal(2)
    }
}
