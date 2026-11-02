//! Socket addresses and name resolution.

use super::{IpAddr, Ipv4Addr, Ipv6Addr};
use crate::fmt::{self, Write};
use crate::io;
use crate::sys;
use crate::vec;

/// An internet socket address, either IPv4 or IPv6.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SocketAddr {
    /// An IPv4 socket address.
    V4(SocketAddrV4),
    /// An IPv6 socket address.
    V6(SocketAddrV6),
}

/// An IPv4 socket address.
#[derive(Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct SocketAddrV4 {
    ip: Ipv4Addr,
    port: u16,
}

/// An IPv6 socket address.
#[derive(Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct SocketAddrV6 {
    ip: Ipv6Addr,
    port: u16,
    flowinfo: u32,
    scope_id: u32,
}

impl SocketAddr {
    /// Creates a new socket address from an IP address and a port number.
    pub fn new(ip: IpAddr, port: u16) -> SocketAddr {
        match ip {
            IpAddr::V4(a) => SocketAddr::V4(SocketAddrV4::new(a, port)),
            IpAddr::V6(a) => SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0)),
        }
    }

    /// Returns the IP address associated with this socket address.
    pub fn ip(&self) -> IpAddr {
        match *self {
            SocketAddr::V4(ref a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(ref a) => IpAddr::V6(*a.ip()),
        }
    }

    /// Changes the IP address associated with this socket address.
    pub fn set_ip(&mut self, new_ip: IpAddr) {
        match (self, new_ip) {
            (&mut SocketAddr::V4(ref mut a), IpAddr::V4(new_ip)) => a.set_ip(new_ip),
            (&mut SocketAddr::V6(ref mut a), IpAddr::V6(new_ip)) => a.set_ip(new_ip),
            (self_, new_ip) => *self_ = SocketAddr::new(new_ip, self_.port()),
        }
    }

    /// Returns the port number associated with this socket address.
    pub fn port(&self) -> u16 {
        match *self {
            SocketAddr::V4(ref a) => a.port(),
            SocketAddr::V6(ref a) => a.port(),
        }
    }

    /// Changes the port number associated with this socket address.
    pub fn set_port(&mut self, new_port: u16) {
        match *self {
            SocketAddr::V4(ref mut a) => a.set_port(new_port),
            SocketAddr::V6(ref mut a) => a.set_port(new_port),
        }
    }

    /// Returns `true` if the IP address in this `SocketAddr` is an IPv4 address.
    pub fn is_ipv4(&self) -> bool {
        match *self {
            SocketAddr::V4(_) => true,
            SocketAddr::V6(_) => false,
        }
    }

    /// Returns `true` if the IP address in this `SocketAddr` is an IPv6 address.
    pub fn is_ipv6(&self) -> bool {
        match *self {
            SocketAddr::V4(_) => false,
            SocketAddr::V6(_) => true,
        }
    }
}

impl SocketAddrV4 {
    /// Creates a new socket address from an IPv4 address and a port number.
    pub const fn new(ip: Ipv4Addr, port: u16) -> SocketAddrV4 {
        SocketAddrV4 { ip, port }
    }

    /// Returns the IP address associated with this socket address.
    pub const fn ip(&self) -> &Ipv4Addr {
        &self.ip
    }

    /// Changes the IP address associated with this socket address.
    pub fn set_ip(&mut self, new_ip: Ipv4Addr) {
        self.ip = new_ip;
    }

    /// Returns the port number associated with this socket address.
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Changes the port number associated with this socket address.
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port;
    }
}

impl SocketAddrV6 {
    /// Creates a new socket address from an IPv6 address, a 16-bit port number,
    /// and the `flowinfo` and `scope_id` fields.
    pub const fn new(ip: Ipv6Addr, port: u16, flowinfo: u32, scope_id: u32) -> SocketAddrV6 {
        SocketAddrV6 { ip, port, flowinfo, scope_id }
    }

    /// Returns the IP address associated with this socket address.
    pub const fn ip(&self) -> &Ipv6Addr {
        &self.ip
    }

    /// Changes the IP address associated with this socket address.
    pub fn set_ip(&mut self, new_ip: Ipv6Addr) {
        self.ip = new_ip;
    }

    /// Returns the port number associated with this socket address.
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Changes the port number associated with this socket address.
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port;
    }

    /// Returns the flow information associated with this address.
    pub const fn flowinfo(&self) -> u32 {
        self.flowinfo
    }

    /// Changes the flow information associated with this socket address.
    pub fn set_flowinfo(&mut self, new_flowinfo: u32) {
        self.flowinfo = new_flowinfo;
    }

    /// Returns the scope ID associated with this address.
    pub const fn scope_id(&self) -> u32 {
        self.scope_id
    }

    /// Changes the scope ID associated with this socket address.
    pub fn set_scope_id(&mut self, new_scope_id: u32) {
        self.scope_id = new_scope_id;
    }
}

impl From<SocketAddrV4> for SocketAddr {
    fn from(sock4: SocketAddrV4) -> SocketAddr {
        SocketAddr::V4(sock4)
    }
}

impl From<SocketAddrV6> for SocketAddr {
    fn from(sock6: SocketAddrV6) -> SocketAddr {
        SocketAddr::V6(sock6)
    }
}

impl<I: Into<IpAddr>> From<(I, u16)> for SocketAddr {
    fn from(pieces: (I, u16)) -> SocketAddr {
        SocketAddr::new(pieces.0.into(), pieces.1)
    }
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SocketAddr::V4(ref a) => fmt::Display::fmt(a, f),
            SocketAddr::V6(ref a) => fmt::Display::fmt(a, f),
        }
    }
}

