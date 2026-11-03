fn main() {
    let vals = [0.1 + 0.2, 1e21, 1e-7, 123456789.0, -0.0, 1.0 / 3.0, 2.5e-308, f64::MIN_POSITIVE, 5e-324, 1e15, 1e16, 0.3];
    for v in vals {
        println!("{} | {:?} | {:e} | {:.3}", v, v, v, v);
    }
    let fs = [0.1f32, 16777217.0, 1.0e-10, 3.4028235e38, f32::EPSILON];
    for v in fs {
        println!("{} | {:?} | {:E}", v, v, v);
    }
    println!("{} {} {}", f64::NAN, f64::INFINITY, -f64::INFINITY);
    println!("{:?} {:?}", f64::NAN, -f32::INFINITY);
    println!("[{:8.2}] [{:<8.2}] [{:^8.2}] [{:08.2}] [{:+08.2}]", 3.14159, 3.14159, 3.14159, -3.14159, 3.14159);
    println!("{:.0} {:.0} {:.0} {:.0}", 0.5, 1.5, 2.5, -0.5);
    println!("{:.1} {:.1} {:.2}", 0.05, 0.25, 1.005);
    println!("{:10.3e}|{:<12e}|", 1234.5678, 0.00042);
    println!("{}", 1.0f64);
    println!("{:?}", 1.0f64);
    println!("{:?}", vec![1.0, 2.5, -0.0]);
    println!("{:#?}", (1.5f32, 2.0f64));
    println!("{}", 100000000000000000000.0f64);
    println!("{}", 0.000001);
    println!("{:?}", 0.000001);
    println!("{}", f64::MAX);
    println!("{:?}", f64::MIN);
    println!("{}", 1.0e-5f32);
    println!("{:.20}", 0.1);
    println!("{:.30}", 1e-10);
    println!("{}", (2.0f64).sqrt());
    println!("{}", (2.0f32).sqrt());
    println!("{} {}", 7.0f64.rem_euclid(-3.0), (-7.5f64).rem_euclid(2.0));
    println!("{} {} {} {}", 2.5f64.round(), (-2.5f64).round(), 2.5f64.round_ties_even(), 3.7f64.trunc());
    println!("{} {}", 1e300 * 1e10, -1e300 * 1e10);
    println!("{:?} {:?}", "1e3".parse::<f64>(), "inf".parse::<f64>());
    println!("{:?} {:?}", "-.5".parse::<f32>(), "1.".parse::<f64>());
    println!("{:?}", "abc".parse::<f64>().map_err(|e| e.to_string()));
    println!("{} {}", 255.9f64 as u8, -1.5f64 as u8);
    println!("{} {}", f64::NAN as i32, 1e20 as i32);
    println!("{} {}", 0.1f32 as f64, 0.1f64 as f32);
    println!("{:?} {:?}", 1.0f64.to_bits(), f32::from_bits(0x3fc00000));
    println!("{:?}", (0.1f64).total_cmp(&-0.0));
    println!("{} {}", f64::NAN.max(1.0), 2.0f64.min(f64::NAN));
    println!("{} {}", 1.0f64.copysign(-0.0), (-0.0f64).is_sign_negative());
    println!("{} {}", 8.0f64.log2(), 1000.0f64.log10());
    println!("{:.6} {:.6}", std::f64::consts::PI.sin(), std::f64::consts::E.ln());
    println!("{}", 3.0f64.hypot(4.0));
    println!("{}", 10.0f64.powi(-2));
    println!("{}", 2.0f64.powf(0.5));
    println!("{}", (0.1f64 + 0.2 - 0.3).abs() < f64::EPSILON);
    println!("{:?}", (1.5f64).fract());
    println!("{:?}", 9.99f64.clamp(0.0, 5.0));
    let w = 10;
    let p = 3;
    println!("[{:w$.p$}] [{:>1$}] [{:.*}] [{:>+w$.2e}]", 1.23456, 7, 2, 9.87654, 12345.678);
}
