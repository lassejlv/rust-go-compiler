mod v01 {
    pub fn main() {
        println!("{}", 1e300);
        println!("{}", 1e-300);
        println!("{}", 1e30f32);
        println!("{}", f32::MIN_POSITIVE);
        println!("{:?}", f32::MIN_POSITIVE);
        println!("{:.20}", 0.1);
        println!("{:.30e}", 0.1);
        println!("{:.3}", 0.0005);
        println!("{:.0}", 0.5);
        println!("{:.0}", 1.5);
        println!("{:.1}", 0.25);
        println!("{:.1}", 0.35);
        println!("{:.2}", 1.005);
        println!("{}", 5e-324);
        println!("{:?}", 5e-324);
        println!("{:e}", 5e-324);
        println!("{}", f64::MAX);
        println!("{:?}", 123456789012345680.0);
        println!("{}", 0.3f32 as f64);
        println!("{:?} {:?}", 1.0e7f32, 1.0e-5f32);
        println!("{:?} {:?}", 16777216.0f32, 0.1f32 + 0.7f32);
        println!("{:10.3e}|{:<10e}|{:+e}", 123.456, 0.001, 1.0);
        println!("{:e}", 1e100);
        println!("{:E}", 12345678901234567890u64);
        println!("{:e}", 100u8);
        println!("{:.2e}", 1999u32);
        println!("{:.0e}", 25u32);
        println!("{:.0e}", 35u32);
        println!("{:.0e}", 2.5);
        println!("{:010.2}", -0.0);
        println!("{:?}", -0.0);
        println!("{}", -0.0f32);
        println!("{:5}|{:<5}|{:^7}|", f64::NAN, f64::INFINITY, -f64::INFINITY);
        println!("{:+}", f64::NAN);
        println!("{:08}", f64::NEG_INFINITY);
        println!("{:.3?}", 1.0f64);
        println!("{:#?}", 1.5f32);
        println!("{:#x}", 0u8);
        println!("{:#b}", 0u8);
        println!("{:+05}", 7);
        println!("{:<+5}|", -7);
        println!("{:^+6}|", 7);
        println!("{:#012x}", -1i32);
        println!("{:x}", i128::MIN);
        println!("{:o}", u64::MAX);
        println!("{:30}|", u128::MAX);
        println!("{:e}", u128::MAX);
        println!("{:e}", i64::MIN);
    }
}

mod v02 {
    pub fn main() {
        #[derive(Debug, Clone)]
        struct R { k: u8, id: u32 }
        let mut v: Vec<R> = (0..2000u32).map(|i| R { k: ((i * 7919) % 13) as u8, id: i }).collect();
        v.sort_by_key(|r| r.k);
        let ok = v.windows(2).all(|w| w[0].k < w[1].k || (w[0].k == w[1].k && w[0].id < w[1].id));
        println!("stable={}", ok);
        v.sort_by(|a, b| b.k.cmp(&a.k));
        let ok = v.windows(2).all(|w| w[0].k > w[1].k || (w[0].k == w[1].k && w[0].id < w[1].id));
        println!("stable desc={}", ok);
        let mut s: Vec<String> = (0..500).map(|i| format!("{}", (i * 37) % 101)).collect();
        s.sort();
        println!("{:?}", &s[..10]);
        s.dedup();
        println!("{}", s.len());
        let mut f: Vec<f64> = (0..100).map(|i| ((i * 31) % 17) as f64 / 3.0).collect();
        f.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{:?}", &f[95..]);
        let mut u: Vec<u64> = (0..1000).map(|i: u64| i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33).collect();
        u.sort_unstable();
        println!("{:?} {:?}", &u[..3], u.last());
        let mut bh: std::collections::BinaryHeap<u64> = u.iter().copied().collect();
        let top: Vec<u64> = (0..3).map(|_| bh.pop().unwrap()).collect();
        println!("{:?}", top);
    }
}

mod v03 {
    pub fn main() {
        let xs = [-7i32, 7, -1, 0, i32::MIN + 1];
        let ds = [2i32, -2, 3, -3];
        for x in xs { for d in ds { print!("{}/{}={},{} ", x, d, x / d, x % d); } println!(); }
        let a: i128 = -170141183460469231731687303715884105727;
        println!("{} {} {}", a / 7, a % 7, a.rem_euclid(7));
        let b: u128 = 340282366920938463463374607431768211455;
        println!("{} {} {}", b / 3, b % 1000000007, b / u64::MAX as u128);
        println!("{}", (b >> 1) as i128 * -1);
        println!("{}", (12345678901234567890123i128).pow(1) * 1000);
        println!("{:?}", 10i128.checked_pow(38));
        println!("{:?}", 10i128.checked_pow(39));
        println!("{}", u128::MAX.leading_zeros() + u128::MAX.trailing_ones() + (1u128 << 100).trailing_zeros());
        println!("{}", i128::MAX.count_ones());
        println!("{}", (-1i128) as u128 == u128::MAX);
        println!("{}", ((1u128 << 127) as f64));
        println!("{}", (-(1i128 << 100)) as f64);
        println!("{}", 1e38f64 as u128);
        println!("{}", -1e38f64 as i128);
        println!("{}", (u64::MAX as u128 * u64::MAX as u128));
        println!("{:?}", u128::MAX.checked_mul(2));
        println!("{:?}", (u128::MAX / 2).overflowing_mul(3));
        println!("{}", i128::MIN.wrapping_mul(-1));
        println!("{}", 0xFFFF_FFFF_FFFF_FFFF_FFFF_u128.swap_bytes());
        println!("{}", 1234567890123456789012345678u128.reverse_bits());
    }
}

mod v04 {
    pub fn main() {
        let big = vec![0u8; 3_000_000];
        println!("{}", big.len());
        let arr = [1u32; 100_000];
        println!("{}", arr.iter().sum::<u32>());
        let b: Box<[u64; 50_000]> = Box::new([7; 50_000]);
        println!("{}", b[49_999]);
        fn depth(n: u32) -> u32 { if n == 0 { 0 } else { 1 + depth(n - 1) } }
        println!("{}", depth(50_000));
        let mut v: Vec<Vec<u16>> = Vec::new();
        for i in 0..1000 { v.push((0..i % 50).collect()); }
        println!("{}", v.iter().map(|x| x.len()).sum::<usize>());
        let s: String = (0..100_000).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
        println!("{} {}", s.len(), &s[s.len() - 5..]);
        let mut h = std::collections::HashMap::new();
        for i in 0..100_000u64 { *h.entry(i % 1000).or_insert(0u64) += i; }
        println!("{} {:?}", h.len(), h.get(&999));
        let mut bt = std::collections::BTreeMap::new();
        for i in (0..50_000u32).rev() { bt.insert(i.wrapping_mul(2654435761) % 100_000, i); }
        println!("{} {:?} {:?}", bt.len(), bt.iter().next(), bt.iter().next_back());
    }
}

fn main() {
    println!("== v01");
    v01::main();
    println!("== v02");
    v02::main();
    println!("== v03");
    v03::main();
    println!("== v04");
    v04::main();
}
