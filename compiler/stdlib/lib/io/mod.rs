//! Traits, helpers, and type definitions for core I/O functionality.

use crate::fmt;
use crate::ops::{Deref, DerefMut};

mod buffered;
mod cursor;
mod error;
mod impls;
mod pipe;
mod stdio;
mod util;

pub use self::buffered::{BufReader, BufWriter, IntoInnerError, LineWriter, WriterPanicked};
pub use self::cursor::Cursor;
pub use self::error::{Error, ErrorKind, Result};
pub use self::pipe::{pipe, PipeReader, PipeWriter};
pub(crate) use self::error::{const_io_error, decode_error_kind};
pub use self::stdio::{
    __eprint, __print, _eprint, _print, stderr, stdin, stdout, IsTerminal, Stderr, StderrLock, Stdin, StdinLock, Stdout, StdoutLock,
};
pub(crate) use self::stdio::fd_is_terminal as stdio_is_terminal;
pub(crate) use self::stdio::capture_active;
#[doc(hidden)]
pub use self::stdio::set_output_capture;
pub use self::util::{copy, empty, repeat, sink, Empty, Repeat, Sink};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

// ---------------------------------------------------------------------------
// IoSlice / IoSliceMut

/// A buffer type used with `Write::write_vectored` (ABI-compatible with `iovec`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct IoSlice<'a> {
    base: *const u8,
    len: usize,
    _marker: crate::marker::PhantomData<&'a [u8]>,
}

unsafe impl<'a> Send for IoSlice<'a> {}
unsafe impl<'a> Sync for IoSlice<'a> {}

impl<'a> IoSlice<'a> {
    /// Creates a new `IoSlice` wrapping a byte slice.
    #[inline]
    pub fn new(buf: &'a [u8]) -> IoSlice<'a> {
        IoSlice { base: buf.as_ptr(), len: buf.len(), _marker: crate::marker::PhantomData }
    }

    /// Advance the internal cursor of the slice.
    #[inline]
    pub fn advance(&mut self, n: usize) {
        if self.len < n {
            panic!("advancing IoSlice beyond its length");
        }
        self.base = unsafe { self.base.add(n) };
        self.len -= n;
    }

    /// Advance a slice of slices.
    pub fn advance_slices(bufs: &mut &mut [IoSlice<'a>], n: usize) {
        let mut remove = 0;
        let mut left = n;
        for buf in bufs.iter() {
            if let Some(remainder) = left.checked_sub(buf.len) {
                left = remainder;
                remove += 1;
            } else {
                break;
            }
        }
        *bufs = &mut crate::mem::take(bufs)[remove..];
        if bufs.is_empty() {
            assert!(left == 0, "advancing io slices beyond their length");
        } else {
            bufs[0].advance(left);
        }
    }

    /// Get the underlying bytes as a slice with the original lifetime.
    #[inline]
    pub fn as_slice(self) -> &'a [u8] {
        unsafe { crate::slice::from_raw_parts(self.base, self.len) }
    }
}

impl<'a> Deref for IoSlice<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { crate::slice::from_raw_parts(self.base, self.len) }
    }
}

impl<'a> fmt::Debug for IoSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

/// A buffer type used with `Read::read_vectored` (ABI-compatible with `iovec`).
#[repr(C)]
pub struct IoSliceMut<'a> {
    base: *mut u8,
    len: usize,
    _marker: crate::marker::PhantomData<&'a mut [u8]>,
}

unsafe impl<'a> Send for IoSliceMut<'a> {}
unsafe impl<'a> Sync for IoSliceMut<'a> {}

