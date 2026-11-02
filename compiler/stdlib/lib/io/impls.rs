//! Forwarding and in-memory implementations of the I/O traits.

use super::{const_io_error, BufRead, ErrorKind, IoSlice, IoSliceMut, Read, Result, Seek, SeekFrom, Write};
use crate::collections::VecDeque;
use crate::fmt;

// ---------------------------------------------------------------------------
// Forwarding impls

impl<R: Read + ?Sized> Read for &mut R {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        (**self).read_vectored(bufs)
    }
    #[inline]
    fn is_read_vectored(&self) -> bool {
        (**self).is_read_vectored()
    }
    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }
    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }
    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        (**self).write_vectored(bufs)
    }
    #[inline]
    fn is_write_vectored(&self) -> bool {
        (**self).is_write_vectored()
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

impl<S: Seek + ?Sized> Seek for &mut S {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
    #[inline]
    fn stream_position(&mut self) -> Result<u64> {
        (**self).stream_position()
    }
}

impl<B: BufRead + ?Sized> BufRead for &mut B {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }
    #[inline]
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }
    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }
    #[inline]
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}

impl<R: Read + ?Sized> Read for Box<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        (**self).read_vectored(bufs)
    }
    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }
    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }
    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}

impl<W: Write + ?Sized> Write for Box<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        (**self).write_vectored(bufs)
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

impl<S: Seek + ?Sized> Seek for Box<S> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
    #[inline]
    fn stream_position(&mut self) -> Result<u64> {
        (**self).stream_position()
    }
}

impl<B: BufRead + ?Sized> BufRead for Box<B> {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }
    #[inline]
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }
    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }
    #[inline]
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}

// ---------------------------------------------------------------------------
// In-memory buffer impls

impl Read for &[u8] {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amt = if buf.len() < self.len() { buf.len() } else { self.len() };
        let (a, b) = self.split_at(amt);
        if amt == 1 {
            buf[0] = a[0];
        } else {
            buf[..amt].copy_from_slice(a);
        }
        *self = b;
        Ok(amt)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let mut nread = 0;
        for buf in bufs.iter_mut() {
            nread += self.read(&mut **buf)?;
            if self.is_empty() {
                break;
            }
        }
        Ok(nread)
    }

    fn is_read_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if buf.len() > self.len() {
            *self = &self[self.len()..];
            return Err(const_io_error(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
        }
        let (a, b) = self.split_at(buf.len());
        buf.copy_from_slice(a);
        *self = b;
        Ok(())
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let len = self.len();
        buf.extend_from_slice(*self);
        *self = &self[len..];
        Ok(len)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        match crate::str::from_utf8(*self) {
            Ok(s) => {
                let len = self.len();
                buf.push_str(s);
                *self = &self[len..];
                Ok(len)
            }
            Err(_) => Err(const_io_error(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
        }
    }
}

impl BufRead for &[u8] {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(*self)
    }
    #[inline]
    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

impl Write for &mut [u8] {
    #[inline]
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let amt = if data.len() < self.len() { data.len() } else { self.len() };
        let (a, b) = crate::mem::take(self).split_at_mut(amt);
        a.copy_from_slice(&data[..amt]);
        *self = b;
        Ok(amt)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let mut nwritten = 0;
        for buf in bufs {
            nwritten += self.write(&**buf)?;
            if self.is_empty() {
                break;
            }
        }
        Ok(nwritten)
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        if self.write(data)? == data.len() {
            Ok(())
        } else {
            Err(const_io_error(ErrorKind::WriteZero, "failed to write whole buffer"))
        }
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Vec<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let mut len = 0;
        for buf in bufs {
            self.extend_from_slice(&**buf);
            len += buf.len();
        }
        Ok(len)
    }

    fn is_write_vectored(&self) -> bool {
        true
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

impl Read for VecDeque<u8> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = {
            let (front, _) = self.as_slices();
            let mut front: &[u8] = front;
            Read::read(&mut front, buf)?
        };
        self.drain(..n);
        Ok(n)
    }
}

impl BufRead for VecDeque<u8> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        let (front, _) = self.as_slices();
        Ok(front)
    }
    fn consume(&mut self, amt: usize) {
        self.drain(..amt);
    }
}

impl Write for VecDeque<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend(buf.iter().copied());
        Ok(buf.len())
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
