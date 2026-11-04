fn main() {
    let fs = [
        0.0f64, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, 255.9, 256.0, -1.0, -129.0, 127.99,
        1e10, -1e10, 1e20, -1e20, 1e40, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        4294967295.5, 9.2233720368547758e18, -9.2233720368547758e18, 1.8446744073709552e19,
    ];
    for &f in fs.iter() {
        println!(
            "{:?}: u8={} i8={} u16={} i16={} u32={} i32={} u64={} i64={} u128={} i128={} usize={} f32={:?}",
            f, f as u8, f as i8, f as u16, f as i16, f as u32, f as i32, f as u64, f as i64, f as u128, f as i128, f as usize, f as f32
        );
    }
    let f32s = [3.4e38f32, f32::MAX, f32::MIN, f32::NAN, 16777217.0, -2147483648.0, 2147483647.0, 1e-45];
    for &f in f32s.iter() {
        println!("{:?}: i32={} u32={} i64={} u128={} f64={:?}", f, f as i32, f as u32, f as i64, f as u128, f as f64);
    }
    // int -> float rounding
    println!("{:?} {:?} {:?}", u64::MAX as f64, i64::MIN as f64, u128::MAX as f32);
    println!("{:?} {:?}", 16777217i32 as f32, 9007199254740993i64 as f64);
    println!("{:?}", u128::MAX as f64);
    println!("{:?}", (u64::MAX as f32) as u64);
    // int -> int
    println!("{} {} {} {}", -1i32 as u8, 300i32 as u8, -1i64 as u128, u128::MAX as i8);
    println!("{} {} {}", 0x1234_5678u32 as i16, -2i8 as u64, 200u8 as i8 as i32);
    println!("{} {}", 'é' as u8, 'é' as u32);
    println!("{:?} {:?}", 65u8 as char, char::from_u32(0x1F600).map(|c| c.len_utf8()));
    println!("{} {}", true as i32 + true as i32, false as u8);
    // float methods
    let x = -3.7f64;
    println!("{} {} {} {} {} {}", x.floor(), x.ceil(), x.round(), x.trunc(), x.fract(), x.abs());
    println!("{} {} {}", 2.5f64.round(), (-2.5f64).round(), 0.49999999999999994f64.round());
    println!("{} {}", 2.5f64.round_ties_even(), 3.5f64.round_ties_even());
    println!("{:?} {:?} {:?}", f64::NAN.max(1.0), 1.0f64.min(f64::NAN), f64::NAN.clamp(0.0, 1.0).is_nan());
    println!("{} {} {}", (-0.0f64).signum(), 0.0f64.signum(), f64::NAN.signum().is_nan());
    println!("{} {}", (-0.0f64).is_sign_negative(), 0.0f64 == -0.0);
    println!("{:?} {:?}", f64::NAN == f64::NAN, f64::NAN.partial_cmp(&1.0));
    println!("{:?}", (-0.0f64).total_cmp(&0.0));
    println!("{} {}", 1e300f64 * 1e10, -1e300f64 * 1e10);
    println!("{} {} {}", 7.0f64.rem_euclid(-3.0), (-7.0f64).rem_euclid(3.0), (-7.0f64) % 3.0);
    println!("{} {}", 2.0f64.powf(0.5), 10f64.log10());
    println!("{:.3} {:.3} {:.3}", 1.0f64.exp(), std::f64::consts::PI.sin(), 0.5f64.atan2(-0.5));
    println!("{} {}", f64::EPSILON, f32::EPSILON);
    println!("{} {} {}", f64::MIN_POSITIVE, f32::MAX, f64::MAX);
    println!("{:?} {:?}", 1.0f64.to_bits(), f32::from_bits(0x4049_0fdb));
    println!("{} {}", 0.1f32 + 0.2f32, 0.1f64 + 0.2f64);
    println!("{:?} {:?} {:?}", 1e15f64, 1e16f64, 1.5e-7f64);
    println!("{} {} {}", 1e15f64, 1e16f64, 1.5e-7f64);
    println!("{:?} {:?}", f32::MAX as f64, 1.0e-40f32);
    println!("{:?} {:?}", "inf".parse::<f64>(), "-NaN".parse::<f64>().map(|x| x.is_nan()));
    println!("{:?} {:?}", "1e400".parse::<f64>(), "  1.5".parse::<f64>().is_err());
    println!("{:?} {:?}", ".5".parse::<f64>(), "5.".parse::<f32>());
    println!("{} {}", f64::NAN as i32, (f64::NAN as u8) + 1);
    println!("{}", (0.1f64 * 3.0).to_string());
    println!("{} {}", 100000000.0f32, 1.0e7f32);
    println!("{} {}", 123456789.0f32, 0.3f32 as f64);
    println!("{:?} {:?}", i32::MAX as f32, i32::MAX as f32 as i32);
    println!("{} {}", u8::MAX as f32 / 2.0, (-1i32) as f32 as u32);
    println!("{}", 1.0f64.hypot(1.0).mul_add(2.0, 1.0));
}
