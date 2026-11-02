//! Buffering wrappers for I/O traits.

use super::{BufRead, Error, ErrorKind, IoSlice, IoSliceMut, Read, Result, Seek, SeekFrom, Write, DEFAULT_BUF_SIZE};
use crate::error;
use crate::fmt;

// ---------------------------------------------------------------------------
// BufReader

/// The `BufReader<R>` struct adds buffering to any reader.
pub struct BufReader<R> {
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    inner: R,
}

impl<R: Read> BufReader<R> {
    /// Creates a new `BufReader<R>` with a default buffer capacity (8 KiB).
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReader<R>` with the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {
        let mut buf = Vec::with_capacity(capacity);
        buf.resize(capacity, 0);
        BufReader { inner, buf, pos: 0, filled: 0 }
    }
}

impl<R> BufReader<R> {
    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns a reference to the internally buffered data.
    pub fn buffer(&self) -> &[u8] {
        &self.buf[self.pos..self.filled]
    }

    /// Returns the number of bytes the internal buffer can hold at once.
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Unwraps this `BufReader<R>`, returning the underlying reader (buffered data is lost).
    pub fn into_inner(self) -> R {
        self.inner
    }

    fn discard_buffer(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }
}

impl<R: Seek> BufReader<R> {
    /// Seeks relative to the current position (keeping the buffer when possible).
    pub fn seek_relative(&mut self, offset: i64) -> Result<()> {
        let pos = self.pos as u64;
        if offset < 0 {
            if let Some(_) = pos.checked_sub(offset.unsigned_abs()) {
                self.pos = (pos - offset.unsigned_abs()) as usize;
                return Ok(());
            }
        } else if let Some(new_pos) = pos.checked_add(offset as u64) {
            if new_pos <= self.filled as u64 {
                self.pos = new_pos as usize;
                return Ok(());
            }
        }
        self.seek(SeekFrom::Current(offset)).map(|_| ())
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        if self.pos == self.filled && out.len() >= self.buf.len() {
            self.discard_buffer();
            return self.inner.read(out);
        }
        let n = {
            let available = self.fill_buf()?;
            let n = if out.len() < available.len() { out.len() } else { available.len() };
            out[..n].copy_from_slice(&available[..n]);
            n
        };
        self.consume(n);
        Ok(n)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        for b in bufs.iter_mut() {
            if !b.is_empty() {
                return self.read(&mut **b);
            }
        }
        Ok(0)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let nread = self.filled - self.pos;
        buf.extend_from_slice(&self.buf[self.pos..self.filled]);
        self.discard_buffer();
        Ok(nread + self.inner.read_to_end(buf)?)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        let mut bytes = Vec::new();
        self.read_to_end(&mut bytes)?;
        match String::from_utf8(bytes) {
            Ok(s) => {
                let n = s.len();
                buf.push_str(&s);
                Ok(n)
            }
            Err(_) => Err(super::const_io_error(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
        }
    }
}

impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.pos >= self.filled {
            let n = self.inner.read(&mut self.buf)?;
            self.pos = 0;
            self.filled = n;
        }
        Ok(&self.buf[self.pos..self.filled])
    }
    fn consume(&mut self, amt: usize) {
        self.pos += amt;
        if self.pos > self.filled {
            self.pos = self.filled;
        }
    }
}

impl<R: Seek> Seek for BufReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let result: u64;
        if let SeekFrom::Current(n) = pos {
            let remainder = (self.filled - self.pos) as i64;
            if let Some(offset) = n.checked_sub(remainder) {
                result = self.inner.seek(SeekFrom::Current(offset))?;
            } else {
                self.inner.seek(SeekFrom::Current(-remainder))?;
                self.discard_buffer();
                result = self.inner.seek(SeekFrom::Current(n))?;
            }
        } else {
            result = self.inner.seek(pos)?;
        }
        self.discard_buffer();
        Ok(result)
    }

    fn stream_position(&mut self) -> Result<u64> {
        let remainder = (self.filled - self.pos) as u64;
        self.inner.stream_position().map(|pos| pos - remainder)
    }
}

impl<R: fmt::Debug> fmt::Debug for BufReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("reader", &self.inner)
            .field("buffer", &format_args!("{}/{}", self.filled - self.pos, self.buf.len()))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// BufWriter

/// Wraps a writer and buffers its output.
pub struct BufWriter<W: Write> {
    inner: Option<W>,
    buf: Vec<u8>,
    panicked: bool,
}

/// Error returned for the buffered data from `BufWriter::into_parts`, when
/// the underlying writer has previously panicked.
pub struct WriterPanicked {
    buf: Vec<u8>,
}

impl WriterPanicked {
    /// Returns the perhaps-unwritten data.
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

impl fmt::Display for WriterPanicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BufWriter inner writer panicked, what data remains unwritten is not known")
    }
}

impl fmt::Debug for WriterPanicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriterPanicked").field("buffer", &format_args!("{}/{}", self.buf.len(), self.buf.capacity())).finish()
    }
}

impl error::Error for WriterPanicked {}

impl<W: Write> BufWriter<W> {
    /// Creates a new `BufWriter<W>` with a default buffer capacity (8 KiB).
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufWriter<W>` with at least the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter { inner: Some(inner), buf: Vec::with_capacity(capacity), panicked: false }
    }

    fn flush_buf(&mut self) -> Result<()> {
        let mut written = 0;
        let len = self.buf.len();
        let mut ret = Ok(());
        while written < len {
            self.panicked = true;
            let r = self.inner.as_mut().unwrap().write(&self.buf[written..]);
            self.panicked = false;
            match r {
                Ok(0) => {
                    ret = Err(super::const_io_error(ErrorKind::WriteZero, "failed to write the buffered data"));
                    break;
                }
                Ok(n) => written += n,
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => {
                    ret = Err(e);
                    break;
                }
            }
        }
        if written > 0 {
            self.buf.drain(..written);
        }
        ret
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

    /// Returns the number of bytes the internal buffer can hold without flushing.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Unwraps this `BufWriter<W>`, returning the underlying writer (the
    /// buffer is written out before returning the writer).
    pub fn into_inner(mut self) -> crate::result::Result<W, IntoInnerError<BufWriter<W>>> {
        match self.flush_buf() {
            Err(e) => Err(IntoInnerError(self, e)),
            Ok(()) => Ok(self.inner.take().unwrap()),
        }
    }

    /// Disassembles this `BufWriter<W>`, returning the underlying writer, and
    /// any buffered but unwritten data.
    pub fn into_parts(mut self) -> (W, crate::result::Result<Vec<u8>, WriterPanicked>) {
        let buf = crate::mem::take(&mut self.buf);
        let buf = if !self.panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };
        let inner = self.inner.take().unwrap();
        (inner, buf)
    }
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write(buf);
            self.panicked = false;
            r
        } else {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write_all(buf);
            self.panicked = false;
            r
        } else {
            self.buf.extend_from_slice(buf);
            Ok(())
        }
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let mut total = 0usize;
        for b in bufs {
            total += b.len();
        }
        if self.buf.len() + total > self.buf.capacity() {
            self.flush_buf()?;
        }
        if total >= self.buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write_vectored(bufs);
            self.panicked = false;
            r
        } else {
            for b in bufs {
                self.buf.extend_from_slice(&**b);
            }
            Ok(total)
        }
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_buf()?;
        self.get_mut().flush()
    }
}

