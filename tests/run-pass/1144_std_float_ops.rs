// Breadth test of f64/f32 operations and formatting edge cases.
use std::num::FpCategory;

fn main() {
    let vals = [0.0, -0.0, 1.0, -1.5, 0.1, 1.0 / 3.0, 2.5e-310, f64::MIN_POSITIVE, f64::MAX, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1e21, 1e-7, 123456789.125];
    for v in vals {
        println!("{:<24} {:<24?} {:<14e} {:<10.3} {:?}", v, v, v, v, v.classify());
    }
    println!("f32: {} {:?} {} {} {:e}", 0.1f32, 1.0f32 / 3.0, f32::MAX, f32::MIN_POSITIVE, 16777217.0f32);
    println!("f32 as f64: {} {}", 0.1f32 as f64, (0.1f32 as f64) as f32);
    let mut sorted = vec![3.0, f64::NAN, -0.0, 0.0, -f64::NAN, f64::INFINITY, -1.0, f64::NEG_INFINITY];
    sorted.sort_by(|a, b| a.total_cmp(b));
    println!("total_cmp sort: {:?}", sorted);
    println!("total_cmp -0 vs 0: {:?} partial_cmp nan: {:?} eq: {} {}", (-0.0f64).total_cmp(&0.0), f64::NAN.partial_cmp(&1.0), 0.0 == -0.0, f64::NAN == f64::NAN);
    println!("clamp {} {} {} {}", 5.5f64.clamp(0.0, 1.0), (-2.0f64).clamp(-1.0, 1.0), 0.5f64.clamp(0.0, 1.0), f64::NAN.clamp(0.0, 1.0).is_nan());
    println!("min/max {} {} {} {}", 1.0f64.min(f64::NAN), f64::NAN.max(2.0), (-3.0f64).min(0.5), 0.25f64.max(-7.0));
    println!("mul_add {} {} fma-vs-naive {}", 2.0f64.mul_add(3.0, 4.0), 0.1f64.mul_add(10.0, -1.0), 0.1f64 * 10.0 - 1.0);
    println!("to_bits {:#018x} {:#018x} {:#x} from_bits {}", 1.0f64.to_bits(), (-0.0f64).to_bits(), 1.5f32.to_bits(), f64::from_bits(0x4009_21FB_5444_2D18));
    println!("nan bits is_nan {} sign {}", f64::from_bits(0x7ff8_0000_0000_0001).is_nan(), (-f64::NAN).is_sign_negative());
    println!("sign: {} {} {} {} copysign {} {}", (-0.0f64).is_sign_negative(), 0.0f64.is_sign_positive(), (-3.0f64).signum(), (-0.0f64).signum(), 3.0f64.copysign(-0.0), (-2.0f64).copysign(1.0));
    println!("rounding: {} {} {} {} {} {}", 2.5f64.round(), -2.5f64.round(), 2.5f64.round_ties_even(), 3.5f64.round_ties_even(), (-2.7f64).trunc(), (-2.2f64).floor());
    println!("ceil {} fract {} {} abs {}", (-2.2f64).ceil(), 3.75f64.fract(), (-3.75f64).fract(), (-0.0f64).abs());
    println!("sqrt {} {} cbrt {} powi {} powf {} {}", 2.0f64.sqrt(), (-1.0f64).sqrt().is_nan(), (-27.0f64).cbrt(), 2.0f64.powi(-2), 2.0f64.powf(0.5), 0.0f64.powf(0.0));
    println!("exp {} ln {} log10 {} log2 {} log {} ln_1p {} exp_m1 {}", 1.0f64.exp(), std::f64::consts::E.ln(), 1000.0f64.log10(), 1024.0f64.log2(), 81.0f64.log(3.0), 1e-10f64.ln_1p(), 1e-10f64.exp_m1());
    println!("trig {:.6} {:.6} {:.6} {:.6} {:.6}", (std::f64::consts::PI / 6.0).sin(), 0.0f64.cos(), 1.0f64.atan(), 1.0f64.atan2(-1.0), 0.5f64.asin());
    println!("hyperbolic {:.6} {:.6} {:.6} hypot {}", 1.0f64.sinh(), 1.0f64.cosh(), 0.5f64.tanh(), 3.0f64.hypot(4.0));
    println!("to_degrees {} to_radians {:.6} recip {} {}", std::f64::consts::FRAC_PI_2.to_degrees(), 180.0f64.to_radians(), 4.0f64.recip(), 0.0f64.recip());
    println!("rem/euclid {} {} {} {}", -7.5f64 % 2.0, (-7.5f64).rem_euclid(2.0), (-7.5f64).div_euclid(2.0), 7.5f64 % -2.0);
    println!("inf arith {} {} {} {}", f64::INFINITY - f64::INFINITY, 1.0 / 0.0, -1.0 / 0.0, 0.0 / 0.0);
    println!("is_finite {} is_infinite {} is_normal {} {} is_subnormal {}", 1e308f64.is_finite(), (1e308f64 * 10.0).is_infinite(), 1e-308f64.is_normal(), 1e-310f64.is_normal(), 1e-310f64.is_subnormal());
    let cats: Vec<FpCategory> = [0.0, 1e-320, 1.0, f64::INFINITY, f64::NAN].iter().map(|x: &f64| x.classify()).collect();
    println!("classify {:?}", cats);
    println!("EPSILON {:e} next_up {} next_down {}", f64::EPSILON, 1.0f64.next_up(), 1.0f64.next_down());
    println!("precision: [{:.0}] [{:.1}] [{:.20}] [{:.3e}] [{:10.2e}] [{:<+12.4}]", 0.5, 0.05, 0.1, 123456.0, 0.000123, 3.14159);
    println!("rounding fmt: {:.0} {:.0} {:.0} {:.1} {:.1} {:.2}", 1.5, 2.5, -0.5, 0.25, 0.35, 1.005);
    println!("signed zero fmt: {} {:?} {:.2} {:+} {:e}", -0.0f64, -0.0f64, -0.0f64, 0.0f64, -0.0f64);
    println!("nan/inf fmt: [{:>6}] [{:<6}] [{:+}] [{:.3}] [{:e}] [{:08}] [{:08}]", f64::NAN, f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -1.5, f64::INFINITY);
    println!("subnormals: {:e} {:?} {} {:e}", f64::from_bits(1), f64::from_bits(1), 5e-324, f64::MIN_POSITIVE / 2.0);
    println!("big/small display: {} {} {} {}", 1e15, 1e16, 1.5e-5, 123e300);
    println!("debug big/small: {:?} {:?} {:?} {:?} {:?}", 1e15, 1e16, 1.5e-5, 1e-4, 0.3f32);
    println!("{:E} {:e} {:.0e} {:e}", 1500.0, 0.00012, 2.5, 1.0f32);
    for s in ["1e308", "1.8e308", "-1e-400", "0.1e1", "  2", "2.", ".", "e5", "1e", "infinity", "-NaN", "0x1p3", "1_0"] {
        print!("{:?}->{:?} ", s, s.parse::<f64>().ok());
    }
    println!();
    println!("f32 parse round {:?} {:?}", "3.4028236e38".parse::<f32>(), "16777217".parse::<f32>());
    println!("sum precision {} {} {}", (0..10).map(|_| 0.1).sum::<f64>(), [1e16, 1.0, -1e16].iter().sum::<f64>(), 0.1 + 0.2 == 0.3);
    println!("as conversions {} {} {} {} {}", 255.9f64 as u8, -1.0f64 as u32, f64::INFINITY as i16, 1e10 as f32, u64::MAX as f64);
    println!("max/min by total_cmp {:?}", [1.0, f64::NAN, 3.0].iter().copied().max_by(f64::total_cmp));
    let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64).sqrt();
    println!("mean {} sd {} ", mean, sd);
    println!("lerp-ish {:.4} {:.4}", 0.25f64.mul_add(10.0 - 2.0, 2.0), f64::consts_check());
}

trait ConstsCheck { fn consts_check() -> f64; }
impl ConstsCheck for f64 {
    fn consts_check() -> f64 { std::f64::consts::TAU / std::f64::consts::PI + std::f64::consts::SQRT_2 * std::f64::consts::FRAC_1_SQRT_2 + std::f64::consts::LN_2.exp() }
}
