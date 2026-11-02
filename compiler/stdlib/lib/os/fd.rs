//! Owned and borrowed Unix-like file descriptors.

use crate::fmt;
use crate::io;
use crate::marker::PhantomData;
use crate::sys;

/// Raw file descriptors.
pub type RawFd = i32;

/// A trait to extract the raw file descriptor from an underlying object.
pub trait AsRawFd {
    /// Extracts the raw file descriptor.
    fn as_raw_fd(&self) -> RawFd;
}

/// A trait to express the ability to construct an object from a raw file descriptor.
pub trait FromRawFd {
    /// Constructs a new instance of `Self` from the given raw file descriptor.
    unsafe fn from_raw_fd(fd: RawFd) -> Self;
}

/// A trait to express the ability to consume an object and acquire ownership of its raw file descriptor.
pub trait IntoRawFd {
    /// Consumes this object, returning the raw underlying file descriptor.
    fn into_raw_fd(self) -> RawFd;
}

impl AsRawFd for RawFd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        *self
    }
}

impl IntoRawFd for RawFd {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        self
    }
}

impl FromRawFd for RawFd {
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> RawFd {
        fd
    }
}

/// A borrowed file descriptor.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct BorrowedFd<'fd> {
    fd: RawFd,
    _phantom: PhantomData<&'fd OwnedFd>,
}

/// An owned file descriptor (closed on drop).
#[repr(transparent)]
pub struct OwnedFd {
    fd: RawFd,
}

impl<'fd> BorrowedFd<'fd> {
    /// Returns a `BorrowedFd` holding the given raw file descriptor.
    #[inline]
    pub const unsafe fn borrow_raw(fd: RawFd) -> Self {
        BorrowedFd { fd, _phantom: PhantomData }
    }

    /// Creates a new `OwnedFd` instance that shares the same underlying file description as the existing `BorrowedFd` instance.
    pub fn try_clone_to_owned(&self) -> io::Result<OwnedFd> {
        let fd = sys::dup_cloexec(self.fd)?;
        Ok(OwnedFd { fd })
    }
}

impl OwnedFd {
    /// Creates a new `OwnedFd` instance that shares the same underlying file description as the existing `OwnedFd` instance.
    pub fn try_clone(&self) -> io::Result<OwnedFd> {
        self.as_fd().try_clone_to_owned()
    }

    #[inline]
    pub(crate) fn raw(&self) -> RawFd {
        self.fd
    }

    pub(crate) fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        sys::read_fd(self.fd, buf)
    }

    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        sys::write_fd(self.fd, buf)
    }

    pub(crate) fn read_vectored(&self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        let n = if bufs.len() > 1024 { 1024 } else { bufs.len() };
        let r = unsafe { sys::c_readv(self.fd, bufs.as_ptr() as *const sys::iovec_t, n as i32) };
        sys::cvt_isize(r).map(|n| n as usize)
    }

    pub(crate) fn write_vectored(&self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let n = if bufs.len() > 1024 { 1024 } else { bufs.len() };
        if self.fd == 1 {
            sys::flush_stdout();
        }
        let r = unsafe { sys::c_writev(self.fd, bufs.as_ptr() as *const sys::iovec_t, n as i32) };
        sys::cvt_isize(r).map(|n| n as usize)
    }
}

impl Drop for OwnedFd {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = sys::c_close(self.fd);
        }
    }
}

impl<'fd> AsRawFd for BorrowedFd<'fd> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl AsRawFd for OwnedFd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for OwnedFd {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        let fd = self.fd;
        crate::mem::forget(self);
        fd
    }
}

impl FromRawFd for OwnedFd {
    /// Constructs a new instance of `Self` from the given raw file descriptor (which must be open and owned).
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        assert!(fd != -1);
        OwnedFd { fd }
    }
}

impl<'fd> fmt::Debug for BorrowedFd<'fd> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedFd").field("fd", &self.fd).finish()
    }
}

impl fmt::Debug for OwnedFd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedFd").field("fd", &self.fd).finish()
    }
}

/// A trait to borrow the file descriptor from an underlying object.
pub trait AsFd {
    /// Borrows the file descriptor.
    fn as_fd(&self) -> BorrowedFd<'_>;
}

impl<T: AsFd + ?Sized> AsFd for &T {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        T::as_fd(self)
    }
}

impl<T: AsFd + ?Sized> AsFd for &mut T {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        T::as_fd(self)
    }
}

impl<T: AsFd + ?Sized> AsFd for Box<T> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl<T: AsFd + ?Sized> AsFd for crate::rc::Rc<T> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl<T: AsFd + ?Sized> AsFd for crate::sync::Arc<T> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl<T: AsRawFd + ?Sized> AsRawFd for Box<T> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        (**self).as_raw_fd()
    }
}

impl<T: AsRawFd + ?Sized> AsRawFd for crate::rc::Rc<T> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        (**self).as_raw_fd()
    }
}

impl<T: AsRawFd + ?Sized> AsRawFd for crate::sync::Arc<T> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        (**self).as_raw_fd()
    }
}

impl<'fd> AsFd for BorrowedFd<'fd> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        *self
    }
}

impl AsFd for OwnedFd {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

// ---- standard streams

macro_rules! std_stream_fd {
    ($fd:expr, $($t:ty),*) => {
        $(
            impl AsRawFd for $t {
                #[inline]
                fn as_raw_fd(&self) -> RawFd {
                    $fd
                }
            }

            impl AsFd for $t {
                #[inline]
                fn as_fd(&self) -> BorrowedFd<'_> {
                    unsafe { BorrowedFd::borrow_raw($fd) }
                }
            }
        )*
    };
}

std_stream_fd!(0, io::Stdin);
std_stream_fd!(1, io::Stdout);
std_stream_fd!(2, io::Stderr);

impl<'a> AsRawFd for io::StdinLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        0
    }
}

impl<'a> AsFd for io::StdinLock<'a> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(0) }
    }
}

impl<'a> AsRawFd for io::StdoutLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        1
    }
}

impl<'a> AsFd for io::StdoutLock<'a> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(1) }
    }
}

impl<'a> AsRawFd for io::StderrLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        2
    }
}

impl<'a> AsFd for io::StderrLock<'a> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(2) }
    }
}

impl<'fd> io::IsTerminal for BorrowedFd<'fd> {
    fn is_terminal(&self) -> bool {
        io::stdio_is_terminal(self.fd)
    }
}

impl io::IsTerminal for OwnedFd {
    fn is_terminal(&self) -> bool {
        io::stdio_is_terminal(self.fd)
    }
}
