//! IP address types.

use crate::cmp::Ordering;
use crate::fmt::{self, Write};

/// An IP address, either IPv4 or IPv6.
#[derive(Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum IpAddr {
    /// An IPv4 address.
    V4(Ipv4Addr),
    /// An IPv6 address.
    V6(Ipv6Addr),
}

/// An IPv4 address.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Ipv4Addr {
    octets: [u8; 4],
}

/// An IPv6 address.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Ipv6Addr {
    octets: [u8; 16],
}

/// Scope of an IPv6 multicast address.
#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
#[non_exhaustive]
pub enum Ipv6MulticastScope {
    InterfaceLocal,
    LinkLocal,
    RealmLocal,
    AdminLocal,
    SiteLocal,
    OrganizationLocal,
    Global,
}

impl IpAddr {
    /// Returns `true` for the special 'unspecified' address.
    pub const fn is_unspecified(&self) -> bool {
        match self {
            IpAddr::V4(ip) => ip.is_unspecified(),
            IpAddr::V6(ip) => ip.is_unspecified(),
        }
    }

    /// Returns `true` if this is a loopback address.
    pub const fn is_loopback(&self) -> bool {
        match self {
            IpAddr::V4(ip) => ip.is_loopback(),
            IpAddr::V6(ip) => ip.is_loopback(),
        }
    }

    /// Returns `true` if this is a multicast address.
    pub const fn is_multicast(&self) -> bool {
        match self {
            IpAddr::V4(ip) => ip.is_multicast(),
            IpAddr::V6(ip) => ip.is_multicast(),
        }
    }

    /// Returns `true` if this address is an IPv4 address, and `false` otherwise.
    pub const fn is_ipv4(&self) -> bool {
        match self {
            IpAddr::V4(_) => true,
            IpAddr::V6(_) => false,
        }
    }

    /// Returns `true` if this address is an IPv6 address, and `false` otherwise.
    pub const fn is_ipv6(&self) -> bool {
        match self {
            IpAddr::V4(_) => false,
            IpAddr::V6(_) => true,
        }
    }

    /// Converts this address to an `IpAddr::V4` if it is an IPv4-mapped IPv6 address.
    pub const fn to_canonical(&self) -> IpAddr {
        match *self {
            IpAddr::V4(_) => *self,
            IpAddr::V6(v6) => v6.to_canonical(),
        }
    }

    /// Returns the eight-bit integers this address consists of as a slice.
    pub fn as_octets(&self) -> &[u8] {
        match self {
            IpAddr::V4(ip) => &ip.octets,
            IpAddr::V6(ip) => &ip.octets,
        }
    }
}