impl fmt::Debug for SocketAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for SocketAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.precision().is_none() && f.width().is_none() {
            write!(f, "{}:{}", self.ip(), self.port())
        } else {
            let mut s = String::new();
            write!(s, "{}:{}", self.ip(), self.port())?;
            f.pad(&s)
        }
    }
}

impl fmt::Debug for SocketAddrV4 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for SocketAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        match self.scope_id() {
            0 => write!(s, "[{}]:{}", self.ip(), self.port())?,
            scope_id => write!(s, "[{}%{}]:{}", self.ip(), scope_id, self.port())?,
        }
        if f.precision().is_none() && f.width().is_none() { f.write_str(&s) } else { f.pad(&s) }
    }
}

impl fmt::Debug for SocketAddrV6 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

// ---------------------------------------------------------------------------
// ToSocketAddrs

/// A trait for objects which can be converted or resolved to one or more `SocketAddr` values.
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond to.
    type Iter: Iterator<Item = SocketAddr>;

    /// Converts this object to an iterator of resolved `SocketAddr`s.
    fn to_socket_addrs(&self) -> io::Result<Self::Iter>;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        Ok(Some(*self).into_iter())
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        SocketAddr::V4(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        SocketAddr::V6(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        let (ip, port) = *self;
        SocketAddr::new(ip, port).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        let (ip, port) = *self;
        SocketAddrV4::new(ip, port).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = crate::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<crate::option::IntoIter<SocketAddr>> {
        let (ip, port) = *self;
        SocketAddrV6::new(ip, port, 0, 0).to_socket_addrs()
    }
}

// Resolves a host name with getaddrinfo.
fn resolve_socket_addr(host: &str, port: u16) -> io::Result<vec::IntoIter<SocketAddr>> {
    let c_host = sys::cstr(host.as_bytes())?;
    let mut hints: sys::addrinfo_t = unsafe { crate::mem::zeroed() };
    hints.ai_socktype = sys::SOCK_STREAM;
    let mut res: *mut sys::addrinfo_t = crate::ptr::null_mut();
    let r = unsafe { sys::c_getaddrinfo(c_host.as_ptr(), crate::ptr::null(), &hints, &mut res) };
    if r != 0 {
        let detail = unsafe { String::from_utf8_lossy(&sys::from_cstr(sys::c_gai_strerror(r))).into_owned() };
        if r == -11 {
            // EAI_SYSTEM
            return Err(io::Error::last_os_error());
        }
        return Err(io::Error::new(
            io::ErrorKind::Uncategorized,
            format!("failed to lookup address information: {}", detail),
        ));
    }
    let mut out = Vec::new();
    let mut cur = res;
    while !cur.is_null() {
        unsafe {
            if let Ok(mut a) = super::socket::sockaddr_to_addr((*cur).ai_addr, (*cur).ai_addrlen as usize) {
                a.set_port(port);
                out.push(a);
            }
            cur = (*cur).ai_next;
        }
    }
    unsafe { sys::c_freeaddrinfo(res) };
    Ok(out.into_iter())
}

impl ToSocketAddrs for (&str, u16) {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        let (host, port) = *self;
        // try to parse the host as a regular IP address first
        if let Ok(addr) = host.parse::<Ipv4Addr>() {
            let addr = SocketAddrV4::new(addr, port);
            return Ok(vec![SocketAddr::V4(addr)].into_iter());
        }
        if let Ok(addr) = host.parse::<Ipv6Addr>() {
            let addr = SocketAddrV6::new(addr, port, 0, 0);
            return Ok(vec![SocketAddr::V6(addr)].into_iter());
        }
        resolve_socket_addr(host, port)
    }
}

impl ToSocketAddrs for (String, u16) {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        (&*self.0, self.1).to_socket_addrs()
    }
}

impl ToSocketAddrs for str {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        // try to parse as a regular SocketAddr first
        if let Ok(addr) = self.parse::<SocketAddr>() {
            return Ok(vec![addr].into_iter());
        }
        // split the string by ':' and convert the second part to u16
        let (host, port_str) = match self.rsplit_once(':') {
            Some(x) => x,
            None => return Err(io::const_io_error(io::ErrorKind::InvalidInput, "invalid socket address")),
        };
        let port: u16 = match port_str.parse() {
            Ok(p) => p,
            Err(_) => return Err(io::const_io_error(io::ErrorKind::InvalidInput, "invalid port value")),
        };
        resolve_socket_addr(host, port)
    }
}

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = crate::iter::Cloned<crate::slice::Iter<'a, SocketAddr>>;
    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        Ok(self.iter().cloned())
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;
    fn to_socket_addrs(&self) -> io::Result<T::Iter> {
        (**self).to_socket_addrs()
    }
}

impl ToSocketAddrs for String {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        (&**self).to_socket_addrs()
    }
}

/// Runs `f` on each address until it succeeds (the last error is returned).
pub(crate) fn each_addr<A: ToSocketAddrs, F, T>(addr: A, mut f: F) -> io::Result<T>
where
    F: FnMut(&SocketAddr) -> io::Result<T>,
{
    let addrs = match addr.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(e) => return f_err(e),
    };
    let mut last_err = None;
    for addr in addrs {
        match f(&addr) {
            Ok(l) => return Ok(l),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        io::const_io_error(io::ErrorKind::InvalidInput, "could not resolve to any addresses")
    }))
}

fn f_err<T>(e: io::Error) -> io::Result<T> {
    Err(e)
}