impl<'a> IoSliceMut<'a> {
    /// Creates a new `IoSliceMut` wrapping a byte slice.
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> IoSliceMut<'a> {
        IoSliceMut { base: buf.as_mut_ptr(), len: buf.len(), _marker: crate::marker::PhantomData }
    }

    /// Advance the internal cursor of the slice.
    #[inline]
    pub fn advance(&mut self, n: usize) {
        if self.len < n {
            panic!("advancing IoSliceMut beyond its length");
        }
        self.base = unsafe { self.base.add(n) };
        self.len -= n;
    }

    /// Advance a slice of slices.
    pub fn advance_slices(bufs: &mut &mut [IoSliceMut<'a>], n: usize) {
        let mut remove = 0;
        let mut left = n;
        for buf in bufs.iter() {
            if let Some(remainder) = left.checked_sub(buf.len) {
                left = remainder;
                remove += 1;
            } else {
                break;
            }
        }
        *bufs = &mut crate::mem::take(bufs)[remove..];
        if bufs.is_empty() {
            assert!(left == 0, "advancing io slices beyond their length");
        } else {
            bufs[0].advance(left);
        }
    }
}

impl<'a> Deref for IoSliceMut<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { crate::slice::from_raw_parts(self.base as *const u8, self.len) }
    }
}

impl<'a> DerefMut for IoSliceMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { crate::slice::from_raw_parts_mut(self.base, self.len) }
    }
}

impl<'a> fmt::Debug for IoSliceMut<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

// ---------------------------------------------------------------------------
// Read

pub(crate) fn default_read_to_end<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize> {
    let start = buf.len();
    let mut chunk = [0u8; DEFAULT_BUF_SIZE];
    loop {
        match r.read(&mut chunk) {
            Ok(0) => return Ok(buf.len() - start),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(ref e) if e.is_interrupted() => {}
            Err(e) => return Err(e),
        }
    }
}

pub(crate) fn append_to_string<F>(buf: &mut String, f: F) -> Result<usize>
where
    F: FnOnce(&mut Vec<u8>) -> Result<usize>,
{
    let old_len = buf.len();
    let mut bytes = Vec::new();
    let ret = f(&mut bytes);
    match crate::str::from_utf8(&bytes) {
        Ok(s) => {
            buf.push_str(s);
            ret
        }
        Err(_) => {
            let _ = old_len;
            match ret {
                Err(e) => Err(e),
                Ok(_) => Err(const_io_error(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
            }
        }
    }
}

pub(crate) fn default_read_exact<R: Read + ?Sized>(this: &mut R, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match this.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.is_interrupted() => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(const_io_error(ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
    } else {
        Ok(())
    }
}

/// The `Read` trait allows for reading bytes from a source.
pub trait Read {
    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Like `read`, except that it reads into a slice of buffers.
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        for b in bufs.iter_mut() {
            if !b.is_empty() {
                return self.read(&mut **b);
            }
        }
        self.read(&mut [])
    }

    /// Determines if this `Read`er has an efficient `read_vectored` implementation.
    fn is_read_vectored(&self) -> bool {
        false
    }

    /// Read all bytes until EOF in this source, placing them into `buf`.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_read_to_end(self, buf)
    }

    /// Read all bytes until EOF in this source, appending them to `buf`.
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |b| self.read_to_end(b))
    }

    /// Read the exact number of bytes required to fill `buf`.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        default_read_exact(self, buf)
    }

    /// Creates a "by reference" adapter for this instance of `Read`.
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// Transforms this `Read` instance to an `Iterator` over its bytes.
    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { inner: self }
    }

    /// Creates an adapter which will chain this stream with another.
    fn chain<R: Read>(self, next: R) -> Chain<Self, R>
    where
        Self: Sized,
    {
        Chain { first: self, second: next, done_first: false }
    }

    /// Creates an adapter which will read at most `limit` bytes from it.
    fn take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, len: limit, limit }
    }
}

/// Reads all bytes from a reader into a new `String`.
pub fn read_to_string<R: Read>(mut reader: R) -> Result<String> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Write

