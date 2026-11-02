//! TCP sockets.

use super::addr::each_addr;
use super::socket::Socket;
use super::{Shutdown, SocketAddr, ToSocketAddrs};
use crate::fmt;
use crate::io::{self, IoSlice, IoSliceMut, Read, Write};
use crate::iter::FusedIterator;
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::sys;
use crate::time::Duration;

/// A TCP stream between a local and a remote socket.
pub struct TcpStream(Socket);

/// A TCP socket server, listening for connections.
pub struct TcpListener(Socket);

/// An iterator that infinitely `accept`s connections on a `TcpListener`.
#[derive(Debug)]
pub struct Incoming<'a> {
    listener: &'a TcpListener,
}

/// An iterator that infinitely `accept`s connections on a `TcpListener` (owning).
#[derive(Debug)]
pub struct IntoIncoming {
    listener: TcpListener,
}

impl TcpStream {
    /// Opens a TCP connection to a remote host.
    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        each_addr(addr, |a| {
            let sock = Socket::new_for(a, sys::SOCK_STREAM)?;
            sock.connect(a)?;
            Ok(TcpStream(sock))
        })
    }

    /// Opens a TCP connection to a remote host with a timeout.
    pub fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> io::Result<TcpStream> {
        let sock = Socket::new_for(addr, sys::SOCK_STREAM)?;
        sock.connect_timeout(addr, timeout)?;
        Ok(TcpStream(sock))
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<TcpStream> {
        Ok(TcpStream(self.0.duplicate()?))
    }

    /// Sets the read timeout to the timeout specified.
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(dur, sys::SO_RCVTIMEO)
    }

    /// Sets the write timeout to the timeout specified.
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(dur, sys::SO_SNDTIMEO)
    }

    /// Returns the read timeout of this socket.
    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_RCVTIMEO)
    }

    /// Returns the write timeout of this socket.
    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(sys::SO_SNDTIMEO)
    }

    /// Receives data on the socket from the remote address without removing it from the queue.
    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.peek(buf)
    }

    /// Sets the value of the `SO_LINGER` option on this socket.
    pub fn set_linger(&self, linger: Option<Duration>) -> io::Result<()> {
        let l = sys::linger_t {
            l_onoff: linger.is_some() as i32,
            l_linger: linger.map(|d| d.as_secs() as i32).unwrap_or(0),
        };
        self.0.setsockopt(sys::SOL_SOCKET, sys::SO_LINGER, l)
    }

    /// Gets the value of the `SO_LINGER` option on this socket.
    pub fn linger(&self) -> io::Result<Option<Duration>> {
        let l: sys::linger_t = self.0.getsockopt(sys::SOL_SOCKET, sys::SO_LINGER)?;
        Ok(if l.l_onoff != 0 { Some(Duration::from_secs(l.l_linger as u64)) } else { None })
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.0.set_nodelay(nodelay)
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    pub fn nodelay(&self) -> io::Result<bool> {
        self.0.nodelay()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.setsockopt(sys::IPPROTO_IP, sys::IP_TTL, ttl as i32)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    pub fn ttl(&self) -> io::Result<u32> {
        let raw: i32 = self.0.getsockopt(sys::IPPROTO_IP, sys::IP_TTL)?;
        Ok(raw as u32)
    }

    /// Gets the value of the `SO_ERROR` option on this socket.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Moves this TCP stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }
}

impl Read for TcpStream {
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

impl Write for TcpStream {
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

impl Read for &TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
}

impl Write for &TcpStream {
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

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("TcpStream");
        if let Ok(addr) = self.local_addr() {
            res.field("addr", &addr);
        }
        if let Ok(peer) = self.peer_addr() {
            res.field("peer", &peer);
        }
        res.field("fd", &self.0.raw()).finish()
    }
}

impl TcpListener {
    /// Creates a new `TcpListener` which will be bound to the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        each_addr(addr, |a| {
            let sock = Socket::new_for(a, sys::SOCK_STREAM)?;
            sock.setsockopt(sys::SOL_SOCKET, sys::SO_REUSEADDR, 1i32)?;
            sock.bind(a)?;
            sock.listen(128)?;
            Ok(TcpListener(sock))
        })
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<TcpListener> {
        Ok(TcpListener(self.0.duplicate()?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (sock, addr) = self.0.accept()?;
        Ok((TcpStream(sock), addr))
    }

    /// Returns an iterator over the connections being received on this listener.
    pub fn incoming(&self) -> Incoming<'_> {
        Incoming { listener: self }
    }

    /// Turn this into an iterator over the connections being received on this listener.
    pub fn into_incoming(self) -> IntoIncoming {
        IntoIncoming { listener: self }
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.setsockopt(sys::IPPROTO_IP, sys::IP_TTL, ttl as i32)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    pub fn ttl(&self) -> io::Result<u32> {
        let raw: i32 = self.0.getsockopt(sys::IPPROTO_IP, sys::IP_TTL)?;
        Ok(raw as u32)
    }

    /// Gets the value of the `SO_ERROR` option on this socket.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Moves this TCP stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }
}

impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<TcpStream>;
    fn next(&mut self) -> Option<io::Result<TcpStream>> {
        Some(self.listener.accept().map(|p| p.0))
    }
}

impl<'a> FusedIterator for Incoming<'a> {}

impl Iterator for IntoIncoming {
    type Item = io::Result<TcpStream>;
    fn next(&mut self) -> Option<io::Result<TcpStream>> {
        Some(self.listener.accept().map(|p| p.0))
    }
}

impl FusedIterator for IntoIncoming {}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("TcpListener");
        if let Ok(addr) = self.local_addr() {
            res.field("addr", &addr);
        }
        res.field("fd", &self.0.raw()).finish()
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

socket_fd_impls!(TcpStream, TcpListener);
