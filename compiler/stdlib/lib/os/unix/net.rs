//! Unix-specific networking functionality (Unix domain sockets).

use crate::ffi::OsStr;
use crate::fmt;
use crate::io::{self, IoSlice, IoSliceMut, Read, Write};
use crate::iter::FusedIterator;
use crate::net::socket::Socket;
use crate::net::Shutdown;
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::path::Path;
use crate::sys;
use crate::time::Duration;

const SUN_PATH_OFFSET: usize = 2;
const SUN_PATH_LEN: usize = 108;

/// An address associated with a Unix socket.
#[derive(Clone)]
pub struct SocketAddr {
    // raw `sockaddr_un` bytes (family + path) and the length reported by the kernel
    addr: [u8; 110],
    len: u32,
}

enum AddressKind<'a> {
    Unnamed,
    Pathname(&'a Path),
    Abstract(&'a [u8]),
}

impl SocketAddr {
    fn new_empty() -> SocketAddr {
        SocketAddr { addr: [0; 110], len: 110 }
    }

    fn from_parts(addr: [u8; 110], mut len: u32) -> io::Result<SocketAddr> {
        if len == 0 {
            // When there is a datagram from unnamed unix socket linux returns zero bytes of address
            len = SUN_PATH_OFFSET as u32;
        } else if addr[0] as i32 != sys::AF_UNIX || addr[1] != 0 {
            return Err(io::const_io_error(
                io::ErrorKind::InvalidInput,
                "file descriptor did not correspond to a Unix socket",
            ));
        }
        Ok(SocketAddr { addr, len })
    }

    /// Constructs a `SocketAddr` with the family `AF_UNIX` and the provided path.
    pub fn from_pathname<P: AsRef<Path>>(path: P) -> io::Result<SocketAddr> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        Ok(SocketAddr { addr, len })
    }

    fn address(&self) -> AddressKind<'_> {
        let len = self.len as usize - SUN_PATH_OFFSET;
        let path = &self.addr[SUN_PATH_OFFSET..];
        if len == 0 {
            AddressKind::Unnamed
        } else if path[0] == 0 {
            AddressKind::Abstract(&path[1..len])
        } else {
            AddressKind::Pathname(Path::new(OsStr::from_inner(&path[..len - 1])))
        }
    }

    /// Returns `true` if the address is unnamed.
    pub fn is_unnamed(&self) -> bool {
        match self.address() {
            AddressKind::Unnamed => true,
            _ => false,
        }
    }

    /// Returns the contents of this address if it is a `pathname` address.
    pub fn as_pathname(&self) -> Option<&Path> {
        match self.address() {
            AddressKind::Pathname(p) => Some(p),
            _ => None,
        }
    }

    /// Returns the contents of this address if it is an abstract namespace address (Linux).
    pub fn as_abstract_name(&self) -> Option<&[u8]> {
        match self.address() {
            AddressKind::Abstract(n) => Some(n),
            _ => None,
        }
    }

    /// Creates a Unix socket address in the abstract namespace (Linux).
    pub fn from_abstract_name<N: AsRef<[u8]>>(name: N) -> io::Result<SocketAddr> {
        let name = name.as_ref();
        if name.len() + 1 > SUN_PATH_LEN {
            return Err(io::const_io_error(io::ErrorKind::InvalidInput, "abstract socket name must be shorter than SUN_LEN"));
        }
        let mut addr = [0u8; 110];
        addr[0] = sys::AF_UNIX as u8;
        addr[SUN_PATH_OFFSET + 1..SUN_PATH_OFFSET + 1 + name.len()].copy_from_slice(name);
        Ok(SocketAddr { addr, len: (SUN_PATH_OFFSET + 1 + name.len()) as u32 })
    }
}

impl fmt::Debug for SocketAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.address() {
            AddressKind::Unnamed => write!(fmt, "(unnamed)"),
            AddressKind::Abstract(name) => write!(fmt, "\"{}\" (abstract)", name.escape_ascii()),
            AddressKind::Pathname(path) => write!(fmt, "{:?} (pathname)", path),
        }
    }
}

fn sockaddr_un(path: &Path) -> io::Result<([u8; 110], u32)> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.contains(&0) {
        return Err(io::const_io_error(io::ErrorKind::InvalidInput, "paths must not contain interior null bytes"));
    }
    if bytes.len() >= SUN_PATH_LEN {
        return Err(io::const_io_error(io::ErrorKind::InvalidInput, "path must be shorter than SUN_LEN"));
    }
    let mut addr = [0u8; 110];
    addr[0] = sys::AF_UNIX as u8;
    addr[SUN_PATH_OFFSET..SUN_PATH_OFFSET + bytes.len()].copy_from_slice(bytes);
    let mut len = SUN_PATH_OFFSET + bytes.len();
    match bytes.first() {
        Some(&0) | None => {}
        Some(_) => len += 1,
    }
    Ok((addr, len as u32))
}

