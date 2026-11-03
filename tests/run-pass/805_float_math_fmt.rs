fn newton_sqrt(x: f64) -> f64 {
    let mut g = x / 2.0;
    for _ in 0..30 { g = 0.5 * (g + x / g); }
    g
}

fn simpson<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> f64 {
    let h = (b - a) / n as f64;
    let mut s = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        s += if i % 2 == 1 { 4.0 * f(x) } else { 2.0 * f(x) };
    }
    s * h / 3.0
}

fn main() {
    let vals = [0.0f64, -0.0, 1.0, -1.5, 0.1, 1.0 / 3.0, 2.5, 3.5, -2.5, 1e15, 1e16, 1e17, 123456789.123, 1e-5, 1.5e-10, f64::MAX, f64::MIN_POSITIVE, f64::EPSILON];
    for v in vals {
        println!("{} | {:?} | {:.2} | {:e} | {:10.3e}", v, v, v, v, v);
    }
    let specials = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    for s in specials {
        println!("{} {:?} {:>6} {:+} {:.3} {:e}", s, s, s, s, s, s);
    }
    let fs = [0.1f32, 1.0 / 3.0, 16777217.0, 3.4e38, 1e-40, 7.0];
    for v in fs {
        println!("f32: {} {:?} {:.10}", v, v, v);
    }
    // rounding behaviours
    for v in [0.5f64, 1.5, 2.5, -0.5, -1.5, 2.675, 0.125, 0.375] {
        println!("{:5}: round={} floor={} ceil={} trunc={} fract={:.3} {:.0} {:.2} re={}", v, v.round(), v.floor(), v.ceil(), v.trunc(), v.fract(), v, v, v.round_ties_even());
    }
    println!("{:.1} {:.1} {:.1} {:.1}", 0.05f64, 0.15f64, 0.25f64, 0.35f64);
    // math functions
    let x = 0.7f64;
    println!("sin={:.6} cos={:.6} tan={:.6} asin={:.6} atan2={:.6}", x.sin(), x.cos(), x.tan(), x.asin(), (1.0f64).atan2(-1.0));
    println!("exp={:.6} ln={:.6} log10={:.6} log2={:.6} log(3)={:.6}", x.exp(), x.ln(), 1000f64.log10(), 1024f64.log2(), 81f64.log(3.0));
    println!("sqrt={} cbrt={} hypot={} powf={:.6} powi={}", 2f64.sqrt(), 27f64.cbrt(), 3f64.hypot(4.0), 2f64.powf(0.5), 1.1f64.powi(10));
    println!("newton={} diff={:e}", newton_sqrt(2.0), newton_sqrt(2.0) - 2f64.sqrt());
    println!("sinh={:.5} cosh={:.5} tanh={:.5} exp_m1={:e} ln_1p={:e}", 1f64.sinh(), 1f64.cosh(), 1f64.tanh(), 1e-10f64.exp_m1(), 1e-10f64.ln_1p());
    println!("min={} max={} clamp={} copysign={} abs={} signum={} {}", 1f64.min(f64::NAN), f64::NAN.max(2.0), 7.5f64.clamp(0.0, 5.0), 3f64.copysign(-0.0), (-4.25f64).abs(), (-0.0f64).signum(), f64::NAN.signum().is_nan());
    println!("mul_add={} rem={} rem_euclid={} div_euclid={} {}", 2f64.mul_add(3.0, 1.0), -7.5f64 % 2.0, (-7.5f64).rem_euclid(2.0), (-7.5f64).div_euclid(2.0), 7.5f64 % -2.0);
    println!("to_degrees={} to_radians={:.6} recip={}", std::f64::consts::PI.to_degrees(), 180f64.to_radians(), 8f64.recip());
    println!("is_nan={} is_finite={} is_infinite={} is_sign_negative={} is_normal={} subnormal={}", (0.0f64 / 0.0).is_nan(), (1.0f64 / 0.0).is_finite(), (-1.0f64 / 0.0).is_infinite(), (-0.0f64).is_sign_negative(), 1e-310f64.is_normal(), 1e-310f64.is_subnormal());
    println!("bits {:#018x} {:#010x} from={} {}", 1.0f64.to_bits(), (-2.0f32).to_bits(), f64::from_bits(0x4009_21fb_5444_2d18), f32::from_bits(0x3f80_0001));
    println!("total_cmp {:?} {:?} partial {:?}", (-0.0f64).total_cmp(&0.0), f64::NAN.total_cmp(&f64::INFINITY), f64::NAN.partial_cmp(&1.0));
    let integral = simpson(|t| t.sin(), 0.0, std::f64::consts::PI, 100);
    println!("simpson sin 0..pi = {:.10}", integral);
    let gauss = simpson(|t| (-t * t).exp(), -5.0, 5.0, 200);
    println!("gauss = {:.8} sqrt(pi) = {:.8}", gauss, std::f64::consts::PI.sqrt());
    let mut v = vec![3.2, -1.0, f64::INFINITY, 0.0, -0.0, 2.2, -7.7];
    v.sort_by(|a, b| a.total_cmp(b));
    println!("{:?}", v);
    let mean = v.iter().filter(|x| x.is_finite()).sum::<f64>() / 6.0;
    println!("mean={:.4}", mean);
    let parsed: Vec<Result<f64, _>> = ["3.5", "-1e3", "inf", "-infinity", "NaN", "1_0", ".5", "5.", "+2", "0x10", ""].iter().map(|s| s.parse::<f64>()).collect();
    for p in &parsed {
        match p { Ok(v) => print!("{} ", v), Err(e) => print!("<{}> ", e) }
    }
    println!();
    println!("{} {} {}", 0.1 + 0.2, 0.1f32 + 0.2f32, 1.0 - 0.9);
    println!("{} {} {}", 100000000000000000000.0f64, 1.0e-7f64, 123e300f64 * 10.0);
    println!("{} {}", f32::MAX, f32::MIN_POSITIVE);
    println!("{:?} {:?}", 1.0e7f32, 1.0e16f64);
}
