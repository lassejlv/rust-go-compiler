//! A private parser implementation of IPv4, IPv6, and socket addresses.

use super::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use crate::error::Error;
use crate::fmt;
use crate::str::FromStr;

struct Parser<'a> {
    // Parsing as ASCII, so can use byte array.
    state: &'a [u8],
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8]) -> Parser<'a> {
        Parser { state: input }
    }

    /// Run a parser, and restore the pre-parse state if it fails.
    fn read_atomically<T, F>(&mut self, inner: F) -> Option<T>
    where
        F: FnOnce(&mut Parser<'a>) -> Option<T>,
    {
        let state = self.state;
        let result = inner(self);
        if result.is_none() {
            self.state = state;
        }
        result
    }

    /// Run a parser, but fail if the entire input wasn't consumed.
    fn parse_with<T, F>(&mut self, inner: F, kind: AddrKind) -> Result<T, AddrParseError>
    where
        F: FnOnce(&mut Parser<'a>) -> Option<T>,
    {
        let result = inner(self);
        match result {
            Some(r) if self.state.is_empty() => Ok(r),
            _ => Err(AddrParseError(kind)),
        }
    }

    fn peek_char(&self) -> Option<u8> {
        self.state.first().copied()
    }

    fn read_char(&mut self) -> Option<u8> {
        match self.state.split_first() {
            Some((&b, tail)) => {
                self.state = tail;
                Some(b)
            }
            None => None,
        }
    }

    fn read_given_char(&mut self, target: u8) -> Option<()> {
        self.read_atomically(|p| match p.read_char() {
            Some(c) if c == target => Some(()),
            _ => None,
        })
    }

    fn read_digit(&mut self, radix: u32) -> Option<u32> {
        self.read_atomically(|p| {
            let c = p.read_char()?;
            (c as char).to_digit(radix)
        })
    }

    /// Reads a number off the front of the input in the given radix, stopping
    /// at the first non-digit character or eof (fails on overflow of `max`).
    fn read_number(&mut self, radix: u32, max_digits: Option<usize>, allow_zero_prefix: bool, max: u64) -> Option<u64> {
        self.read_atomically(move |p| {
            let mut result: u64 = 0;
            let mut digit_count = 0;
            let has_leading_zero = p.peek_char() == Some(b'0');
            while let Some(digit) = p.read_digit(radix) {
                result = result * radix as u64 + digit as u64;
                if result > max {
                    return None;
                }
                digit_count += 1;
                if let Some(max_digits) = max_digits {
                    if digit_count > max_digits {
                        return None;
                    }
                }
            }
            if digit_count == 0 {
                None
            } else if !allow_zero_prefix && has_leading_zero && digit_count > 1 {
                None
            } else {
                Some(result)
            }
        })
    }

    /// Reads an IPv4 address.
    fn read_ipv4_addr(&mut self) -> Option<Ipv4Addr> {
        self.read_atomically(|p| {
            let mut groups = [0u8; 4];
            let mut i = 0;
            while i < 4 {
                if i > 0 {
                    p.read_given_char(b'.')?;
                }
                groups[i] = p.read_number(10, Some(3), false, 255)? as u8;
                i += 1;
            }
            Some(Ipv4Addr::from(groups))
        })
    }

    // Read a chunk of an IPv6 address into `groups`. Returns the number of
    // groups read, along with a bool indicating if an embedded trailing IPv4
    // address was read.
    fn read_groups(&mut self, groups: &mut [u16]) -> (usize, bool) {
        let limit = groups.len();
        let mut i = 0;
        while i < limit {
            // Try to read a trailing embedded IPv4 address. There must be at least two groups left.
            if i < limit - 1 {
                let ipv4 = self.read_atomically(|p| {
                    if i > 0 {
                        p.read_given_char(b':')?;
                    }
                    p.read_ipv4_addr()
                });
                if let Some(v4_addr) = ipv4 {
                    let o = v4_addr.octets();
                    groups[i] = ((o[0] as u16) << 8) | o[1] as u16;
                    groups[i + 1] = ((o[2] as u16) << 8) | o[3] as u16;
                    return (i + 2, true);
                }
            }
            let group = self.read_atomically(|p| {
                if i > 0 {
                    p.read_given_char(b':')?;
                }
                p.read_number(16, Some(4), true, 0xffff)
            });
            match group {
                Some(g) => groups[i] = g as u16,
                None => return (i, false),
            }
            i += 1;
        }
        (limit, false)
    }

    /// Reads an IPv6 address.
    fn read_ipv6_addr(&mut self) -> Option<Ipv6Addr> {
        self.read_atomically(|p| {
            // Read the front part of the address; either the whole thing, or up to the first ::
            let mut head = [0u16; 8];
            let (head_size, head_ipv4) = p.read_groups(&mut head);
            if head_size == 8 {
                return Some(Ipv6Addr::from(head));
            }
            // IPv4 part is not allowed before `::`
            if head_ipv4 {
                return None;
            }
            // Read `::` if previous code parsed less than 8 groups.
            // `::` indicates one or more groups of 16 bits of zeros.
            p.read_given_char(b':')?;
            p.read_given_char(b':')?;
            // Read the back part of the address. The :: must contain at least one
            // set of zeroes, so our max length is 7.
            let mut tail = [0u16; 7];
            let limit = 8 - (head_size + 1);
            let (tail_size, _) = p.read_groups(&mut tail[..limit]);
            // Concat the head and tail of the IP address
            let mut j = 0;
            while j < tail_size {
                head[8 - tail_size + j] = tail[j];
                j += 1;
            }
            Some(Ipv6Addr::from(head))
        })
    }

    /// Reads an IP address, either IPv4 or IPv6.
    fn read_ip_addr(&mut self) -> Option<IpAddr> {
        match self.read_ipv4_addr() {
            Some(v4) => Some(IpAddr::V4(v4)),
            None => self.read_ipv6_addr().map(IpAddr::V6),
        }
    }

    /// Reads a `:` followed by a port in base 10.
    fn read_port(&mut self) -> Option<u16> {
        self.read_atomically(|p| {
            p.read_given_char(b':')?;
            p.read_number(10, None, true, 0xffff).map(|n| n as u16)
        })
    }

    /// Reads a `%` followed by a scope ID in base 10.
    fn read_scope_id(&mut self) -> Option<u32> {
        self.read_atomically(|p| {
            p.read_given_char(b'%')?;
            p.read_number(10, None, true, 0xffff_ffff).map(|n| n as u32)
        })
    }

    /// Reads an IPv4 address with a port.
    fn read_socket_addr_v4(&mut self) -> Option<SocketAddrV4> {
        self.read_atomically(|p| {
            let ip = p.read_ipv4_addr()?;
            let port = p.read_port()?;
            Some(SocketAddrV4::new(ip, port))
        })
    }

    /// Reads an IPv6 address with a port.
    fn read_socket_addr_v6(&mut self) -> Option<SocketAddrV6> {
        self.read_atomically(|p| {
            p.read_given_char(b'[')?;
            let ip = p.read_ipv6_addr()?;
            let scope_id = p.read_scope_id().unwrap_or(0);
            p.read_given_char(b']')?;
            let port = p.read_port()?;
            Some(SocketAddrV6::new(ip, port, 0, scope_id))
        })
    }

    /// Reads an IP address with a port.
    fn read_socket_addr(&mut self) -> Option<SocketAddr> {
        match self.read_socket_addr_v4() {
            Some(v4) => Some(SocketAddr::V4(v4)),
            None => self.read_socket_addr_v6().map(SocketAddr::V6),
        }
    }
}

impl IpAddr {
    /// Parse an IP address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        Parser::new(b).parse_with(|p| p.read_ip_addr(), AddrKind::Ip)
    }
}