/// A trait for objects which are byte-oriented sinks.
pub trait Write {
    /// Writes a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Like `write`, except that it writes from a slice of buffers.
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        for b in bufs.iter() {
            if !b.is_empty() {
                return self.write(&**b);
            }
        }
        self.write(&[])
    }

    /// Determines if this `Write`r has an efficient `write_vectored` implementation.
    fn is_write_vectored(&self) -> bool {
        false
    }

    /// Flushes this output stream.
    fn flush(&mut self) -> Result<()>;

    /// Attempts to write an entire buffer into this writer.
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(const_io_error(ErrorKind::WriteZero, "failed to write whole buffer"));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Attempts to write multiple buffers into this writer.
    fn write_all_vectored(&mut self, mut bufs: &mut [IoSlice<'_>]) -> Result<()> {
        IoSlice::advance_slices(&mut bufs, 0);
        while !bufs.is_empty() {
            match self.write_vectored(bufs) {
                Ok(0) => {
                    return Err(const_io_error(ErrorKind::WriteZero, "failed to write whole buffer"));
                }
                Ok(n) => IoSlice::advance_slices(&mut bufs, n),
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes a formatted string into this writer.
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        // each piece the formatting wrote is a separate `write_all`, like
        // rustc's adapter over `fmt::write`
        let n = args.segment_count();
        let mut i = 0;
        while i < n {
            self.write_all(args.segment(i).as_bytes())?;
            i += 1;
        }
        if args.failed() {
            return Err(Error::new(ErrorKind::Other, "formatter error"));
        }
        Ok(())
    }

    /// Creates a "by reference" adapter for this instance of `Write`.
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

// ---------------------------------------------------------------------------
// Seek

/// Enumeration of possible methods to seek within an I/O object.
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    /// Sets the offset to the provided number of bytes.
    Start(u64),
    /// Sets the offset to the size of this object plus the specified number of bytes.
    End(i64),
    /// Sets the offset to the current position plus the specified number of bytes.
    Current(i64),
}

/// The `Seek` trait provides a cursor which can be moved within a stream of bytes.
pub trait Seek {
    /// Seek to an offset, in bytes, in a stream.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    /// Rewind to the beginning of a stream.
    fn rewind(&mut self) -> Result<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    /// Returns the length of this stream (in bytes).
    fn stream_len(&mut self) -> Result<u64> {
        let old_pos = self.stream_position()?;
        let len = self.seek(SeekFrom::End(0))?;
        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }
        Ok(len)
    }

    /// Returns the current seek position from the start of the stream.
    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    /// Seeks relative to the current position.
    fn seek_relative(&mut self, offset: i64) -> Result<()> {
        self.seek(SeekFrom::Current(offset))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BufRead

fn read_until<R: BufRead + ?Sized>(r: &mut R, delim: u8, buf: &mut Vec<u8>) -> Result<usize> {
    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.is_interrupted() => continue,
                Err(e) => return Err(e),
            };
            match available.iter().position(|&b| b == delim) {
                Some(i) => {
                    buf.extend_from_slice(&available[..i + 1]);
                    (true, i + 1)
                }
                None => {
                    buf.extend_from_slice(available);
                    (false, available.len())
                }
            }
        };
        r.consume(used);
        read += used;
        if done || used == 0 {
            return Ok(read);
        }
    }
}

/// A `BufRead` is a type of `Read`er which has an internal buffer.
pub trait BufRead: Read {
    /// Returns the contents of the internal buffer, filling it with more data
    /// from the inner reader if it is empty.
    fn fill_buf(&mut self) -> Result<&[u8]>;

    /// Tells this buffer that `amt` bytes have been consumed from the buffer.
    fn consume(&mut self, amt: usize);

    /// Checks if the underlying `Read` has any data left to be read.
    fn has_data_left(&mut self) -> Result<bool> {
        self.fill_buf().map(|b| !b.is_empty())
    }

