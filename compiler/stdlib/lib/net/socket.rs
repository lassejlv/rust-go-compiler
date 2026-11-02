//! The shared socket implementation (BSD sockets via the C library).

use super::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6};
use crate::io::{self, IoSlice, IoSliceMut};
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::sys;
use crate::time::Duration;

/// `struct sockaddr_storage` (128 bytes, 8-aligned).
#[repr(C)]
pub(crate) struct SockAddrStorage {
    pub family: u16,
    pub data: [u8; 126],
    _align: [u64; 0],
}

impl SockAddrStorage {
    pub(crate) fn zeroed() -> SockAddrStorage {
        SockAddrStorage { family: 0, data: [0; 126], _align: [] }
    }
}

/// Encodes an address as a C `sockaddr_in`/`sockaddr_in6`.
pub(crate) fn addr_to_raw(addr: &SocketAddr) -> (SockAddrStorage, u32) {
    let mut st = SockAddrStorage::zeroed();
    match addr {
        SocketAddr::V4(a) => {
            st.family = sys::AF_INET as u16;
            let p = a.port().to_be_bytes();
            st.data[0] = p[0];
            st.data[1] = p[1];
            let o = a.ip().octets();
            st.data[2..6].copy_from_slice(&o);
            (st, 16)
        }
        SocketAddr::V6(a) => {
            st.family = sys::AF_INET6 as u16;
            let p = a.port().to_be_bytes();
            st.data[0] = p[0];
            st.data[1] = p[1];
            st.data[2..6].copy_from_slice(&a.flowinfo().to_ne_bytes());
            st.data[6..22].copy_from_slice(&a.ip().octets());
            st.data[22..26].copy_from_slice(&a.scope_id().to_ne_bytes());
            (st, 28)
        }
    }
}

/// Decodes a C socket address.
pub(crate) unsafe fn sockaddr_to_addr(p: *const u8, len: usize) -> io::Result<SocketAddr> {
    let family = *(p as *const u16) as i32;
    let b = crate::slice::from_raw_parts(p, len);
    if family == sys::AF_INET && len >= 16 {
        let port = u16::from_be_bytes([b[2], b[3]]);
        let ip = Ipv4Addr::new(b[4], b[5], b[6], b[7]);
        Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
    } else if family == sys::AF_INET6 && len >= 28 {
        let port = u16::from_be_bytes([b[2], b[3]]);
        let flowinfo = u32::from_ne_bytes([b[4], b[5], b[6], b[7]]);
        let mut o = [0u8; 16];
        o.copy_from_slice(&b[8..24]);
        let scope_id = u32::from_ne_bytes([b[24], b[25], b[26], b[27]]);
        Ok(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(o), port, flowinfo, scope_id)))
    } else {
        Err(io::const_io_error(io::ErrorKind::InvalidInput, "invalid argument"))
    }
}

/// An owned socket.
pub(crate) struct Socket(OwnedFd);

impl Socket {
    pub(crate) fn new(family: i32, ty: i32) -> io::Result<Socket> {
        let fd = sys::cvt(unsafe { sys::c_socket(family, ty | sys::SOCK_CLOEXEC, 0) })?;
        Ok(Socket(unsafe { OwnedFd::from_raw_fd(fd) }))
    }

    pub(crate) fn new_for(addr: &SocketAddr, ty: i32) -> io::Result<Socket> {
        let family = match *addr {
            SocketAddr::V4(..) => sys::AF_INET,
            SocketAddr::V6(..) => sys::AF_INET6,
        };
        Socket::new(family, ty)
    }

    pub(crate) fn new_pair(family: i32, ty: i32) -> io::Result<(Socket, Socket)> {
        let mut fds = [0i32; 2];
        sys::cvt(unsafe { sys::c_socketpair(family, ty | sys::SOCK_CLOEXEC, 0, fds.as_mut_ptr()) })?;
        Ok(unsafe { (Socket(OwnedFd::from_raw_fd(fds[0])), Socket(OwnedFd::from_raw_fd(fds[1]))) })
    }

    #[inline]
    pub(crate) fn raw(&self) -> RawFd {
        self.0.raw()
    }

    pub(crate) fn connect_raw(&self, addr: *const u8, len: u32) -> io::Result<()> {
        loop {
            let r = unsafe { sys::c_connect(self.raw(), addr, len) };
            if r == -1 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            return Ok(());
        }
    }

    pub(crate) fn connect(&self, addr: &SocketAddr) -> io::Result<()> {
        let (st, len) = addr_to_raw(addr);
        self.connect_raw(&st as *const SockAddrStorage as *const u8, len)
    }

