// Float formatting with precision, rounding behavior, special values, f32 vs f64 printing,
// total_cmp sorting, lerp/clamp helpers used by layout code, and parsing round trips.

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps * a.abs().max(b.abs()).max(1.0)
}

fn mean_std(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let m = xs.iter().sum::<f64>() / n;
    let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0);
    (m, v.sqrt())
}

fn main() {
    let vals = [0.0, -0.0, 1.0, 0.1, 1.0 / 3.0, 2.0 / 3.0, 123456.789, 1e-7, 1e21, 1e15, 5e-324, f64::MAX, f64::MIN_POSITIVE, f64::EPSILON];
    for v in vals {
        println!("{:<24} {:<12.3} {:<14e} {:?}", v, v, v, v);
    }
    let fvals = [0.1f32, 1.0 / 3.0, 16777217.0, 3.4028235e38, 1e-45, 0.3];
    for v in fvals {
        println!("f32 {} {:?} {:.10} as f64 {}", v, v, v, v as f64);
    }
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        println!("[{}] [{:?}] [{:>6}] [{:<6.2}] [{:+}]", v, v, v, v, v);
    }
    // rounding of halfway cases in formatting (round-half-to-even on the exact binary value)
    for v in [0.125, 0.375, 2.5, 3.5, 0.045, 1.005, 2.675, -0.5, -1.5] {
        println!("{:>6} -> {:.0} {:.1} {:.2}", v, v, v, v);
    }
    println!("{:.3} {:.20}", 1e-10, 0.1);
    println!("{:10.4}|{:<10.2}|{:^10.1}|{:+.3}|{:08.2}", 3.14159265, 2.71828, 1.41421, 1.0, -3.14159);
    println!("{} {} {} {}", 2.5f64.round(), (-2.5f64).round(), 2.5f64.round_ties_even(), 0.1 + 0.2);
    println!("{} {} {} {}", 7.9f64.floor(), (-7.1f64).ceil(), (-7.9f64).trunc(), 7.25f64.fract());
    println!("{} {} {}", 10f64.ln(), 2f64.powf(0.5), 1e3f64.log10());
    println!("{} {} {}", (1.0f64).atan2(1.0), std::f64::consts::PI.sin().abs() < 1e-15, 0.5f64.acos());
    println!("{} {} {}", f64::NAN.max(1.0), 1.0f64.min(f64::NAN), f64::NAN == f64::NAN);
    println!("{} {} {}", (0.0f64).signum(), (-0.0f64).signum(), (-0.0f64).is_sign_negative());
    println!("{} {}", 1.5f64.mul_add(2.0, 0.25), 9f64.sqrt().hypot(4.0));
    println!("{} {}", f64::from_bits(0x3ff8000000000000), 1.5f64.to_bits());
    println!("{} {}", f32::from_bits(0x40490fdb), (-1.0f32).to_bits());

    let mut xs = vec![3.5, -0.0, f64::NAN, 0.0, -2.25, f64::INFINITY, 1e-300, f64::NEG_INFINITY];
    xs.sort_by(|a, b| a.total_cmp(b));
    println!("{:?}", xs);
    let ys = [3.0f64, 1.0, 2.0];
    let max = ys.iter().cloned().fold(f64::MIN, f64::max);
    println!("max {} clamp {} {}", max, 5.5f64.clamp(0.0, 1.0), (-3.0f32).clamp(-1.0, 1.0));
    println!("{:?}", (0..=4).map(|i| lerp(10.0, 20.0, i as f32 / 4.0)).collect::<Vec<_>>());
    let (m, s) = mean_std(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
    println!("mean {:.4} std {:.4}", m, s);
    println!("{} {}", approx_eq(0.1 + 0.2, 0.3, 1e-12), approx_eq(1.0, 1.001, 1e-6));
    for s in ["3.14", "-0", "1e10", "inf", "-infinity", "NaN", "1_0", ".5", "5.", "0x10", "1e400", "abc"] {
        match s.parse::<f64>() {
            Ok(v) => println!("parse {:<10} = {:?}", s, v),
            Err(e) => println!("parse {:<10} err {}", s, e),
        }
    }
    let mut acc = 0.0f32;
    for _ in 0..10 {
        acc += 0.1;
    }
    println!("{} {} {}", acc, acc == 1.0, (acc - 1.0).abs() < f32::EPSILON * 4.0);
    println!("{} {} {}", 255.0f64 as u8, 256.7f64 as u8, -5.0f64 as u8);
    println!("{} {}", i64::MAX as f64, u64::MAX as f32);
    println!("{} {}", (0.1f32 + 0.2f32) as f64, 0.1f32 as f64 + 0.2f32 as f64);
}
