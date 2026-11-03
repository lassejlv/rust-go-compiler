// integer parsing with radix, to radix strings, bit ops: leading_zeros/count_ones/rotate/reverse/swap
fn to_radix(mut n: u64, r: u32) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut ds = vec![];
    while n > 0 {
        ds.push(std::char::from_digit((n % r as u64) as u32, r).unwrap());
        n /= r as u64;
    }
    ds.iter().rev().collect()
}

fn gray(n: u32) -> u32 {
    n ^ (n >> 1)
}
fn ungray(mut g: u32) -> u32 {
    let mut n = 0;
    while g != 0 {
        n ^= g;
        g >>= 1;
    }
    n
}

fn main() {
    for (s, r) in [("ff", 16), ("FF", 16), ("777", 8), ("101010", 2), ("zz", 36), ("Zz", 36), ("-80", 16), ("+7f", 16), ("12", 3), ("2", 2), ("", 10)] {
        println!("{:?} r{} -> i32 {:?} u8 {:?}", s, r, i32::from_str_radix(s, r).map_err(|e| e.to_string()), u8::from_str_radix(s, r).map_err(|e| e.to_string()));
    }
    for n in [0u64, 7, 255, 1000, 65535, u64::MAX] {
        println!("{} -> b{} o{} x{} r36 {}", n, to_radix(n, 2).len(), to_radix(n, 8), to_radix(n, 16), to_radix(n, 36));
    }
    let x: u32 = 0b1011_0000_1111_0000_0000_0000_0001_0100;
    println!("{:032b}", x);
    println!("lz={} tz={} ones={} zeros={} lo={} to={}", x.leading_zeros(), x.trailing_zeros(), x.count_ones(), x.count_zeros(), x.leading_ones(), x.trailing_ones());
    println!("{:032b}", x.rotate_left(8));
    println!("{:032b}", x.rotate_right(4));
    println!("{:032b}", x.reverse_bits());
    println!("{:08x} {:08x}", x, x.swap_bytes());
    println!("{:?} {:?}", x.to_be_bytes(), x.to_le_bytes());
    println!("{}", u32::from_be_bytes([0x12, 0x34, 0x56, 0x78]) == 0x12345678);
    println!("{:#x}", u32::from_le_bytes([0x12, 0x34, 0x56, 0x78]));
    println!("{:#x}", u16::from_ne_bytes(0xabcdu16.to_ne_bytes()));
    println!("{:?}", (-2i16).to_be_bytes());
    println!("{}", i64::from_le_bytes([0xff; 8]));
    for v in [1u8, 0x80, 0xff, 0] {
        println!("u8 {:3}: lz {} tz {} ilog2 {:?}", v, v.leading_zeros(), v.trailing_zeros(), v.checked_ilog2());
    }
    println!("{} {}", (-1i32).leading_zeros(), (-1i32).count_ones());
    println!("{} {}", i32::MIN.trailing_zeros(), (1i64 << 40).trailing_zeros());
    println!("{}", (0u16).leading_zeros());
    let g: Vec<u32> = (0..8).map(gray).collect();
    println!("{:?}", g.iter().map(|v| format!("{:03b}", v)).collect::<Vec<_>>());
    println!("{:?}", g.iter().map(|&v| ungray(v)).collect::<Vec<_>>());
    let mut subsets = vec![];
    let mask: u32 = 0b1101;
    let mut s = mask;
    loop {
        subsets.push(s);
        if s == 0 {
            break;
        }
        s = (s - 1) & mask;
    }
    println!("{:?}", subsets);
    let lowest = |v: i32| v & v.wrapping_neg();
    println!("{} {} {}", lowest(12), lowest(7), lowest(i32::MIN));
    println!("{}", (0..16u32).filter(|n| n.count_ones() == 2).count());
    let popcounts: Vec<u32> = (0..10u8).map(|n| n.count_ones()).collect();
    println!("{:?}", popcounts);
    println!("{}", u64::MAX.count_ones() + u128::MAX.count_zeros());
    println!("{:x}", 0x12u8.rotate_left(4));
    println!("{}", 0b1000_0000u8 as i8 >> 3);
    println!("{}", 0b1000_0000u8 >> 3);
    println!("{}", -17i32 >> 2);
    println!("{}", (-17i32 as u32) >> 28);
    println!("{}", 1u64.checked_shl(64).is_none());
    println!("{:?}", "1_000".parse::<i32>().is_err());
    println!("{:?}", i8::from_str_radix("-128", 10));
    println!("{:?}", i8::from_str_radix("-129", 10).map_err(|e| e.to_string()));
    println!("{:?}", u64::from_str_radix("1".repeat(64).as_str(), 2));
    println!("{:?}", u64::from_str_radix("1".repeat(65).as_str(), 2).is_err());
    println!("{}", isize::BITS + usize::MAX.count_ones());
    let bits = format!("{:b}", 1234567u32);
    println!("{} {}", bits, u32::from_str_radix(&bits, 2).unwrap());
    println!("{:?}", std::char::from_digit(35, 36));
    println!("{:?}", 'Z'.to_digit(36));
}