    pub(crate) fn connect_timeout(&self, addr: &SocketAddr, timeout: Duration) -> io::Result<()> {
        self.set_nonblocking(true)?;
        let r = self.connect(addr);
        self.set_nonblocking(false)?;
        match r {
            Ok(()) => return Ok(()),
            Err(ref e) if e.raw_os_error() == Some(sys::EINPROGRESS) => {}
            Err(e) => return Err(e),
        }
        if timeout.as_secs() == 0 && timeout.subsec_nanos() == 0 {
            return Err(io::const_io_error(io::ErrorKind::InvalidInput, "cannot set a 0 duration timeout"));
        }
        let mut pfd = sys::pollfd_t { fd: self.raw(), events: sys::POLLOUT, revents: 0 };
        let ms = timeout.as_millis();
        let ms = if ms > i32::MAX as u128 { i32::MAX } else if ms == 0 { 1 } else { ms as i32 };
        loop {
            let r = unsafe { sys::c_poll(&mut pfd, 1, ms) };
            if r == -1 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            if r == 0 {
                return Err(io::const_io_error(io::ErrorKind::TimedOut, "connection timed out"));
            }
            break;
        }
        match self.take_error()? {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub(crate) fn bind_raw(&self, addr: *const u8, len: u32) -> io::Result<()> {
        sys::cvt(unsafe { sys::c_bind(self.raw(), addr, len) })?;
        Ok(())
    }

    pub(crate) fn bind(&self, addr: &SocketAddr) -> io::Result<()> {
        let (st, len) = addr_to_raw(addr);
        self.bind_raw(&st as *const SockAddrStorage as *const u8, len)
    }

    pub(crate) fn listen(&self, backlog: i32) -> io::Result<()> {
        sys::cvt(unsafe { sys::c_listen(self.raw(), backlog) })?;
        Ok(())
    }

    pub(crate) fn accept_raw(&self, storage: *mut u8, len: *mut u32) -> io::Result<Socket> {
        loop {
            let fd = unsafe { sys::c_accept4(self.raw(), storage, len, sys::SOCK_CLOEXEC) };
            if fd == -1 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            return Ok(Socket(unsafe { OwnedFd::from_raw_fd(fd) }));
        }
    }

    pub(crate) fn accept(&self) -> io::Result<(Socket, SocketAddr)> {
        let mut st = SockAddrStorage::zeroed();
        let mut len: u32 = 128;
        let s = self.accept_raw(&mut st as *mut SockAddrStorage as *mut u8, &mut len)?;
        let a = unsafe { sockaddr_to_addr(&st as *const SockAddrStorage as *const u8, len as usize)? };
        Ok((s, a))
    }

    pub(crate) fn duplicate(&self) -> io::Result<Socket> {
        Ok(Socket(self.0.try_clone()?))
    }

    pub(crate) fn recv_with_flags(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        loop {
            let r = unsafe { sys::c_recv(self.raw(), buf.as_mut_ptr(), buf.len(), flags) };
            if r < 0 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            return Ok(r as usize);
        }
    }

    pub(crate) fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_with_flags(buf, 0)
    }

    pub(crate) fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_with_flags(buf, sys::MSG_PEEK)
    }

