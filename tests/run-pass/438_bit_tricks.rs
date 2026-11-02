// Bit manipulation tricks: popcount, parity, isolate lowest bit, gray code, bit reversal,
// next permutation of bits (Gosper's hack), bitboard ops on u64, set iteration, sign extension,
// byte swapping, leading/trailing ones, and a tiny bitset type with operator overloading.
use std::ops::{BitAnd, BitOr, BitXor, Not, Shl};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Bits(u64);

impl BitAnd for Bits {
    type Output = Bits;
    fn bitand(self, o: Bits) -> Bits {
        Bits(self.0 & o.0)
    }
}
impl BitOr for Bits {
    type Output = Bits;
    fn bitor(self, o: Bits) -> Bits {
        Bits(self.0 | o.0)
    }
}
impl BitXor for Bits {
    type Output = Bits;
    fn bitxor(self, o: Bits) -> Bits {
        Bits(self.0 ^ o.0)
    }
}
impl Not for Bits {
    type Output = Bits;
    fn not(self) -> Bits {
        Bits(!self.0)
    }
}
impl Shl<u32> for Bits {
    type Output = Bits;
    fn shl(self, n: u32) -> Bits {
        Bits(self.0 << n)
    }
}
impl Bits {
    fn iter(self) -> impl Iterator<Item = u32> {
        let mut v = self.0;
        std::iter::from_fn(move || {
            if v == 0 {
                None
            } else {
                let t = v.trailing_zeros();
                v &= v - 1;
                Some(t)
            }
        })
    }
}

fn gosper(x: u32) -> u32 {
    let c = x & x.wrapping_neg();
    let r = x + c;
    (((r ^ x) >> 2) / c) | r
}

fn sign_extend(v: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((v << shift) as i32) >> shift
}

fn manual_popcount(mut x: u64) -> u32 {
    x = x - ((x >> 1) & 0x5555555555555555);
    x = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333);
    x = (x + (x >> 4)) & 0x0f0f0f0f0f0f0f0f;
    (x.wrapping_mul(0x0101010101010101) >> 56) as u32
}

fn reverse_byte(b: u8) -> u8 {
    let mut b = b;
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    b
}

fn main() {
    for x in [0u64, 1, 0xff, 0xdeadbeef, u64::MAX, 0x8000000000000001] {
        println!("{:#x}: pop={} manual={} parity={} lowbit={:#x} lz={} tz={}", x, x.count_ones(), manual_popcount(x), x.count_ones() & 1, x & x.wrapping_neg(), x.leading_zeros(), x.trailing_zeros());
    }
    let gray: Vec<String> = (0u8..8).map(|i| format!("{:03b}", i ^ (i >> 1))).collect();
    println!("gray {:?}", gray);
    let mut g = 0b0111u32;
    let mut perms = vec![];
    for _ in 0..6 {
        perms.push(format!("{:06b}", g));
        g = gosper(g);
    }
    println!("gosper {:?}", perms);

    println!("{} {} {} {}", sign_extend(0b1111, 4), sign_extend(0b0111, 4), sign_extend(0x80, 8), sign_extend(0xFFF, 12));
    println!("{:08b} {:08b} {}", reverse_byte(0b0000_0110), reverse_byte(0b1000_0000), reverse_byte(0x5a) == 0x5au8.reverse_bits());
    println!("{} {} {} {}", 0b1110_0000u8.leading_ones(), 0b0000_0111u8.trailing_ones(), (-1i32).leading_ones(), 0i64.trailing_ones());
    println!("{:#x} {:#x}", 0x12345678u32.swap_bytes(), 0x12345678u32.to_be());
    println!("{:?} {:?}", 0x1234u16.to_le_bytes(), 0x1234u16.to_be_bytes());
    println!("{} {}", i32::from_be_bytes([0xff, 0xff, 0xff, 0xfe]), u32::from_ne_bytes(7u32.to_ne_bytes()));

    // bitboard: knight moves from a square on 8x8
    let not_a: u64 = 0xfefefefefefefefe;
    let not_ab: u64 = 0xfcfcfcfcfcfcfcfc;
    let not_h: u64 = 0x7f7f7f7f7f7f7f7f;
    let not_gh: u64 = 0x3f3f3f3f3f3f3f3f;
    let knight = |sq: u32| -> u64 {
        let b = 1u64 << sq;
        ((b << 17) & not_a) | ((b << 15) & not_h) | ((b << 10) & not_ab) | ((b << 6) & not_gh)
            | ((b >> 17) & not_h) | ((b >> 15) & not_a) | ((b >> 10) & not_gh) | ((b >> 6) & not_ab)
    };
    for sq in [0u32, 27, 63] {
        let m = Bits(knight(sq));
        println!("knight {} -> {:?} ({} moves)", sq, m.iter().collect::<Vec<_>>(), m.0.count_ones());
    }

    let a = Bits(0b1100_1010);
    let b = Bits(0b1010_0110);
    println!("{:?} {:?} {:?}", (a & b).iter().collect::<Vec<_>>(), (a | b).0, (a ^ b).0);
    println!("{:#x} {:?}", (!a).0 >> 56, (a << 4).iter().collect::<Vec<_>>());
    println!("{}", (a & !b) == Bits(0b0100_1000));

    // isolate/clear/toggle helpers
    let mut flags: u16 = 0;
    for bit in [1, 3, 5, 15] {
        flags |= 1 << bit;
    }
    flags &= !(1 << 3);
    flags ^= 1 << 0;
    println!("{:016b} {}", flags, flags.count_ones());
    let subsets: Vec<u8> = {
        let mask = 0b1011u8;
        let mut out = vec![];
        let mut s = mask;
        loop {
            out.push(s);
            if s == 0 {
                break;
            }
            s = (s - 1) & mask;
        }
        out
    };
    println!("subsets {:?}", subsets);
    println!("{} {}", (12u32 & 11).is_power_of_two(), u64::MAX.rotate_left(13) == u64::MAX);
}
