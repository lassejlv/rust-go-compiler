fn main() {
    let a = 3.5;
    let b = 2.0;
    println!("a + b = {}", a + b);
    println!("a - b = {}", a - b);
    println!("a * b = {}", a * b);
    println!("a / b = {}", a / b);
    println!("a % b = {}", a % b);
    let c: f64 = 10.0 / 3.0;
    println!("10/3 = {}", c);
    println!("10/3 (2dp) = {:.2}", c);
    println!("sqrt(2) = {}", 2.0f64.sqrt());
    println!("powi = {}", 1.5f64.powi(3));
    println!("powf = {:.10}", 2.0f64.powf(0.5));
    println!("floor/ceil/round = {} {} {}", 2.7f64.floor(), 2.2f64.ceil(), 2.5f64.round());
    println!("trunc = {}", (-2.7f64).trunc());
    println!("abs = {}", (-4.25f64).abs());
    println!("min/max = {} {}", 1.5f64.min(0.5), 1.5f64.max(0.5));
    let pi = std::f64::consts::PI;
    println!("pi = {}", pi);
    println!("sin(pi/2) = {:.10}", (pi / 2.0).sin());
    println!("cos(0) = {}", 0.0f64.cos());
    println!("exp(1) = {:.10}", 1.0f64.exp());
    println!("ln(e) = {:.10}", std::f64::consts::E.ln());
    println!("1e3 = {}", 1e3);
    println!("whole = {}", 5.0);
    println!("negative = {}", -0.125);
    let mut acc = 0.0;
    for i in 1..=10 {
        acc += 1.0 / (i as f64);
    }
    println!("harmonic(10) = {}", acc);
    println!("compare: {} {}", 0.1 + 0.2 == 0.3, (0.1f64 + 0.2 - 0.3).abs() < 1e-10);
    let x: f32 = 1.1;
    println!("f32 = {}", x * 3.0);
}