    pub(crate) fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }

    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let r = unsafe { sys::c_send(self.raw(), buf.as_ptr(), buf.len(), sys::MSG_NOSIGNAL) };
            if r < 0 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            return Ok(r as usize);
        }
    }

    pub(crate) fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    pub(crate) fn recv_from_raw(&self, buf: &mut [u8], flags: i32, st: *mut u8, len: *mut u32) -> io::Result<usize> {
        loop {
            let r = unsafe { sys::c_recvfrom(self.raw(), buf.as_mut_ptr(), buf.len(), flags, st, len) };
            if r < 0 {
                let e = sys::errno();
                if e == sys::EINTR {
                    continue;
                }
                return Err(io::Error::from_raw_os_error(e));
            }
            return Ok(r as usize);
        }
    }

    pub(crate) fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, SocketAddr)> {
        let mut st = SockAddrStorage::zeroed();
        let mut len: u32 = 128;
        let n = self.recv_from_raw(buf, flags, &mut st as *mut SockAddrStorage as *mut u8, &mut len)?;
        let a = unsafe { sockaddr_to_addr(&st as *const SockAddrStorage as *const u8, len as usize)? };
        Ok((n, a))
    }

    pub(crate) fn send_to_raw(&self, buf: &[u8], st: *const u8, len: u32) -> io::Result<usize> {
        let r = unsafe { sys::c_sendto(self.raw(), buf.as_ptr(), buf.len(), sys::MSG_NOSIGNAL, st, len) };
        sys::cvt_isize(r).map(|n| n as usize)
    }

    pub(crate) fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> io::Result<usize> {
        let (st, len) = addr_to_raw(addr);
        self.send_to_raw(buf, &st as *const SockAddrStorage as *const u8, len)
    }

    pub(crate) fn name_raw(&self, peer: bool, st: *mut u8, len: *mut u32) -> io::Result<()> {
        let r = unsafe {
            if peer { sys::c_getpeername(self.raw(), st, len) } else { sys::c_getsockname(self.raw(), st, len) }
        };
        sys::cvt(r)?;
        Ok(())
    }

    pub(crate) fn socket_addr(&self) -> io::Result<SocketAddr> {
        let mut st = SockAddrStorage::zeroed();
        let mut len: u32 = 128;
        self.name_raw(false, &mut st as *mut SockAddrStorage as *mut u8, &mut len)?;
        unsafe { sockaddr_to_addr(&st as *const SockAddrStorage as *const u8, len as usize) }
    }

    pub(crate) fn peer_addr(&self) -> io::Result<SocketAddr> {
        let mut st = SockAddrStorage::zeroed();
        let mut len: u32 = 128;
        self.name_raw(true, &mut st as *mut SockAddrStorage as *mut u8, &mut len)?;
        unsafe { sockaddr_to_addr(&st as *const SockAddrStorage as *const u8, len as usize) }
    }

    pub(crate) fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        let how = match how {
            Shutdown::Write => sys::SHUT_WR,
            Shutdown::Read => sys::SHUT_RD,
            Shutdown::Both => sys::SHUT_RDWR,
        };
        sys::cvt(unsafe { sys::c_shutdown(self.raw(), how) })?;
        Ok(())
    }

    pub(crate) fn setsockopt<T>(&self, level: i32, name: i32, val: T) -> io::Result<()> {
        let len = crate::mem::size_of::<T>() as u32;
        sys::cvt(unsafe { sys::c_setsockopt(self.raw(), level, name, &val as *const T as *const u8, len) })?;
        Ok(())
    }

    pub(crate) fn getsockopt<T: Copy>(&self, level: i32, name: i32) -> io::Result<T> {
        unsafe {
            let mut val: T = crate::mem::zeroed();
            let mut len = crate::mem::size_of::<T>() as u32;
            sys::cvt(sys::c_getsockopt(self.raw(), level, name, &mut val as *mut T as *mut u8, &mut len))?;
            Ok(val)
        }
    }

    pub(crate) fn set_timeout(&self, dur: Option<Duration>, kind: i32) -> io::Result<()> {
        let timeout = match dur {
            Some(dur) => {
                if dur.as_secs() == 0 && dur.subsec_nanos() == 0 {
                    return Err(io::const_io_error(io::ErrorKind::InvalidInput, "cannot set a 0 duration timeout"));
                }
                let secs = if dur.as_secs() > i64::MAX as u64 { i64::MAX } else { dur.as_secs() as i64 };
                let mut timeout = sys::timeval_t { tv_sec: secs, tv_usec: (dur.subsec_nanos() / 1000) as i64 };
                if timeout.tv_sec == 0 && timeout.tv_usec == 0 {
                    timeout.tv_usec = 1;
                }
                timeout
            }
            None => sys::timeval_t { tv_sec: 0, tv_usec: 0 },
        };
        self.setsockopt(sys::SOL_SOCKET, kind, timeout)
    }

    pub(crate) fn timeout(&self, kind: i32) -> io::Result<Option<Duration>> {
        let raw: sys::timeval_t = self.getsockopt(sys::SOL_SOCKET, kind)?;
        if raw.tv_sec == 0 && raw.tv_usec == 0 {
            Ok(None)
        } else {
            Ok(Some(Duration::new(raw.tv_sec as u64, (raw.tv_usec as u32) * 1000)))
        }
    }

    pub(crate) fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.setsockopt(sys::IPPROTO_TCP, sys::TCP_NODELAY, nodelay as i32)
    }

    pub(crate) fn nodelay(&self) -> io::Result<bool> {
        let raw: i32 = self.getsockopt(sys::IPPROTO_TCP, sys::TCP_NODELAY)?;
        Ok(raw != 0)
    }

    pub(crate) fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        sys::set_nonblocking(self.raw(), nonblocking)
    }

    pub(crate) fn take_error(&self) -> io::Result<Option<io::Error>> {
        let raw: i32 = self.getsockopt(sys::SOL_SOCKET, sys::SO_ERROR)?;
        if raw == 0 { Ok(None) } else { Ok(Some(io::Error::from_raw_os_error(raw))) }
    }

    pub(crate) fn as_inner(&self) -> &OwnedFd {
        &self.0
    }

    pub(crate) fn into_inner(self) -> OwnedFd {
        self.0
    }

    pub(crate) fn from_inner(fd: OwnedFd) -> Socket {
        Socket(fd)
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.raw()
    }
}

impl AsFd for Socket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: RawFd) -> Socket {
        Socket(OwnedFd::from_raw_fd(fd))
    }
}