impl Ipv4Addr {
    /// Creates a new IPv4 address from four eight-bit octets.
    #[inline]
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
        Ipv4Addr { octets: [a, b, c, d] }
    }

    /// The size of an IPv4 address in bits.
    pub const BITS: u32 = 32;

    /// An IPv4 address with the address pointing to localhost: `127.0.0.1`.
    pub const LOCALHOST: Self = Ipv4Addr::new(127, 0, 0, 1);

    /// An IPv4 address representing an unspecified address: `0.0.0.0`.
    pub const UNSPECIFIED: Self = Ipv4Addr::new(0, 0, 0, 0);

    /// An IPv4 address representing the broadcast address: `255.255.255.255`.
    pub const BROADCAST: Self = Ipv4Addr::new(255, 255, 255, 255);

    /// Converts the address into a `u32` (big-endian bits).
    #[inline]
    pub const fn to_bits(self) -> u32 {
        ((self.octets[0] as u32) << 24)
            | ((self.octets[1] as u32) << 16)
            | ((self.octets[2] as u32) << 8)
            | (self.octets[3] as u32)
    }

    /// Converts a native byte order `u32` into an IPv4 address.
    #[inline]
    pub const fn from_bits(bits: u32) -> Ipv4Addr {
        Ipv4Addr::new((bits >> 24) as u8, (bits >> 16) as u8, (bits >> 8) as u8, bits as u8)
    }

    /// Returns the four eight-bit integers that make up this address.
    #[inline]
    pub const fn octets(&self) -> [u8; 4] {
        self.octets
    }

    /// Creates an `Ipv4Addr` from a four element byte array.
    #[inline]
    pub const fn from_octets(octets: [u8; 4]) -> Ipv4Addr {
        Ipv4Addr { octets }
    }

    /// Returns the four eight-bit integers that make up this address as a slice.
    #[inline]
    pub const fn as_octets(&self) -> &[u8; 4] {
        &self.octets
    }

    /// Returns `true` for the special 'unspecified' address (`0.0.0.0`).
    #[inline]
    pub const fn is_unspecified(&self) -> bool {
        self.to_bits() == 0
    }

    /// Returns `true` if this is a loopback address (`127.0.0.0/8`).
    #[inline]
    pub const fn is_loopback(&self) -> bool {
        self.octets[0] == 127
    }

    /// Returns `true` if this is a private address (10/8, 172.16/12, 192.168/16).
    #[inline]
    pub const fn is_private(&self) -> bool {
        match self.octets {
            [10, ..] => true,
            [172, b, ..] if b >= 16 && b <= 31 => true,
            [192, 168, ..] => true,
            _ => false,
        }
    }

    /// Returns `true` if the address is link-local (`169.254.0.0/16`).
    #[inline]
    pub const fn is_link_local(&self) -> bool {
        self.octets[0] == 169 && self.octets[1] == 254
    }

    /// Returns `true` if this address is part of the Shared Address Space defined in IETF RFC 6598 (`100.64.0.0/10`).
    #[inline]
    pub const fn is_shared(&self) -> bool {
        self.octets[0] == 100 && (self.octets[1] & 0b1100_0000 == 0b0100_0000)
    }

    /// Returns `true` if this is a multicast address (`224.0.0.0/4`).
    #[inline]
    pub const fn is_multicast(&self) -> bool {
        self.octets[0] >= 224 && self.octets[0] <= 239
    }

    /// Returns `true` if this is a broadcast address (`255.255.255.255`).
    #[inline]
    pub const fn is_broadcast(&self) -> bool {
        self.to_bits() == 0xffff_ffff
    }

    /// Returns `true` if this address is in a range designated for documentation.
    #[inline]
    pub const fn is_documentation(&self) -> bool {
        match self.octets {
            [192, 0, 2, _] => true,
            [198, 51, 100, _] => true,
            [203, 0, 113, _] => true,
            _ => false,
        }
    }

    /// Returns `true` if the address appears to be globally reachable.
    pub const fn is_global(&self) -> bool {
        !(self.octets[0] == 0
            || self.is_private()
            || self.is_shared()
            || self.is_loopback()
            || self.is_link_local()
            || (self.octets[0] == 192 && self.octets[1] == 0 && self.octets[2] == 0)
            || self.is_documentation()
            || (self.octets[0] == 198 && (self.octets[1] & 0xfe) == 18)
            || (self.octets[0] & 240 == 240)
            || self.is_broadcast())
    }

    /// Converts this address to an IPv4-compatible IPv6 address (`::a.b.c.d`).
    #[inline]
    pub const fn to_ipv6_compatible(&self) -> Ipv6Addr {
        let [a, b, c, d] = self.octets;
        Ipv6Addr { octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d] }
    }

    /// Converts this address to an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`).
    #[inline]
    pub const fn to_ipv6_mapped(&self) -> Ipv6Addr {
        let [a, b, c, d] = self.octets;
        Ipv6Addr { octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, a, b, c, d] }
    }
}

impl Ipv6Addr {
    /// Creates a new IPv6 address from eight 16-bit segments.
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Ipv6Addr {
        Ipv6Addr {
            octets: [
                (a >> 8) as u8,
                a as u8,
                (b >> 8) as u8,
                b as u8,
                (c >> 8) as u8,
                c as u8,
                (d >> 8) as u8,
                d as u8,
                (e >> 8) as u8,
                e as u8,
                (f >> 8) as u8,
                f as u8,
                (g >> 8) as u8,
                g as u8,
                (h >> 8) as u8,
                h as u8,
            ],
        }
    }