fn local_addr_of(s: &Socket, peer: bool) -> io::Result<SocketAddr> {
    let mut a = SocketAddr::new_empty();
    let mut len: u32 = 110;
    s.name_raw(peer, a.addr.as_mut_ptr(), &mut len)?;
    SocketAddr::from_parts(a.addr, len)
}

// ---------------------------------------------------------------------------
// UnixStream

/// A Unix stream socket.
pub struct UnixStream(Socket);

impl UnixStream {
    /// Connects to the socket named by `path`.
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_STREAM)?;
        inner.connect_raw(addr.as_ptr(), len)?;
        Ok(UnixStream(inner))
    }

    /// Connects to the socket specified by `address`.
    pub fn connect_addr(socket_addr: &SocketAddr) -> io::Result<UnixStream> {
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_STREAM)?;
        inner.connect_raw(socket_addr.addr.as_ptr(), socket_addr.len)?;
        Ok(UnixStream(inner))
    }

    /// Creates an unnamed pair of connected sockets.
    pub fn pair() -> io::Result<(UnixStream, UnixStream)> {
        let (i1, i2) = Socket::new_pair(sys::AF_UNIX, sys::SOCK_STREAM)?;
        Ok((UnixStream(i1), UnixStream(i2)))
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<UnixStream> {
        Ok(UnixStream(self.0.duplicate()?))
    }

    /// Returns the socket address of the local half of this connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        local_addr_of(&self.0, false)
    }

    /// Returns the socket address of the remote half of this connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        local_addr_of(&self.0, true)
    }

    /// Sets the read timeout for the socket.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(timeout, sys::SO_RCVTIMEO)
    }

    /// Sets the write timeout for the socket.
    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(timeout, sys::SO_SNDTIMEO)
    }

    /// Returns the read timeout of this socket.
    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_RCVTIMEO)
    }

    /// Returns the write timeout of this socket.
    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_SNDTIMEO)
    }

    /// Moves the socket into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Shuts down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }

    /// Receives data on the socket from the remote address without removing it from the queue.
    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.peek(buf)
    }
}

impl fmt::Debug for UnixStream {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = fmt.debug_struct("UnixStream");
        builder.field("fd", &self.0.raw());
        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }
        if let Ok(addr) = self.peer_addr() {
            builder.field("peer", &addr);
        }
        builder.finish()
    }
}

impl Read for UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool {
        true
    }
}

impl Read for &UnixStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
}

impl Write for UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }
    fn is_write_vectored(&self) -> bool {
        true
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for &UnixStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// UnixListener

/// A structure representing a Unix domain socket server.
pub struct UnixListener(Socket);

impl UnixListener {
    /// Creates a new `UnixListener` bound to the specified socket.
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_STREAM)?;
        inner.bind_raw(addr.as_ptr(), len)?;
        inner.listen(128)?;
        Ok(UnixListener(inner))
    }

    /// Creates a new `UnixListener` bound to the specified socket address.
    pub fn bind_addr(socket_addr: &SocketAddr) -> io::Result<UnixListener> {
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_STREAM)?;
        inner.bind_raw(socket_addr.addr.as_ptr(), socket_addr.len)?;
        inner.listen(128)?;
        Ok(UnixListener(inner))
    }

    /// Accepts a new incoming connection to this listener.
    pub fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
        let mut a = SocketAddr::new_empty();
        let mut len: u32 = 110;
        let sock = self.0.accept_raw(a.addr.as_mut_ptr(), &mut len)?;
        let addr = SocketAddr::from_parts(a.addr, len)?;
        Ok((UnixStream(sock), addr))
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<UnixListener> {
        Ok(UnixListener(self.0.duplicate()?))
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        local_addr_of(&self.0, false)
    }

    /// Moves the socket into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Returns an iterator over incoming connections.
    pub fn incoming(&self) -> Incoming<'_> {
        Incoming { listener: self }
    }
}

impl fmt::Debug for UnixListener {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = fmt.debug_struct("UnixListener");
        builder.field("fd", &self.0.raw());
        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }
        builder.finish()
    }
}

/// An iterator over incoming connections to a `UnixListener`.
#[derive(Debug)]
pub struct Incoming<'a> {
    listener: &'a UnixListener,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<UnixStream>;
    fn next(&mut self) -> Option<io::Result<UnixStream>> {
        Some(self.listener.accept().map(|s| s.0))
    }
}

impl<'a> FusedIterator for Incoming<'a> {}

impl<'a> IntoIterator for &'a UnixListener {
    type Item = io::Result<UnixStream>;
    type IntoIter = Incoming<'a>;
    fn into_iter(self) -> Incoming<'a> {
        self.incoming()
    }
}

// ---------------------------------------------------------------------------
// UnixDatagram

