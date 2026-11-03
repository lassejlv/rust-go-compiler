// float formatting: precision, NaN/inf with width and sign, -0.0, Debug vs Display, f32 vs f64 rounding
fn main() {
    let vals = [0.0f64, -0.0, 1.0, -1.5, 0.1, 1.0 / 3.0, 2.0 / 3.0, 100.0, 1e-10, 1.5e300, 123456.789];
    for v in vals {
        println!("{} | {:?} | {:.3} | {:10.2} | {:<10.1}| {:+.4} | {:e}", v, v, v, v, v, v, v);
    }
    let specials = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::NAN];
    for v in specials {
        println!("[{}] [{:?}] [{:8}] [{:<8}] [{:+}] [{:.2}] [{:08.1}]", v, v, v, v, v, v, v);
    }
    println!("[{:^9}] [{:*>9}]", f64::INFINITY, f32::NAN);
    println!("{} {} {}", -0.0f64, -0.0f64 == 0.0, (-0.0f64).is_sign_negative());
    println!("{:.0} {:.1} {:+}", -0.0f64, -0.04f64, -0.0f32);
    for v in [0.125f64, 0.375, 2.5, 3.5, -2.5, 1.005, 1.115, 0.045] {
        println!("{} -> {:.2} {:.1} {:.0}", v, v, v, v);
    }
    let f = 0.1f32;
    println!("{} {:?} {} {:.10}", f, f, f as f64, f);
    println!("{} {}", 1.1f32 * 1.1f32, 1.1f64 * 1.1f64);
    println!("{} {}", 16777216.0f32 + 1.0, 9007199254740992.0f64 + 1.0);
    println!("{:?} {:?}", f32::MAX, f32::MIN_POSITIVE);
    println!("{:?} {:?}", f64::EPSILON, f32::EPSILON);
    println!("{} {}", f64::MAX, f64::MIN);
    println!("{:.3e} {:.3e}", f64::MAX, f64::MIN_POSITIVE);
    println!("{}", 5e-324f64);
    println!("{:?}", 5e-324f64);
    println!("{}", 1e23f64);
    println!("{:?} {:?} {:?}", 1e23f64, 1e22f64, 9.999999999999999e22f64);
    println!("{} {}", 0.3f32, 0.3f32 as f64);
    println!("{} {}", 1.0e7f32, 1.0e16f32);
    println!("{:?} {:?}", 1.0e7f32, 1.0e16f32);
    println!("{} {}", 123456789.0f32, 0.000001f32);
    println!("{} {}", f64::NAN == f64::NAN, f64::NAN.is_nan());
    println!("{:?}", f64::NAN.partial_cmp(&1.0));
    println!("{} {}", (0.0f64 / 0.0).is_nan(), (1.0f64 / 0.0));
    println!("{} {} {}", (-1.0f64).sqrt().is_nan(), (-8.0f64).cbrt(), 0.0f64.ln());
    println!("{} {}", f64::INFINITY - f64::INFINITY, f64::INFINITY * 0.0);
    println!("{} {}", 1e308f64 * 10.0, -1e308f64 * 10.0);
    println!("{}", (2.0f64).powi(-1074));
    println!("{}", (2.0f64).powi(-1075));
    println!("{} {}", 1.0f64.to_bits(), (-2.0f32).to_bits());
    println!("{} {}", f64::from_bits(0x7ff0000000000000), f32::from_bits(0x3f800001));
    println!("{:?}", f32::from_bits(0x3f800001));
    println!("{} {} {} {}", 3.9f64 as i32, -3.9f64 as i32, 300.0f64 as u8, -5.0f64 as u8);
    println!("{} {} {}", f64::NAN as i32, f64::INFINITY as i64, f64::NEG_INFINITY as i16);
    println!("{} {}", i64::MAX as f64, u32::MAX as f32);
    println!("{} {}", 0.1 + 0.2 == 0.3, ((0.1f64 + 0.2) - 0.3).abs() < f64::EPSILON);
    println!("{}", 100.0f64.to_string() + &2.5f32.to_string());
    println!("{:?}", [1.0f64, 2.5, -0.0].map(|x| x.to_string()));
    println!("{:?}", "1e400".parse::<f64>());
    println!("{:?}", "-1e-400".parse::<f64>());
    println!("{:?}", "0.1".parse::<f32>().map(|x| x as f64));
    println!("{:5.1}|{:<7.3}|{:>+9.2e}|", 9.96f64, 2.0f32, 12345.0f64);
    println!("{:.20}", 0.1f64);
    println!("{:.30}", 1e-20f64);
    println!("{:.3}", 1e21f64);
    println!("{}", 1.0e-7f64);
    println!("{:?}", 1.0e-7f64);
    println!("{} {:?}", 4.0f64.sqrt(), 2.0f64.sqrt());
    println!("{}", std::f64::consts::PI);
    println!("{:.15} {}", std::f64::consts::E, std::f32::consts::PI);
    println!("{} {}", (1.0f64).atan2(1.0), (0.5f64).asin());
    println!("{} {}", 1e-5f32, 1.0e-45f32);
    println!("{} {}", 255.0f32 / 3.0, 255.0f64 / 3.0);
    for v in [0.1f32, 0.7, 1.0 / 3.0, 3.4028235e38, 1.17549435e-38, 1e-45, 7.006492e-45] {
        println!("{} {:?} {:e}", v, v, v);
    }
    for v in [123.456f64, 0.000123456, 98765432.1, 5e-310] {
        println!("{:10.4} {:<12.2e} {:+.6}", v, v, v);
    }
    println!("{:?}", [0.5f64, 1.5, 2.5, 3.5, 4.5].map(|x| format!("{:.0}", x)));
    println!("{:?}", [0.05f64, 0.15, 0.25, 0.35].map(|x| format!("{:.1}", x)));
}