    /// Read all bytes into `buf` until the delimiter `byte` or EOF is reached.
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        read_until(self, byte, buf)
    }

    /// Skips all bytes until the delimiter `byte` or EOF is reached.
    fn skip_until(&mut self, byte: u8) -> Result<usize> {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = self.fill_buf()?;
                match available.iter().position(|&b| b == byte) {
                    Some(i) => (true, i + 1),
                    None => (false, available.len()),
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                return Ok(read);
            }
        }
    }

    /// Read all bytes until a newline (the `0xA` byte) is reached, and append
    /// them to the provided `String` buffer.
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |b| read_until(self, b'\n', b))
    }

    /// Returns an iterator over the contents of this reader split on the byte `byte`.
    fn split(self, byte: u8) -> Split<Self>
    where
        Self: Sized,
    {
        Split { buf: self, delim: byte }
    }

    /// Returns an iterator over the lines of this reader.
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { buf: self }
    }
}

/// An iterator over the lines of an instance of `BufRead`.
#[derive(Debug)]
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

/// An iterator over the contents of an instance of `BufRead` split on a particular byte.
#[derive(Debug)]
pub struct Split<B> {
    buf: B,
    delim: u8,
}

impl<B: BufRead> Iterator for Split<B> {
    type Item = Result<Vec<u8>>;
    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.read_until(self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf[buf.len() - 1] == self.delim {
                    buf.pop();
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapters: Bytes, Chain, Take

/// An iterator over `u8` values of a reader.
#[derive(Debug)]
pub struct Bytes<R> {
    inner: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte = [0u8; 1];
        loop {
            return match self.inner.read(&mut byte) {
                Ok(0) => None,
                Ok(..) => Some(Ok(byte[0])),
                Err(ref e) if e.is_interrupted() => continue,
                Err(e) => Some(Err(e)),
            };
        }
    }
}

/// Adapter to chain together two readers.
#[derive(Debug)]
pub struct Chain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> Chain<T, U> {
    /// Consumes the `Chain`, returning the wrapped readers.
    pub fn into_inner(self) -> (T, U) {
        (self.first, self.second)
    }

    /// Gets references to the underlying readers in this `Chain`.
    pub fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }

    /// Gets mutable references to the underlying readers in this `Chain`.
    pub fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.first, &mut self.second)
    }
}

impl<T: Read, U: Read> Read for Chain<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.read(buf)? {
                0 if !buf.is_empty() => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.read(buf)
    }
}

impl<T: BufRead, U: BufRead> BufRead for Chain<T, U> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            match self.first.fill_buf()? {
                buf if buf.is_empty() => self.done_first = true,
                _ => return self.first.fill_buf(),
            }
        }
        self.second.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        if !self.done_first { self.first.consume(amt) } else { self.second.consume(amt) }
    }
}

/// Reader adapter which limits the bytes read from an underlying reader.
#[derive(Debug)]
pub struct Take<T> {
    inner: T,
    len: u64,
    limit: u64,
}

impl<T> Take<T> {
    /// Returns the number of bytes that can be read before this instance will return EOF.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Sets the number of bytes that can be read before this instance will return EOF.
    pub fn set_limit(&mut self, limit: u64) {
        self.len = limit;
        self.limit = limit;
    }

    /// Returns the number of bytes read so far.
    pub fn position(&self) -> u64 {
        self.len - self.limit
    }

    /// Consumes the `Take`, returning the wrapped reader.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Read> Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = if (buf.len() as u64) < self.limit { buf.len() } else { self.limit as usize };
        let n = self.inner.read(&mut buf[..max])?;
        assert!(n as u64 <= self.limit, "number of read bytes exceeds limit");
        self.limit -= n as u64;
        Ok(n)
    }
}

impl<T: BufRead> BufRead for Take<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.limit == 0 {
            return Ok(&[]);
        }
        let limit = self.limit;
        let buf = self.inner.fill_buf()?;
        let cap = if (buf.len() as u64) < limit { buf.len() } else { limit as usize };
        Ok(&buf[..cap])
    }
    fn consume(&mut self, amt: usize) {
        let amt = if (amt as u64) < self.limit { amt } else { self.limit as usize };
        self.limit -= amt as u64;
        self.inner.consume(amt);
    }
}

/// Prelude of commonly used I/O traits.
pub mod prelude {
    pub use super::{BufRead, Read, Seek, Write};
}