/// A Unix datagram socket.
pub struct UnixDatagram(Socket);

impl UnixDatagram {
    /// Creates a Unix datagram socket bound to the given path.
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixDatagram> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_DGRAM)?;
        inner.bind_raw(addr.as_ptr(), len)?;
        Ok(UnixDatagram(inner))
    }

    /// Creates a Unix datagram socket bound to an address.
    pub fn bind_addr(socket_addr: &SocketAddr) -> io::Result<UnixDatagram> {
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_DGRAM)?;
        inner.bind_raw(socket_addr.addr.as_ptr(), socket_addr.len)?;
        Ok(UnixDatagram(inner))
    }

    /// Creates a Unix datagram socket which is not bound to any address.
    pub fn unbound() -> io::Result<UnixDatagram> {
        let inner = Socket::new(sys::AF_UNIX, sys::SOCK_DGRAM)?;
        Ok(UnixDatagram(inner))
    }

    /// Creates an unnamed pair of connected sockets.
    pub fn pair() -> io::Result<(UnixDatagram, UnixDatagram)> {
        let (i1, i2) = Socket::new_pair(sys::AF_UNIX, sys::SOCK_DGRAM)?;
        Ok((UnixDatagram(i1), UnixDatagram(i2)))
    }

    /// Connects the socket to the specified path address.
    pub fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        self.0.connect_raw(addr.as_ptr(), len)
    }

    /// Connects the socket to an address.
    pub fn connect_addr(&self, socket_addr: &SocketAddr) -> io::Result<()> {
        self.0.connect_raw(socket_addr.addr.as_ptr(), socket_addr.len)
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<UnixDatagram> {
        Ok(UnixDatagram(self.0.duplicate()?))
    }

    /// Returns the address of this socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        local_addr_of(&self.0, false)
    }

    /// Returns the address of this socket's peer.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        local_addr_of(&self.0, true)
    }

    /// Receives data from the socket.
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let mut a = SocketAddr::new_empty();
        let mut len: u32 = 110;
        let n = self.0.recv_from_raw(buf, 0, a.addr.as_mut_ptr(), &mut len)?;
        Ok((n, SocketAddr::from_parts(a.addr, len)?))
    }

    /// Receives data from the socket.
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    /// Sends data on the socket to the specified address.
    pub fn send_to<P: AsRef<Path>>(&self, buf: &[u8], path: P) -> io::Result<usize> {
        let (addr, len) = sockaddr_un(path.as_ref())?;
        self.0.send_to_raw(buf, addr.as_ptr(), len)
    }

    /// Sends data on the socket to the specified `SocketAddr`.
    pub fn send_to_addr(&self, buf: &[u8], socket_addr: &SocketAddr) -> io::Result<usize> {
        self.0.send_to_raw(buf, socket_addr.addr.as_ptr(), socket_addr.len)
    }

    /// Sends data on the socket to the socket's peer.
    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    /// Sets the read timeout for the socket.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(timeout, sys::SO_RCVTIMEO)
    }

    /// Sets the write timeout for the socket.
    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(timeout, sys::SO_SNDTIMEO)
    }

    /// Returns the read timeout of this socket.
    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_RCVTIMEO)
    }

    /// Returns the write timeout of this socket.
    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_SNDTIMEO)
    }

    /// Moves the socket into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Shut down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }

    /// Receives data on the socket from the remote address without removing it from the queue.
    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.peek(buf)
    }
}

impl fmt::Debug for UnixDatagram {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = fmt.debug_struct("UnixDatagram");
        builder.field("fd", &self.0.raw());
        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }
        if let Ok(addr) = self.peer_addr() {
            builder.field("peer", &addr);
        }
        builder.finish()
    }
}

macro_rules! socket_fd_impls {
    ($($t:ident),*) => {
        $(
            impl AsRawFd for $t {
                #[inline]
                fn as_raw_fd(&self) -> RawFd {
                    self.0.raw()
                }
            }

            impl AsFd for $t {
                #[inline]
                fn as_fd(&self) -> BorrowedFd<'_> {
                    self.0.as_fd()
                }
            }

            impl IntoRawFd for $t {
                #[inline]
                fn into_raw_fd(self) -> RawFd {
                    self.0.into_raw_fd()
                }
            }

            impl FromRawFd for $t {
                #[inline]
                unsafe fn from_raw_fd(fd: RawFd) -> $t {
                    $t(Socket::from_raw_fd(fd))
                }
            }

            impl From<$t> for OwnedFd {
                #[inline]
                fn from(s: $t) -> OwnedFd {
                    s.0.into_inner()
                }
            }

            impl From<OwnedFd> for $t {
                #[inline]
                fn from(fd: OwnedFd) -> $t {
                    $t(Socket::from_inner(fd))
                }
            }
        )*
    };
}

socket_fd_impls!(UnixStream, UnixListener, UnixDatagram);