impl<W: Write + Seek> Seek for BufWriter<W> {
    /// Seek to the offset, in bytes, in the underlying writer (flushing first).
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.flush_buf()?;
        self.get_mut().seek(pos)
    }
}

impl<W: Write + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufWriter")
            .field("writer", self.inner.as_ref().unwrap())
            .field("buffer", &format_args!("{}/{}", self.buf.len(), self.buf.capacity()))
            .finish()
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            let _ = self.flush_buf();
        }
    }
}

/// An error returned by `BufWriter::into_inner` which combines an error that
/// happened while writing out the buffer, and the buffered writer object.
#[derive(Debug)]
pub struct IntoInnerError<W>(W, Error);

impl<W> IntoInnerError<W> {
    /// Returns the error which caused the call to `into_inner` to fail.
    pub fn error(&self) -> &Error {
        &self.1
    }

    /// Returns the buffered writer instance which generated the error.
    pub fn into_inner(self) -> W {
        self.0
    }

    /// Consumes the `IntoInnerError` and returns the error which caused the call to fail.
    pub fn into_error(self) -> Error {
        self.1
    }

    /// Consumes the `IntoInnerError` and returns the error and the writer.
    pub fn into_parts(self) -> (Error, W) {
        (self.1, self.0)
    }
}

impl<W> From<IntoInnerError<W>> for Error {
    fn from(iie: IntoInnerError<W>) -> Error {
        iie.1
    }
}

impl<W: fmt::Debug> error::Error for IntoInnerError<W> {}

impl<W> fmt::Display for IntoInnerError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.1, f)
    }
}

// ---------------------------------------------------------------------------
// LineWriter

/// Wraps a writer and buffers output to it, flushing whenever a newline is written.
pub struct LineWriter<W: Write> {
    inner: BufWriter<W>,
}

impl<W: Write> LineWriter<W> {
    /// Creates a new `LineWriter` (1 KiB buffer).
    pub fn new(inner: W) -> LineWriter<W> {
        LineWriter::with_capacity(1024, inner)
    }

    /// Creates a new `LineWriter` with at least the specified capacity for the internal buffer.
    pub fn with_capacity(capacity: usize, inner: W) -> LineWriter<W> {
        LineWriter { inner: BufWriter::with_capacity(capacity, inner) }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }

    /// Unwraps this `LineWriter`, returning the underlying writer.
    pub fn into_inner(self) -> crate::result::Result<W, IntoInnerError<LineWriter<W>>> {
        match self.inner.into_inner() {
            Ok(w) => Ok(w),
            Err(e) => {
                let (err, buf) = e.into_parts();
                Err(IntoInnerError(LineWriter { inner: buf }, err))
            }
        }
    }
}

impl<W: Write> Write for LineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match buf.iter().rposition(|&b| b == b'\n') {
            None => {
                // A completed line is still buffered: flush it first.
                if let Some(&b'\n') = self.inner.buffer().last() {
                    self.inner.flush_buf()?;
                }
                self.inner.write(buf)
            }
            Some(i) => {
                let (lines, tail) = buf.split_at(i + 1);
                self.inner.write_all(lines)?;
                self.inner.flush_buf()?;
                if !tail.is_empty() {
                    self.inner.write_all(tail)?;
                }
                Ok(buf.len())
            }
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let mut rest = buf;
        while !rest.is_empty() {
            match self.write(rest) {
                Ok(0) => return Err(super::const_io_error(ErrorKind::WriteZero, "failed to write whole buffer")),
                Ok(n) => rest = &rest[n..],
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W: Write + fmt::Debug> fmt::Debug for LineWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineWriter")
            .field("writer", self.get_ref())
            .field("buffer", &format_args!("{}/{}", self.inner.buf.len(), self.inner.buf.capacity()))
            .finish_non_exhaustive()
    }
}