    /// The size of an IPv6 address in bits.
    pub const BITS: u32 = 128;

    /// An IPv6 address representing localhost: `::1`.
    pub const LOCALHOST: Self = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

    /// An IPv6 address representing the unspecified address: `::`.
    pub const UNSPECIFIED: Self = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    /// Converts an IPv6 address into a `u128` representation.
    pub const fn to_bits(self) -> u128 {
        let mut r: u128 = 0;
        let mut i = 0;
        while i < 16 {
            r = (r << 8) | (self.octets[i] as u128);
            i += 1;
        }
        r
    }

    /// Converts a native byte order `u128` into an IPv6 address.
    pub const fn from_bits(bits: u128) -> Ipv6Addr {
        let mut octets = [0u8; 16];
        let mut i = 0;
        while i < 16 {
            octets[i] = (bits >> (8 * (15 - i))) as u8;
            i += 1;
        }
        Ipv6Addr { octets }
    }

    /// Returns the eight 16-bit segments that make up this address.
    pub const fn segments(&self) -> [u16; 8] {
        let o = &self.octets;
        [
            ((o[0] as u16) << 8) | o[1] as u16,
            ((o[2] as u16) << 8) | o[3] as u16,
            ((o[4] as u16) << 8) | o[5] as u16,
            ((o[6] as u16) << 8) | o[7] as u16,
            ((o[8] as u16) << 8) | o[9] as u16,
            ((o[10] as u16) << 8) | o[11] as u16,
            ((o[12] as u16) << 8) | o[13] as u16,
            ((o[14] as u16) << 8) | o[15] as u16,
        ]
    }

    /// Creates an `Ipv6Addr` from an eight element 16-bit array.
    pub const fn from_segments(s: [u16; 8]) -> Ipv6Addr {
        Ipv6Addr::new(s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7])
    }

    /// Returns `true` for the special 'unspecified' address (`::`).
    pub const fn is_unspecified(&self) -> bool {
        self.to_bits() == 0
    }

    /// Returns `true` if this is the loopback address (`::1`).
    pub const fn is_loopback(&self) -> bool {
        self.to_bits() == 1
    }

    /// Returns `true` if this is a multicast address (`ff00::/8`).
    pub const fn is_multicast(&self) -> bool {
        self.octets[0] == 0xff
    }

    /// Returns `true` if this is a unique local address (`fc00::/7`).
    pub const fn is_unique_local(&self) -> bool {
        (self.octets[0] & 0xfe) == 0xfc
    }

    /// Returns `true` if the address is a unicast address with link-local scope (`fe80::/10`).
    pub const fn is_unicast_link_local(&self) -> bool {
        self.octets[0] == 0xfe && (self.octets[1] & 0xc0) == 0x80
    }

    /// Returns the address's multicast scope if the address is multicast.
    pub const fn multicast_scope(&self) -> Option<Ipv6MulticastScope> {
        if self.is_multicast() {
            match self.octets[1] & 0x0f {
                1 => Some(Ipv6MulticastScope::InterfaceLocal),
                2 => Some(Ipv6MulticastScope::LinkLocal),
                3 => Some(Ipv6MulticastScope::RealmLocal),
                4 => Some(Ipv6MulticastScope::AdminLocal),
                5 => Some(Ipv6MulticastScope::SiteLocal),
                8 => Some(Ipv6MulticastScope::OrganizationLocal),
                14 => Some(Ipv6MulticastScope::Global),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Converts this address to an `IPv4` address if it's an IPv4-mapped address.
    pub const fn to_ipv4_mapped(&self) -> Option<Ipv4Addr> {
        match self.octets {
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, a, b, c, d] => Some(Ipv4Addr::new(a, b, c, d)),
            _ => None,
        }
    }

    /// Converts this address to an IPv4 address if it is either IPv4-compatible or IPv4-mapped.
    pub const fn to_ipv4(&self) -> Option<Ipv4Addr> {
        if let [0, 0, 0, 0, 0, f, g, h] = self.segments() {
            if f == 0 || f == 0xffff {
                let [a, b] = g.to_be_bytes();
                let [c, d] = h.to_be_bytes();
                return Some(Ipv4Addr::new(a, b, c, d));
            }
        }
        None
    }

    /// Converts this address to an `IpAddr::V4` if it is an IPv4-mapped address.
    pub const fn to_canonical(&self) -> IpAddr {
        if let Some(mapped) = self.to_ipv4_mapped() {
            return IpAddr::V4(mapped);
        }
        IpAddr::V6(*self)
    }

    /// Returns the sixteen eight-bit integers the IPv6 address consists of.
    pub const fn octets(&self) -> [u8; 16] {
        self.octets
    }

    /// Creates an `Ipv6Addr` from a sixteen element byte array.
    pub const fn from_octets(octets: [u8; 16]) -> Ipv6Addr {
        Ipv6Addr { octets }
    }

    /// Returns the sixteen eight-bit integers the IPv6 address consists of as a slice.
    pub const fn as_octets(&self) -> &[u8; 16] {
        &self.octets
    }
}

// ---- formatting

impl fmt::Display for IpAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(ip) => fmt::Display::fmt(ip, fmt),
            IpAddr::V6(ip) => fmt::Display::fmt(ip, fmt),
        }
    }
}

