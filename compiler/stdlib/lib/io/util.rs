//! Utility readers and writers.

use super::{BufRead, IoSlice, IoSliceMut, Read, Result, Seek, SeekFrom, Write, DEFAULT_BUF_SIZE};
use crate::fmt;

/// `Empty` ignores any data written via `Write`, and will always be empty
/// (returning zero bytes) when read via `Read`.
#[derive(Clone, Copy, Default)]
pub struct Empty;

/// Creates a value that is always at EOF for reads, and ignores all data written.
pub const fn empty() -> Empty {
    Empty
}

impl Read for Empty {
    #[inline]
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl BufRead for Empty {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&[])
    }
    #[inline]
    fn consume(&mut self, _amt: usize) {}
}

impl Seek for Empty {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }
    fn stream_len(&mut self) -> Result<u64> {
        Ok(0)
    }
    fn stream_position(&mut self) -> Result<u64> {
        Ok(0)
    }
}

impl Write for Empty {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let mut n = 0;
        for b in bufs {
            n += b.len();
        }
        Ok(n)
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &Empty {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Empty")
    }
}

/// A reader which yields one byte over and over and over and over and over and...
pub struct Repeat {
    byte: u8,
}

/// Creates an instance of a reader that infinitely repeats one byte.
pub const fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

impl Read for Repeat {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for slot in &mut *buf {
            *slot = self.byte;
        }
        Ok(buf.len())
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let mut nwritten = 0;
        for buf in bufs {
            nwritten += self.read(&mut **buf)?;
        }
        Ok(nwritten)
    }

    fn read_to_end(&mut self, _: &mut Vec<u8>) -> Result<usize> {
        Err(crate::io::Error::from(crate::io::ErrorKind::OutOfMemory))
    }
}

impl fmt::Debug for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Repeat").finish_non_exhaustive()
    }
}

/// A writer which will move data into the void.
#[derive(Clone, Copy, Default)]
pub struct Sink;

/// Creates an instance of a writer which will successfully consume all data.
pub const fn sink() -> Sink {
    Sink
}

impl Write for Sink {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let mut n = 0;
        for b in bufs {
            n += b.len();
        }
        Ok(n)
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &Sink {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Sink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sink")
    }
}

/// Copies the entire contents of a reader into a writer.
pub fn copy<R: Read + ?Sized, W: Write + ?Sized>(reader: &mut R, writer: &mut W) -> Result<u64> {
    let mut buf = [0u8; DEFAULT_BUF_SIZE];
    let mut total: u64 = 0;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => return Ok(total),
            Ok(n) => n,
            Err(ref e) if e.is_interrupted() => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..n])?;
        total += n as u64;
    }
}
