// Bit manipulation used by hashers, codecs and allocators: rotate, count/leading/trailing zeros,
// swap_bytes, reverse_bits, to/from be/le/ne bytes, pdep-like spreading, morton codes, popcount tables.

fn morton_encode(x: u16, y: u16) -> u32 {
    fn spread(v: u16) -> u32 {
        let mut v = v as u32;
        v = (v | (v << 8)) & 0x00FF00FF;
        v = (v | (v << 4)) & 0x0F0F0F0F;
        v = (v | (v << 2)) & 0x33333333;
        v = (v | (v << 1)) & 0x55555555;
        v
    }
    spread(x) | (spread(y) << 1)
}

fn morton_decode(m: u32) -> (u16, u16) {
    fn compact(mut v: u32) -> u16 {
        v &= 0x55555555;
        v = (v | (v >> 1)) & 0x33333333;
        v = (v | (v >> 2)) & 0x0F0F0F0F;
        v = (v | (v >> 4)) & 0x00FF00FF;
        v = (v | (v >> 8)) & 0x0000FFFF;
        v as u16
    }
    (compact(m), compact(m >> 1))
}

fn next_permutation_bits(v: u32) -> u32 {
    // next higher number with same popcount (Gosper's hack)
    let t = v | (v.wrapping_sub(1));
    (t.wrapping_add(1)) | (((!t & (!t).wrapping_neg()).wrapping_sub(1)) >> (v.trailing_zeros() + 1))
}

fn murmur_fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

fn align_up(x: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (x + align - 1) & !(align - 1)
}

fn main() {
    let x: u32 = 0x12345678;
    println!("{:08x} {:08x} {:08x}", x.rotate_left(8), x.rotate_right(4), x.swap_bytes());
    println!("{:032b}", x.reverse_bits());
    println!("{} {} {} {}", x.count_ones(), x.count_zeros(), x.leading_zeros(), x.trailing_zeros());
    println!("{:?} {:?}", x.to_be_bytes(), x.to_le_bytes());
    println!("{:#x} {:#x}", u32::from_be_bytes([0xde, 0xad, 0xbe, 0xef]), u32::from_le_bytes([0xde, 0xad, 0xbe, 0xef]));
    println!("{}", u32::from_ne_bytes(x.to_ne_bytes()) == x);
    let y: i16 = -2;
    println!("{:016b} {} {} {:?}", y, y.count_ones(), y.leading_ones(), y.to_be_bytes());
    println!("{} {}", (-1i8) as u8 >> 1, (-128i8) >> 1);
    println!("{} {} {}", 1u64.rotate_right(1) == 1 << 63, u8::MAX.rotate_left(3), 0b1011_0000u8.trailing_zeros());
    println!("{} {} {}", 40u32.is_power_of_two(), 64u32.is_power_of_two(), 40u32.next_power_of_two());
    println!("{:?} {:?}", 0u32.checked_next_power_of_two(), (u32::MAX / 2 + 2).checked_next_power_of_two());
    println!("{} {}", u64::MAX.ilog2(), 1000u32.ilog10());
    println!("{} {} {}", 7u8.wrapping_shl(9), 1u32.checked_shl(32).is_none(), 0x80u8.overflowing_shl(1).0);

    for (a, b) in [(0u16, 0u16), (1, 0), (0, 1), (3, 5), (255, 255), (1234, 4321)] {
        let m = morton_encode(a, b);
        println!("morton({},{}) = {:#010x} -> {:?}", a, b, m, morton_decode(m));
    }
    let mut v = 0b0111u32;
    let mut seq = vec![];
    for _ in 0..8 {
        seq.push(format!("{:06b}", v));
        v = next_permutation_bits(v);
    }
    println!("{}", seq.join(" "));
    for k in [0u64, 1, 42, u64::MAX] {
        println!("fmix64({}) = {:016x}", k, murmur_fmix64(k));
    }
    println!("crc32 = {:08x} {:08x} {:08x}", crc32(b""), crc32(b"123456789"), crc32(b"The quick brown fox jumps over the lazy dog"));
    println!("{:?}", [1usize, 8, 9, 15, 16, 17].map(|x| align_up(x, 8)));
    let table: Vec<u8> = (0..16u8).map(|i| i.count_ones() as u8).collect();
    println!("{:?}", table);
    let packed: u64 = (0..8).fold(0u64, |acc, i| acc | ((i as u64 * 3) << (i * 8)));
    println!("{:016x} {:?}", packed, packed.to_le_bytes());
    let mut bits = 0u128;
    for i in (0..128).step_by(7) {
        bits |= 1 << i;
    }
    println!("{} {:032x}", bits.count_ones(), bits);
    println!("{:x}", u64::from_str_radix("1010", 2).unwrap() << 60 | 0xf);
    println!("{} {}", i32::MIN.unsigned_abs(), (-5i32).signum() * 5i32.pow(2));
    println!("{} {}", 0x0Fu8 & !0x03, 0xF0u8 ^ 0xFF);
}
