// Float formatting: Display shortest round-trip, precision rounding (banker's? no: round-half-even
// on exact binary value), {:e}/{:E} with precision, sign/zero-pad/width combos, f32 vs f64,
// subnormals, very large/small magnitudes, NaN/inf with width and sign, Debug of floats.
fn main() {
    let xs: [f64; 12] = [0.0, -0.0, 1.0, 0.1, 0.3, 1.5, 2.5, 100.0, 1e15, 1e16, 1e-7, 123456.789];
    for x in xs {
        println!("{} | {:?} | {:.2} | {:e} | {:10.3e} | {:+08.2}", x, x, x, x, x, x);
    }

    let f32s: [f32; 6] = [0.1, 1.0 / 3.0, 16777216.0, 3.4028235e38, 1.17549435e-38, 7.0e-45];
    for x in f32s {
        println!("{} {:?} {:e} {:.10}", x, x, x, x);
    }

    // rounding at precision boundaries
    for x in [0.125f64, 0.375, 0.5, 1.5, 2.5, -0.5, 0.045, 1.005, 9.995, 99.5] {
        println!("{:>6} -> .0:{:.0} .1:{:.1} .2:{:.2}", x, x, x, x);
    }

    // width / alignment / fill
    let pi = std::f64::consts::PI;
    println!("[{:10.4}] [{:<10.4}] [{:^10.4}] [{:>10.4}] [{:*^12.3}]", pi, pi, pi, pi, pi);
    println!("[{:010.3}] [{:+010.3}] [{:+.0}] [{:08.1e}]", -pi, pi, pi, pi);
    println!("[{:e}] [{:E}] [{:.0e}] [{:.12e}]", pi * 1e100, -pi * 1e-100, 1500.0, 1.0f64 / 7.0);

    // special values
    let specials = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::NAN];
    for s in specials {
        println!("[{}] [{:?}] [{:8}] [{:<8}] [{:+}] [{:08.2}] [{:e}]", s, s, s, s, s, s, s);
    }

    // extremes
    println!("{} {}", f64::MAX, f64::MIN_POSITIVE);
    println!("{:?} {:e}", f64::MAX, f64::MIN_POSITIVE);
    println!("{} {:?}", 5e-324f64, 5e-324f64);
    println!("{} {}", f64::EPSILON, f32::EPSILON);
    println!("{} {}", 1e21f64, 1e-5f64);
    println!("{:?} {:?} {:?}", 1e21f64, 1e-5f64, 1.0e7f32);
    println!("{} {}", 0.1f32 as f64, (0.1f64 as f32) as f64);
    println!("{} {}", 2.0f64.powi(60), 2.0f64.powi(-20));

    // arithmetic results printed
    println!("{} {} {}", 0.1 + 0.2, 1.0 - 0.9, 100.0 * 1.1);
    println!("{} {}", 1.0f64 / 3.0, 2.0f32 / 3.0);
    println!("{:.20}", 0.1f64);
    println!("{:.30}", 1e-10f64);
    println!("{:.3} {:.3}", f64::MAX, -f64::MIN_POSITIVE);

    // Debug in containers
    let v = vec![1.0, 2.5, -0.0, f64::NAN];
    println!("{:?}", v);
    println!("{:.1?}", v);
    println!("{:?}", (1.0f32, 2e30f32, 1e-30f64));
    #[derive(Debug)]
    struct P {
        x: f64,
        y: f32,
    }
    println!("{:?}", P { x: 1.0 / 0.0, y: -1.25 });
    println!("{:#?}", P { x: 1e100, y: 0.5 });
    println!("{:8.3?}|{:<8?}|", 1.23456, 2.0);

    // to_string and format! equivalence, parse round trip
    for x in [0.1f64, 1.0 / 3.0, 1e-300, 6.02214076e23] {
        let s = x.to_string();
        let back: f64 = s.parse().unwrap();
        println!("{} roundtrip={}", s, back == x);
    }
    let total: f64 = (1..=10).map(|i| 1.0 / i as f64).sum();
    println!("harmonic(10) = {} ~ {:.5}", total, total);

    // a small report table with mixed float formats
    let samples = [("north", 1234.5678f64, 0.0123f64), ("south", -0.5, 12.0), ("east", 1e9, 1e-9), ("west", 0.0, -0.0)];
    for (name, v, r) in samples {
        println!("{:<6}|{:>14.2}|{:>12.3e}|{:>+9.1}%|{:>10}", name, v, r, r * 100.0, v / 7.0);
    }
    let mean = samples.iter().map(|s| s.1).sum::<f64>() / samples.len() as f64;
    let var = samples.iter().map(|s| (s.1 - mean).powi(2)).sum::<f64>() / samples.len() as f64;
    println!("mean={:.4} sd={:.4e} sqrt2={} cbrt27={}", mean, var.sqrt(), 2f64.sqrt(), 27f64.cbrt());
    println!("{} {} {}", 1.0e300 * 1.0e10, -1.0e300 * 1.0e10, (0.0f64 / 0.0).is_nan());
}
