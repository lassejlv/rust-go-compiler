mod f01 {
    fn takes_u64(x: u64) -> u64 { x }
    fn takes_f32(x: f32) -> f32 { x }
    pub fn main() {
        let mut x = 1;
        for _ in 0..12 { x = x * 10; }
        println!("{}", takes_u64(x));
        let a = 1.0;
        let third = a / 3.0;
        println!("{} {}", third, takes_f32(a));
        let f = |x| x * 2;
        println!("{}", f(200u8 / 2));
        let s: u8 = 3;
        let sh = 1u64 << s;
        let sh2 = 1 << s;
        let k: i64 = sh2;
        println!("{} {}", sh, k);
        let v = vec![1, 2, 3];
        let total: u128 = v.iter().sum();
        println!("{}", total * u64::MAX as u128);
        let mut acc = 0;
        acc += v.len();
        println!("{}", acc.pow(20) > 0);
        let neg = -1;
        let as_u = neg as u32;
        println!("{}", as_u);
        let big = 3_000_000_000u32;
        let y = big as i32;
        println!("{}", y);
        let m = [0.1, 0.2];
        let mf: f32 = m.iter().sum();
        println!("{}", mf);
        let z = 255;
        let zz = z as u8 as i8;
        println!("{}", zz);
        let q = 7 / 2 * 2;
        let r = 7.0 / 2.0 * 2.0;
        println!("{} {}", q, r);
        let e = 2u32.pow(31) as i32;
        println!("{}", e);
        let idx = 5;
        let arr = [10u8; 8];
        println!("{}", arr[idx] as u32 * 1000);
        let mut counts = [0; 4];
        counts[1] += 300;
        let cs: [u16; 4] = counts;
        println!("{:?}", cs);
        let w = 0xFFi64;
        println!("{}", (w << 56) >> 56);
        let cmp = 1 < 2u8 as i32;
        println!("{}", cmp);
        let unit_f = |a: f64, b| a + b;
        println!("{}", unit_f(1.0, 2.0));
        let lens: Vec<_> = vec!["a", "bb"].iter().map(|s| s.len() as u8).collect();
        let lsum = lens.iter().fold(250, |a, &b| a + b as u32);
        println!("{}", lsum);
        let i = -7i8;
        println!("{} {}", i >> 1, (i as u8) >> 1);
        println!("{}", (-7i32).wrapping_rem(3));
        println!("{}", 10_i8.wrapping_mul(13));
        println!("{:?}", (u8::MAX).checked_next_power_of_two());
        println!("{}", 17u32.next_power_of_two());
        println!("{}", (0.1f32 as f64 == 0.1f64));
    }
}

mod f02 {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(u8)]
    enum Op { Nop = 0, Inc = 1, Dbl = 2, Neg = 0x80 }
    impl TryFrom<u8> for Op { type Error = u8; fn try_from(b: u8) -> Result<Op, u8> { Ok(match b { 0 => Op::Nop, 1 => Op::Inc, 2 => Op::Dbl, 0x80 => Op::Neg, x => return Err(x) }) } }
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Signed { A = -2, B, C = 5, D }
    pub fn main() {
        let bytes = [1u8, 2, 0x80, 7, 0];
        let ops: Vec<Result<Op, u8>> = bytes.iter().map(|&b| Op::try_from(b)).collect();
        println!("{:?}", ops);
        let mut v: i32 = 1;
        for op in ops.iter().flatten() { match op { Op::Nop => {} Op::Inc => v += 1, Op::Dbl => v *= 2, Op::Neg => v = -v } }
        println!("{}", v);
        println!("{} {} {} {}", Signed::A as i32, Signed::B as i32, Signed::C as i8, Signed::D as u64);
        println!("{}", Op::Neg as u8 as i8);
        let mut sorted = vec![Op::Neg, Op::Nop, Op::Dbl];
        sorted.sort();
        println!("{:?} {}", sorted, std::mem::size_of::<Op>());
        println!("{}", std::mem::size_of::<Option<Op>>());
        println!("{}", std::mem::size_of::<Option<Box<u8>>>());
        println!("{}", std::mem::size_of::<Option<&u8>>() == std::mem::size_of::<&u8>());
        #[derive(Debug)] enum Big { X(u64), Y(u8, u8), Z }
        println!("{} {:?}", std::mem::size_of::<Big>(), [Big::X(1), Big::Y(2, 3), Big::Z]);
    }
}

fn main() {
    println!("== f01");
    f01::main();
    println!("== f02");
    f02::main();
}