impl fmt::Debug for IpAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

fn write_v4(o: &[u8; 4], f: &mut dyn Write) -> fmt::Result {
    write!(f, "{}.{}.{}.{}", o[0], o[1], o[2], o[3])
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if fmt.precision().is_none() && fmt.width().is_none() {
            write_v4(&self.octets, fmt)
        } else {
            let mut s = String::new();
            write_v4(&self.octets, &mut s)?;
            fmt.pad(&s)
        }
    }
}

impl fmt::Debug for Ipv4Addr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

fn fmt_subslice(f: &mut dyn Write, chunk: &[u16]) -> fmt::Result {
    if let Some((first, tail)) = chunk.split_first() {
        write!(f, "{:x}", first)?;
        for segment in tail {
            f.write_char(':')?;
            write!(f, "{:x}", segment)?;
        }
    }
    Ok(())
}

fn write_v6(ip: &Ipv6Addr, f: &mut dyn Write) -> fmt::Result {
    let segments = ip.segments();
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        f.write_str("::ffff:")?;
        return write_v4(&ipv4.octets, f);
    }
    // Find the longest run of zero segments.
    let (mut lstart, mut llen) = (0usize, 0usize);
    let (mut cstart, mut clen) = (0usize, 0usize);
    for (i, &segment) in segments.iter().enumerate() {
        if segment == 0 {
            if clen == 0 {
                cstart = i;
            }
            clen += 1;
            if clen > llen {
                lstart = cstart;
                llen = clen;
            }
        } else {
            clen = 0;
            cstart = 0;
        }
    }
    if llen > 1 {
        fmt_subslice(f, &segments[..lstart])?;
        f.write_str("::")?;
        fmt_subslice(f, &segments[lstart + llen..])
    } else {
        fmt_subslice(f, &segments)
    }
}

impl fmt::Display for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.precision().is_none() && f.width().is_none() {
            write_v6(self, f)
        } else {
            let mut s = String::new();
            write_v6(self, &mut s)?;
            f.pad(&s)
        }
    }
}

impl fmt::Debug for Ipv6Addr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

// ---- conversions and comparisons

impl From<Ipv4Addr> for IpAddr {
    #[inline]
    fn from(ipv4: Ipv4Addr) -> IpAddr {
        IpAddr::V4(ipv4)
    }
}

impl From<Ipv6Addr> for IpAddr {
    #[inline]
    fn from(ipv6: Ipv6Addr) -> IpAddr {
        IpAddr::V6(ipv6)
    }
}

impl From<Ipv4Addr> for u32 {
    #[inline]
    fn from(ip: Ipv4Addr) -> u32 {
        ip.to_bits()
    }
}

impl From<u32> for Ipv4Addr {
    #[inline]
    fn from(ip: u32) -> Ipv4Addr {
        Ipv4Addr::from_bits(ip)
    }
}

