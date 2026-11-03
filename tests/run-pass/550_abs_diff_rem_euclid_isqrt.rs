// abs_diff, rem_euclid/div_euclid, isqrt, ilog2/ilog10/ilog, pow/checked_pow, gcd/lcm, midpoint, div_ceil
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}
fn lcm(a: u64, b: u64) -> u64 {
    a / gcd(a, b) * b
}
fn ext_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        (a, 1, 0)
    } else {
        let (g, x, y) = ext_gcd(b, a.rem_euclid(b));
        (g, y, x - a.div_euclid(b) * y)
    }
}
fn mod_inv(a: i64, m: i64) -> Option<i64> {
    let (g, x, _) = ext_gcd(a, m);
    if g != 1 { None } else { Some(x.rem_euclid(m)) }
}

fn main() {
    for (a, b) in [(3u32, 10u32), (10, 3), (0, u32::MAX)] {
        print!("{} ", a.abs_diff(b));
    }
    println!();
    println!("{} {} {}", (-5i32).abs_diff(5), i64::MIN.abs_diff(0), (-128i8).abs_diff(127));
    for (a, b) in [(7i32, 3i32), (-7, 3), (7, -3), (-7, -3), (0, 5), (-1, 5)] {
        println!("{:>3} {:>3}: / {:>2} % {:>2} div_e {:>2} rem_e {:>2}", a, b, a / b, a % b, a.div_euclid(b), a.rem_euclid(b));
    }
    println!("{} {}", (-7.5f64).rem_euclid(2.0), (-7.5f64).div_euclid(2.0));
    println!("{} {}", 7.5f64 % -2.0, (-7.5f64) % 2.0);
    for n in [0u64, 1, 2, 3, 4, 15, 16, 17, 99, 100, 1 << 40, u64::MAX] {
        print!("{} ", n.isqrt());
    }
    println!();
    println!("{} {}", 1000000i32.isqrt(), i64::MAX.isqrt());
    println!("{:?}", (-4i32).checked_isqrt());
    for n in [1u32, 2, 7, 8, 9, 1000, 1023, 1024, u32::MAX] {
        print!("{}:{}/{} ", n, n.ilog2(), n.ilog10());
    }
    println!();
    println!("{} {} {}", 81u32.ilog(3), 80u32.ilog(3), 1000000u64.ilog(100));
    println!("{:?} {:?}", 0u32.checked_ilog2(), 0u8.checked_ilog10());
    println!("{} {}", 3u64.pow(40), 2i32.pow(30));
    println!("{:?} {:?}", 3u64.checked_pow(41), 3u64.checked_pow(40));
    println!("{:?} {:?}", 2i8.checked_pow(7), (-2i8).checked_pow(7));
    println!("{} {}", 10u8.saturating_pow(3), (-10i8).saturating_pow(3));
    println!("{} {:?}", 3u8.wrapping_pow(7), 3u8.overflowing_pow(7));
    println!("{} {}", gcd(1071, 462), lcm(21, 6));
    println!("{}", (1..=20u64).fold(1, lcm));
    println!("{:?}", ext_gcd(240, 46));
    println!("{:?} {:?}", mod_inv(3, 11), mod_inv(6, 9));
    println!("{} {} {}", 7u32.div_ceil(3), 9u32.div_ceil(3), 0u32.div_ceil(5));
    println!("{} {} {}", 0u8.midpoint(255), 10i32.midpoint(-3), (-7i32).midpoint(-8));
    println!("{} {}", 1.0f64.midpoint(4.0), u64::MAX.midpoint(u64::MAX - 2));
    println!("{} {}", 13u32.next_multiple_of(5), 15u32.next_multiple_of(5));
    println!("{:?}", 13u32.checked_next_multiple_of(0));
    println!("{} {}", 255u8.is_power_of_two(), 128u8.is_power_of_two());
    println!("{} {}", (-17i32).signum() * (-17i32).abs(), (-17i32).unsigned_abs());
    println!("{} {}", i32::MIN.unsigned_abs(), (-5i8).pow(3));
    println!("{} {}", 17i32.rem_euclid(-5), (-17i32).rem_euclid(-5));
    println!("{:?} {:?}", 5i32.checked_rem_euclid(0), i32::MIN.checked_div_euclid(-1));
    println!("{}", i32::MIN.wrapping_rem_euclid(-1));
    let clock = |h: i32, d: i32| (h + d).rem_euclid(12);
    println!("{} {} {}", clock(10, 5), clock(2, -5), clock(0, -25));
    let ang = |a: f64| a.rem_euclid(360.0);
    println!("{} {} {}", ang(-90.0), ang(725.0), ang(360.0));
    println!("{}", (1u64..=30).map(|n| n.isqrt()).sum::<u64>());
    let perfect: Vec<u32> = (1..200u32).filter(|n| n.isqrt().pow(2) == *n).collect();
    println!("{:?}", perfect);
    let digits = |n: u64| if n == 0 { 1 } else { n.ilog10() + 1 };
    println!("{} {} {} {}", digits(0), digits(9), digits(10), digits(u64::MAX));
    let bits_needed = |n: u32| u32::BITS - n.leading_zeros();
    println!("{} {} {}", bits_needed(0), bits_needed(1), bits_needed(1000));
    println!("{}", i128::MAX.isqrt());
    let primes: Vec<u32> = (2..60u32).filter(|&n| (2..=n.isqrt()).all(|d| n % d != 0)).collect();
    println!("{:?}", primes);
    let tri = |n: u64| n * (n + 1) / 2;
    let is_tri = |x: u64| {
        let n = (8 * x + 1).isqrt();
        n * n == 8 * x + 1
    };
    println!("{:?}", (1..12).map(tri).collect::<Vec<_>>());
    println!("{:?}", [1u64, 2, 3, 6, 10, 11, 15, 21].map(is_tri));
    let mean_floor = |a: i32, b: i32| (a + b).div_euclid(2);
    println!("{} {} {}", mean_floor(-3, 0), mean_floor(3, 0), mean_floor(-5, -6));
}
