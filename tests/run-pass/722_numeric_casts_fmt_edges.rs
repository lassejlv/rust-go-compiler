// Numeric casts (saturating float->int, truncation, sign changes, int->float
// rounding), float Display/Debug/exp formatting, rounding modes for {:.N},
// and integer {:e} formatting.
fn main() {
    let floats = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.5, 255.5, 256.0, 1e20, -1e20, 0.999, -0.999];
    for f in floats {
        println!("{:>8} -> u8 {:>3} i8 {:>4} i32 {:>11} u64 {:>20} i128 {}", f, f as u8, f as i8, f as i32, f as u64, f as i128);
    }
    let ints: [i64; 6] = [-1, 255, 256, -129, i64::MAX, i64::MIN];
    for i in ints {
        println!("{:>20} as u8={:>3} i8={:>4} u16={:>5} u32={:>10} f32={:e} f64={}", i, i as u8, i as i8, i as u16, i as u32, i as f32, i as f64);
    }
    println!("u64::MAX as f32 = {} as f64 = {}", u64::MAX as f32, u64::MAX as f64);
    println!("16777217 as f32 = {} 9007199254740993 as f64 = {}", 16777217i32 as f32, 9007199254740993i64 as f64);
    println!("char casts: {} {} {} {:?}", 'A' as u8, 'é' as u8, '€' as u32, 97u8 as char);
    println!("bool casts: {} {}", true as i32, false as u8);
    println!("f32<->f64: {} {} {}", 0.1f32 as f64, 0.1f64 as f32, 1e40f64 as f32);

    // rounding: Rust uses round-half-to-even on the exact decimal value
    for v in [0.5, 1.5, 2.5, -0.5, 0.125, 0.375, 2.675, 1.005, 0.045] {
        print!("[{} {:.0} {:.1} {:.2}] ", v, v, v, v);
    }
    println!();
    // Display of extreme magnitudes
    for v in [1e15, 1e16, 1e17, 1.5e300, 1e-5, 1e-7, 123456.789e-10, f64::MAX, f64::MIN_POSITIVE, 5e-324] {
        println!("{} | {:?} | {:e} | {:.3e} | {:E}", v, v, v, v, v);
    }
    for v in [1.0f32, 0.1, 16777216.0, 3.4028235e38, 1e-45, 7.0e-3] {
        println!("f32 {} {:?} {:e}", v, v, v);
    }
    // integer exponent formatting
    println!("{:e} {:e} {:E} {:e} {:e}", 1234, 1200u32, 5i8, 0, -1000);
    println!("{:.2e} {:.0e} {:e}", 123456, 98765, u64::MAX);
    // width / sign / zero padding combos
    let x = -3.14159;
    println!("[{:10.3}] [{:<10.3}] [{:^10.3}] [{:+010.2}] [{:010.2}] [{:+}] [{:+.1}]", x, x, x, -x, x, 2.5, 0.0);
    println!("[{:08.3e}] [{:+e}] [{:12e}] [{:<12.1e}]", 1234.5, 0.00042, -5.0, 9.99);
    println!("[{:#x}] [{:#010x}] [{:#b}] [{:#o}] [{:x}] [{:X}] [{:#X}]", 255, 255, 5, 64, -1i8, 3054i32, u128::MAX);
    println!("[{:>+6}] [{:<+6}] [{:^+7}] [{:+06}] [{:06}] [{:<06}]", 42, 42, 42, -42, -42, 7);
    let w = 9;
    let p = 2;
    println!("[{:w$.p$}] [{:>1$}] [{:.*}] [{:>w$}] [{:^w$.p$e}]", 1.23456, 7, 3, 2.71828, "abc", 31415.9);
    println!("[{:5}] [{:<5}] [{:^5}] [{:>5}] [{:*^9}] [{:->6}]", 'x', true, "ab", "", "mid", 1.5);
    println!("{:?} {:?} {:?} {:?}", 0.1 + 0.2, 1.0 / 3.0, 2.0f64.sqrt(), (0.1f32 + 0.2f32) as f64);
    println!("{} {} {} {}", f64::EPSILON, f32::EPSILON, -0.0f64, 0.0f64 == -0.0);
    println!("{} {} {}", (2.5f64).round(), (-2.5f64).round(), (2.5f64).round_ties_even());
    println!("{} {} {} {}", 7.0f64.rem_euclid(-3.0), (-7.0f64) % 3.0, (-7i32).rem_euclid(3), (-7i32).div_euclid(3));
    println!("{} {} {}", f64::MAX + f64::MAX, (-1.0f64).sqrt().is_nan(), 1.0f64 / 0.0);
    println!("{:?} {:?}", (1.0f64).max(f64::NAN), f64::NAN.min(2.0));
    println!("{} {} {}", 1e300 * 1e10, 2.0f64.powi(-1074), 2.0f64.powi(-1075));
    println!("total_cmp sort: {:?}", {
        let mut v = vec![3.0, -0.0, f64::NAN, 0.0, -1.0, f64::NEG_INFINITY];
        v.sort_by(|a, b| a.total_cmp(b));
        v
    });
}
