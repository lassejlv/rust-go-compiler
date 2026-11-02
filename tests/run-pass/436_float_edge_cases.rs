// Float edge cases: NaN comparisons, total_cmp sorting with -0.0/NaN, min/max with NaN,
// signum/copysign/is_sign_negative of -0.0, rounding modes (round, round_ties_even, floor, ceil,
// trunc, fract), float->int saturating casts, to_bits/from_bits, clamp, mul_add, hypot, rem.
fn main() {
    let nan = f64::NAN;
    println!("{} {} {} {}", nan == nan, nan != nan, nan < 1.0, nan > 1.0);
    println!("{:?} {:?}", nan.partial_cmp(&1.0), 1.0f64.partial_cmp(&2.0));
    println!("{} {} {}", nan.max(1.0), 1.0f64.min(nan), nan.max(nan).is_nan());
    println!("{} {} {}", f64::INFINITY > f64::MAX, -f64::INFINITY < f64::MIN, f64::INFINITY - f64::INFINITY);

    let z = 0.0f64;
    let nz = -0.0f64;
    println!("{} {} {}", z == nz, nz.is_sign_negative(), z.is_sign_negative());
    println!("{} {} {} {}", nz.signum(), z.signum(), nan.signum().is_nan(), 1.0f64.copysign(nz));
    println!("{} {} {}", 1.0 / z, 1.0 / nz, (nz * 0.0).is_sign_negative());
    println!("{:?} {:?}", z.total_cmp(&nz), nz.total_cmp(&z));

    let mut v = vec![3.5, -0.0, f64::NAN, 0.0, -f64::INFINITY, 1e-300, -2.0, f64::INFINITY, -f64::NAN];
    v.sort_by(|a, b| a.total_cmp(b));
    println!("{:?}", v);
    let mut w = vec![2.5f32, -1.0, 0.5, 9.0];
    w.sort_by(|a, b| b.partial_cmp(a).unwrap());
    println!("{:?}", w);
    let maxf = [1.0, 5.5, -3.0, 2.0].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("max via fold {}", maxf);

    println!("round:");
    for x in [-2.5f64, -1.5, -0.5, 0.5, 1.5, 2.5, 2.4999, -0.49, 3.7, -3.7] {
        println!(
            "{:>7}: round={:>3} even={:>3} floor={:>3} ceil={:>3} trunc={:>3} fract={:.4}",
            x, x.round(), x.round_ties_even(), x.floor(), x.ceil(), x.trunc(), x.fract()
        );
    }

    println!("casts:");
    for x in [f64::NAN, f64::INFINITY, -f64::INFINITY, 1e20, -1e20, 255.9, -1.9, 300.0, -0.99] {
        println!("{:>8}: u8={:>3} i8={:>4} i32={:>11} u64={}", x, x as u8, x as i8, x as i32, x as u64);
    }
    println!("{} {}", (2.9f32 as i32), (-2.9f32 as i32));
    println!("{} {}", u64::MAX as f32, i64::MIN as f32);

    println!("bits:");
    println!("{:#018x} {:#018x} {:#x}", 1.0f64.to_bits(), (-0.0f64).to_bits(), 1.0f32.to_bits());
    println!("{} {}", f64::from_bits(0x400921fb54442d18), f32::from_bits(0x7f800000));
    println!("{}", f64::from_bits(f64::NAN.to_bits()).is_nan());
    let next = f64::from_bits(1.0f64.to_bits() + 1);
    println!("next after 1.0 = {:e} diff={:e}", next, next - 1.0);
    println!("{} {}", f64::MIN_POSITIVE / 2.0 > 0.0, (f64::MIN_POSITIVE / 2.0).is_normal());
    println!("{:?} {:?}", (1e-310f64).classify(), nan.classify());

    println!("misc:");
    println!("{} {} {}", 5.5f64.clamp(0.0, 5.0), (-1.0f64).clamp(0.0, 5.0), nan.clamp(0.0, 1.0).is_nan());
    println!("{} {}", 2.0f64.mul_add(3.0, 1.0), 3.0f64.hypot(4.0));
    println!("{} {} {}", 7.5f64 % 2.0, -7.5f64 % 2.0, (-7.5f64).rem_euclid(2.0));
    println!("{} {}", (-7.5f64).div_euclid(2.0), 7.5f64.div_euclid(-2.0));
    println!("{} {} {}", 0.1f64 + 0.2 == 0.3, ((0.1f64 + 0.2) - 0.3).abs() < f64::EPSILON, 1e16 + 1.0 == 1e16);
    println!("{} {} {}", 2.0f64.powf(0.5), (-8.0f64).powf(1.0 / 3.0).is_nan(), 0.0f64.powf(0.0));
    println!("{} {} {}", f64::NAN.powi(0), 1.0f64.powf(f64::NAN), (-1.0f64).sqrt().is_nan());
    println!("{} {}", 1e308f64 * 10.0, -1e308f64 * 10.0);
    println!("{} {}", 100.0f64.ln().exp().round(), 1000.0f64.log10());
    println!("{:.6} {:.6} {:.6}", (1.0f64).atan2(1.0), std::f64::consts::E.ln(), 0.5f64.asin());
    println!("{} {}", 1.0f64.to_degrees(), 180.0f64.to_radians());
    println!("{} {}", f32::MAX + f32::MAX, (f32::MAX as f64) * 2.0);
    println!("{} {} {}", 0.1f32 + 0.2f32, 0.1f32 + 0.2f32 == 0.3f32, 1.0f32 / 3.0);
    println!("{} {}", (0.1f32 as f64) == 0.1f64, 16777217i32 as f32);
    println!("{:?}", [1.0f64, -1.0, 0.0].map(|x: f64| x.recip()));
    let inf_sum: f64 = [1.0, f64::INFINITY, -1.0].iter().sum();
    let nan_sum: f64 = [f64::INFINITY, f64::NEG_INFINITY].iter().sum();
    println!("{} {}", inf_sum, nan_sum);
    println!("{} {}", f64::NAN.min(f64::NAN).is_nan(), f64::maximum_test());
}

trait MaxTest {
    fn maximum_test() -> String;
}
impl MaxTest for f64 {
    fn maximum_test() -> String {
        format!("{}|{}", (-0.0f64).max(0.0), 0.0f64.min(-0.0))
    }
}