impl FromStr for IpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<IpAddr, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

impl Ipv4Addr {
    /// Parse an IPv4 address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        // don't try to parse if too long
        if b.len() > 15 {
            Err(AddrParseError(AddrKind::Ipv4))
        } else {
            Parser::new(b).parse_with(|p| p.read_ipv4_addr(), AddrKind::Ipv4)
        }
    }
}

impl FromStr for Ipv4Addr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Ipv4Addr, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

impl Ipv6Addr {
    /// Parse an IPv6 address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        Parser::new(b).parse_with(|p| p.read_ipv6_addr(), AddrKind::Ipv6)
    }
}

impl FromStr for Ipv6Addr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Ipv6Addr, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

impl SocketAddrV4 {
    /// Parse an IPv4 socket address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        Parser::new(b).parse_with(|p| p.read_socket_addr_v4(), AddrKind::SocketV4)
    }
}

impl FromStr for SocketAddrV4 {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<SocketAddrV4, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

impl SocketAddrV6 {
    /// Parse an IPv6 socket address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        Parser::new(b).parse_with(|p| p.read_socket_addr_v6(), AddrKind::SocketV6)
    }
}

impl FromStr for SocketAddrV6 {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<SocketAddrV6, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

impl SocketAddr {
    /// Parse a socket address from a slice of bytes.
    pub fn parse_ascii(b: &[u8]) -> Result<Self, AddrParseError> {
        Parser::new(b).parse_with(|p| p.read_socket_addr(), AddrKind::Socket)
    }
}

impl FromStr for SocketAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<SocketAddr, AddrParseError> {
        Self::parse_ascii(s.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddrKind {
    Ip,
    Ipv4,
    Ipv6,
    Socket,
    SocketV4,
    SocketV6,
}

/// An error which can be returned when parsing an IP address or a socket address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrParseError(AddrKind);

impl fmt::Display for AddrParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.pad(match self.0 {
            AddrKind::Ip => "invalid IP address syntax",
            AddrKind::Ipv4 => "invalid IPv4 address syntax",
            AddrKind::Ipv6 => "invalid IPv6 address syntax",
            AddrKind::Socket => "invalid socket address syntax",
            AddrKind::SocketV4 => "invalid IPv4 socket address syntax",
            AddrKind::SocketV6 => "invalid IPv6 socket address syntax",
        })
    }
}

impl Error for AddrParseError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match self.0 {
            AddrKind::Ip => "invalid IP address syntax",
            AddrKind::Ipv4 => "invalid IPv4 address syntax",
            AddrKind::Ipv6 => "invalid IPv6 address syntax",
            AddrKind::Socket => "invalid socket address syntax",
            AddrKind::SocketV4 => "invalid IPv4 socket address syntax",
            AddrKind::SocketV6 => "invalid IPv6 socket address syntax",
        }
    }
}