impl From<[u8; 4]> for Ipv4Addr {
    #[inline]
    fn from(octets: [u8; 4]) -> Ipv4Addr {
        Ipv4Addr { octets }
    }
}

impl From<[u8; 4]> for IpAddr {
    #[inline]
    fn from(octets: [u8; 4]) -> IpAddr {
        IpAddr::V4(Ipv4Addr::from(octets))
    }
}

impl From<Ipv6Addr> for u128 {
    #[inline]
    fn from(ip: Ipv6Addr) -> u128 {
        ip.to_bits()
    }
}

impl From<u128> for Ipv6Addr {
    #[inline]
    fn from(ip: u128) -> Ipv6Addr {
        Ipv6Addr::from_bits(ip)
    }
}

impl From<[u8; 16]> for Ipv6Addr {
    #[inline]
    fn from(octets: [u8; 16]) -> Ipv6Addr {
        Ipv6Addr { octets }
    }
}

impl From<[u16; 8]> for Ipv6Addr {
    #[inline]
    fn from(segments: [u16; 8]) -> Ipv6Addr {
        Ipv6Addr::from_segments(segments)
    }
}

impl From<[u8; 16]> for IpAddr {
    #[inline]
    fn from(octets: [u8; 16]) -> IpAddr {
        IpAddr::V6(Ipv6Addr::from(octets))
    }
}

impl From<[u16; 8]> for IpAddr {
    #[inline]
    fn from(segments: [u16; 8]) -> IpAddr {
        IpAddr::V6(Ipv6Addr::from(segments))
    }
}

impl PartialEq<Ipv4Addr> for IpAddr {
    #[inline]
    fn eq(&self, other: &Ipv4Addr) -> bool {
        match self {
            IpAddr::V4(v4) => v4 == other,
            IpAddr::V6(_) => false,
        }
    }
}

impl PartialEq<IpAddr> for Ipv4Addr {
    #[inline]
    fn eq(&self, other: &IpAddr) -> bool {
        match other {
            IpAddr::V4(v4) => self == v4,
            IpAddr::V6(_) => false,
        }
    }
}

impl PartialEq<Ipv6Addr> for IpAddr {
    #[inline]
    fn eq(&self, other: &Ipv6Addr) -> bool {
        match self {
            IpAddr::V4(_) => false,
            IpAddr::V6(v6) => v6 == other,
        }
    }
}

impl PartialEq<IpAddr> for Ipv6Addr {
    #[inline]
    fn eq(&self, other: &IpAddr) -> bool {
        match other {
            IpAddr::V4(_) => false,
            IpAddr::V6(v6) => self == v6,
        }
    }
}

impl PartialOrd for Ipv4Addr {
    #[inline]
    fn partial_cmp(&self, other: &Ipv4Addr) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ipv4Addr {
    #[inline]
    fn cmp(&self, other: &Ipv4Addr) -> Ordering {
        self.octets.cmp(&other.octets)
    }
}

impl PartialOrd for Ipv6Addr {
    #[inline]
    fn partial_cmp(&self, other: &Ipv6Addr) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ipv6Addr {
    #[inline]
    fn cmp(&self, other: &Ipv6Addr) -> Ordering {
        self.segments().cmp(&other.segments())
    }
}

impl crate::ops::Not for Ipv4Addr {
    type Output = Ipv4Addr;
    #[inline]
    fn not(self) -> Ipv4Addr {
        Ipv4Addr::from_bits(!self.to_bits())
    }
}

impl crate::ops::BitAnd for Ipv4Addr {
    type Output = Ipv4Addr;
    #[inline]
    fn bitand(self, rhs: Ipv4Addr) -> Ipv4Addr {
        Ipv4Addr::from_bits(self.to_bits() & rhs.to_bits())
    }
}

impl crate::ops::BitOr for Ipv4Addr {
    type Output = Ipv4Addr;
    #[inline]
    fn bitor(self, rhs: Ipv4Addr) -> Ipv4Addr {
        Ipv4Addr::from_bits(self.to_bits() | rhs.to_bits())
    }
}
