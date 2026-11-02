//! UDP sockets.

use super::addr::each_addr;
use super::socket::Socket;
use super::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use crate::fmt;
use crate::io;
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::sys;
use crate::time::Duration;

/// A UDP socket.
pub struct UdpSocket(Socket);

impl UdpSocket {
    /// Creates a UDP socket from the given address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
        each_addr(addr, |a| {
            let sock = Socket::new_for(a, sys::SOCK_DGRAM)?;
            sock.bind(a)?;
            Ok(UdpSocket(sock))
        })
    }

    /// Receives a single datagram message on the socket.
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, 0)
    }

    /// Receives a single datagram message on the socket, without removing it from the queue.
    pub fn peek_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, sys::MSG_PEEK)
    }

    /// Sends data on the socket to the given address.
    pub fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
        match addr.to_socket_addrs()?.next() {
            Some(addr) => self.0.send_to(buf, &addr),
            None => Err(io::const_io_error(io::ErrorKind::InvalidInput, "no addresses to send data to")),
        }
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Returns the socket address that this socket was created from.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<UdpSocket> {
        Ok(UdpSocket(self.0.duplicate()?))
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

    /// Sets the value of the `SO_BROADCAST` option for this socket.
    pub fn set_broadcast(&self, broadcast: bool) -> io::Result<()> {
        self.0.setsockopt(sys::SOL_SOCKET, sys::SO_BROADCAST, broadcast as i32)
    }

    /// Gets the value of the `SO_BROADCAST` option for this socket.
    pub fn broadcast(&self) -> io::Result<bool> {
        let raw: i32 = self.0.getsockopt(sys::SOL_SOCKET, sys::SO_BROADCAST)?;
        Ok(raw != 0)
    }

    /// Sets the value of the `IP_MULTICAST_LOOP` option for this socket.
    pub fn set_multicast_loop_v4(&self, on: bool) -> io::Result<()> {
        self.0.setsockopt(sys::IPPROTO_IP, sys::IP_MULTICAST_LOOP, on as u8)
    }

    /// Sets the value of the `IP_MULTICAST_TTL` option for this socket.
    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> io::Result<()> {
        self.0.setsockopt(sys::IPPROTO_IP, sys::IP_MULTICAST_TTL, ttl as u8)
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

    /// Executes an operation of the `IP_ADD_MEMBERSHIP` type (unsupported).
    pub fn join_multicast_v4(&self, _multiaddr: &Ipv4Addr, _interface: &Ipv4Addr) -> io::Result<()> {
        Err(io::const_io_error(io::ErrorKind::Unsupported, "operation not supported"))
    }

    /// Executes an operation of the `IPV6_ADD_MEMBERSHIP` type (unsupported).
    pub fn join_multicast_v6(&self, _multiaddr: &Ipv6Addr, _interface: u32) -> io::Result<()> {
        Err(io::const_io_error(io::ErrorKind::Unsupported, "operation not supported"))
    }

    /// Gets the value of the `SO_ERROR` option on this socket.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// Connects this UDP socket to a remote address.
    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        each_addr(addr, |addr| self.0.connect(addr))
    }

    /// Sends data on the socket to the remote address to which it is connected.
    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    /// Receives a single datagram message on the socket from the remote address to which it is connected.
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    /// Receives single datagram on the socket from the remote address to which it is connected, without removing the message from input queue.
    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.peek(buf)
    }

    /// Moves this UDP socket into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("UdpSocket");
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

socket_fd_impls!(UdpSocket);
