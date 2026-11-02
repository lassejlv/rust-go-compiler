//! Anonymous pipes (`std::io::pipe`).

use super::{IoSlice, IoSliceMut, Read, Result, Write};
use crate::fmt;
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::sys;

/// Read end of an anonymous pipe.
pub struct PipeReader(OwnedFd);

/// Write end of an anonymous pipe.
pub struct PipeWriter(OwnedFd);

/// Creates an anonymous pipe.
pub fn pipe() -> Result<(PipeReader, PipeWriter)> {
    let mut fds = [0i32; 2];
    sys::cvt(unsafe { sys::c_pipe2(fds.as_mut_ptr(), sys::O_CLOEXEC) })?;
    unsafe { Ok((PipeReader(OwnedFd::from_raw_fd(fds[0])), PipeWriter(OwnedFd::from_raw_fd(fds[1])))) }
}

impl PipeReader {
    /// Creates a new `PipeReader` instance that shares the same underlying file description.
    pub fn try_clone(&self) -> Result<PipeReader> {
        Ok(PipeReader(self.0.try_clone()?))
    }
}

impl PipeWriter {
    /// Creates a new `PipeWriter` instance that shares the same underlying file description.
    pub fn try_clone(&self) -> Result<PipeWriter> {
        Ok(PipeWriter(self.0.try_clone()?))
    }
}

impl Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.0.read_vectored(bufs)
    }
}

impl Read for &PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.0.write_vectored(bufs)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &PipeWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

macro_rules! pipe_fd_impls {
    ($($t:ident),*) => {
        $(
            impl AsRawFd for $t {
                fn as_raw_fd(&self) -> RawFd {
                    self.0.raw()
                }
            }
            impl AsFd for $t {
                fn as_fd(&self) -> BorrowedFd<'_> {
                    self.0.as_fd()
                }
            }
            impl IntoRawFd for $t {
                fn into_raw_fd(self) -> RawFd {
                    self.0.into_raw_fd()
                }
            }
            impl FromRawFd for $t {
                unsafe fn from_raw_fd(fd: RawFd) -> $t {
                    $t(OwnedFd::from_raw_fd(fd))
                }
            }
            impl From<$t> for OwnedFd {
                fn from(p: $t) -> OwnedFd {
                    p.0
                }
            }
            impl From<OwnedFd> for $t {
                fn from(fd: OwnedFd) -> $t {
                    $t(fd)
                }
            }
            impl From<$t> for crate::process::Stdio {
                fn from(p: $t) -> crate::process::Stdio {
                    crate::process::Stdio::from(p.0)
                }
            }
            impl fmt::Debug for $t {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_tuple(stringify!($t)).field(&self.0).finish()
                }
            }
        )*
    };
}

pipe_fd_impls!(PipeReader, PipeWriter);
