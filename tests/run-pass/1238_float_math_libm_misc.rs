// libm-backed float functions (exp_m1, ln_1p, asinh, acosh, ...) and assorted std APIs
use std::time::Duration;
use std::num::{NonZeroU32, NonZeroU8, Wrapping, Saturating};
use std::ops::{ControlFlow, Bound, RangeBounds};
fn misc_apis() {
    let d = Duration::from_millis(1500);
    println!("{:?} {:?} {:?}", d, Duration::from_micros(1), Duration::from_nanos(1_000_000_123));
    println!("{:?} {:?}", Duration::from_secs(90) + d, d * 3);
    println!("{} {} {}", d.as_secs(), d.subsec_millis(), d.as_secs_f64());
    println!("{:?} {:?}", d.checked_sub(Duration::from_secs(2)), Duration::from_secs_f32(0.25));
    let total: Duration = [d, d, Duration::from_millis(1)].iter().sum();
    println!("{:?} {:?}", total, Duration::ZERO);
    println!("{:.2?} {:?}", Duration::from_nanos(123456789), Duration::from_secs(3600));
    let nz = NonZeroU32::new(5).unwrap();
    println!("{} {:?} {:?}", nz, NonZeroU8::new(0), std::mem::size_of::<Option<NonZeroU32>>());
    println!("{}", nz.get() * 2);
    let w = Wrapping(250u8) + Wrapping(10);
    let s = Saturating(250u8) + Saturating(10);
    println!("{} {} {:?}", w, s, Wrapping(3i8) * Wrapping(100));
    let r = (1..10).try_for_each(|x| if x * x > 30 { ControlFlow::Break(x) } else { ControlFlow::Continue(()) });
    println!("{:?}", r);
    let r2: Result<(), String> = ["1", "2", "x"].iter().try_for_each(|s| s.parse::<i32>().map(|_| ()).map_err(|e| format!("{}: {}", s, e)));
    println!("{:?}", r2);
    println!("{} {} {}", (1..=0).is_empty(), (3..=5).contains(&5), (3..5).contains(&5));
    println!("{:?} {:?}", (0..10).rev().nth(2), (0..=10).rev().step_by(4).collect::<Vec<_>>());
    let rb = 2..;
    println!("{:?} {:?}", rb.start_bound(), (..=7).end_bound());
    println!("{:?}", matches!((..3).start_bound(), Bound::Unbounded));
    println!("{} {} {}", 'z'.to_digit(36).unwrap(), 'A'.is_ascii_graphic(), b'7'.is_ascii_hexdigit());
    println!("{} {}", 7i32.abs_diff(-3), std::f64::consts::PI.to_degrees());
    println!("{:?}", "a1b22c333".bytes().rposition(|b| b == b'2'));
    println!("{:?}", String::from_iter(['x', 'y']));
    let arr = [String::from("p"), String::from("q")];
    let mut it = arr.into_iter();
    println!("{:?} {:?}", it.next_back(), it.as_slice());
    println!("{:?}", u32::from_str_radix("7fffffff", 16));
    println!("{:?}", i64::from_str_radix("-1010", 2));
    println!("{:?}", 1.5f32.mul_add(2.0, 0.5));
    println!("{} {}", f64::MAX.sqrt(), (-1f64).sqrt().is_nan());
    println!("{} {}", 2f64.ln(), 8f64.log2());
    println!("{:?}", (0.0f64).asin());
    println!("{}", 1e-3f64.ln_1p());
    println!("{:?}", 100u8.checked_div(0));
    println!("{:?}", (-128i8).checked_div(-1));
    println!("{}", u64::MAX.count_zeros());
    println!("{:?}", [1, 2, 3].iter().copied().reduce(|a, b| a.max(b)));
    println!("{:?}", std::iter::empty::<u8>().max());
    println!("{:?}", (1..=4).map(|x| x as f32).sum::<f32>());
    println!("{:?}", (1..=3).flat_map(|x| std::iter::repeat(x).take(x)).collect::<Vec<_>>());
}

fn float_math() {
    let xs64 = [-3.7f64, -1.0, -0.5, -1e-9, 0.0, 1e-7, 0.3, 0.999, 1.0, 1.5, 2.0, 7.25, 100.0, 1e10, 1e300];
    for &x in &xs64 {
        println!("{:e}: {:?} {:?} {:?} {:?} {:?} {:?} {:?}", x, x.exp_m1(), x.ln_1p(), x.asinh(), x.acosh(), x.atanh(), x.cbrt(), x.powi(7));
        println!("  {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?}", x.sinh(), x.cosh(), x.tanh(), x.exp(), x.ln(), x.powf(1.7), x.powi(-3), x.hypot(2.5));
        println!("  {:?} {:?} {:?} {:?} {:?} {:?}", x.sin(), x.cos(), x.tan(), x.atan(), x.exp2(), x.log(3.0));
    }
    let xs32 = [-3.7f32, -0.5, 1e-6, 0.3, 1.0, 1.5, 7.25, 100.0, 1e20];
    for &x in &xs32 {
        println!("{:e}: {:?} {:?} {:?} {:?} {:?} {:?} {:?}", x, x.exp_m1(), x.ln_1p(), x.asinh(), x.acosh(), x.atanh(), x.cbrt(), x.powi(5));
        println!("  {:?} {:?} {:?} {:?} {:?} {:?} {:?}", x.sinh(), x.cosh(), x.tanh(), x.exp(), x.ln(), x.powf(0.3), x.powi(-2));
        println!("  {:?} {:?} {:?} {:?} {:?} {:?}", x.sin(), x.cos(), x.tan(), x.atan2(1.5), x.log10(), x.log2());
    }
    println!("{:?} {:?}", 1.0000001f64.powi(1000000), 3.0f64.powi(-40));
    println!("{:?} {:?}", 0.1f32.powi(13), 2.0f64.powi(1023));
    println!("{:?}", (1.0f64 / 3.0).powi(i32::MIN));
    println!("{:?} {:?}", f64::NAN.powi(0), 0.0f64.powi(-1));
}

fn main() {
    misc_apis();
    float_math();
}
