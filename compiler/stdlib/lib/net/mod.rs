//! Networking primitives for TCP/UDP communication.

mod addr;
mod ip;
mod parser;
pub(crate) mod socket;
mod tcp;
mod udp;

pub use self::addr::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};
pub use self::ip::{IpAddr, Ipv4Addr, Ipv6Addr, Ipv6MulticastScope};
pub use self::parser::AddrParseError;
pub use self::tcp::{Incoming, IntoIncoming, TcpListener, TcpStream};
pub use self::udp::UdpSocket;

/// Possible values which can be passed to the `TcpStream::shutdown` method.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Shutdown {
    /// The reading portion of the stream should be shut down.
    Read,
    /// The writing portion of the stream should be shut down.
    Write,
    /// Both the reading and the writing portions of the stream should be shut down.
    Both,
}
