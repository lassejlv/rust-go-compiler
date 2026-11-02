fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn std_dev(xs: &[f64]) -> f64 {
    let m = mean(xs);
    let var = xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / xs.len() as f64;
    var.sqrt()
}

fn newton_sqrt(x: f64) -> f64 {
    let mut guess = x / 2.0;
    for i in 0..20 {
        let next = 0.5 * (guess + x / guess);
        if (next - guess).abs() < 1e-12 {
            println!("  converged after {} iterations", i + 1);
            return next;
        }
        guess = next;
    }
    guess
}

fn main() {
    let data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    println!("mean = {}", mean(&data));
    println!("std_dev = {}", std_dev(&data));
    println!("newton sqrt(2) = {:.12}", newton_sqrt(2.0));
    println!("newton sqrt(1e6) = {:.6}", newton_sqrt(1e6));
    let a: f32 = 0.1;
    let b: f32 = 0.2;
    println!("f32 sum = {}", a + b);
    println!("f32 product = {}", a * b);
    println!("f32 -> f64 = {}", (a + b) as f64);
    let mut x = 1.0f64;
    for _ in 0..10 {
        x *= 1.1;
    }
    println!("1.1^10 = {}", x);
    println!("clamp: {} {} {}", 5.5f64.clamp(0.0, 1.0), (-2.0f64).clamp(0.0, 1.0), 0.5f64.clamp(0.0, 1.0));
    println!("round: {} {} {} {}", 0.5f64.round(), 1.5f64.round(), -0.5f64.round(), -1.5f64.round());
    println!("fract: {}", 3.75f64.fract());
    println!("recip: {}", 4.0f64.recip());
    println!("to_degrees: {:.4}", std::f64::consts::FRAC_PI_2.to_degrees());
    println!("hypot: {}", 3.0f64.hypot(4.0));
    println!("mul_add: {}", 2.0f64.mul_add(3.0, 1.0));
    println!("max of list: {}", data.iter().cloned().fold(f64::MIN, f64::max));
    println!("min of list: {}", data.iter().cloned().fold(f64::INFINITY, f64::min));
    println!("parse: {:?} {:?} {:?}", "3.25".parse::<f64>(), "1e3".parse::<f64>(), "-.5".parse::<f64>());
    println!("parse inf: {:?}", "inf".parse::<f64>());
    println!("parse bad: {}", "abc".parse::<f64>().is_err());
    let celsius = [-40.0, 0.0, 36.6, 100.0];
    for c in celsius {
        let f = c * 9.0 / 5.0 + 32.0;
        println!("{:>6.1}C = {:>6.1}F", c, f);
    }
    let total_cents: i64 = (19.99f64 * 100.0).round() as i64 * 3;
    println!("3 x 19.99 = {}.{:02}", total_cents / 100, total_cents % 100);
    let mut sum = 0.0f64;
    for _ in 0..10 {
        sum += 0.1;
    }
    println!("0.1 x10 = {} (== 1.0: {})", sum, sum == 1.0);
    println!("total_cmp ordering: {:?}", 1.0f64.total_cmp(&2.0));
    println!("to_bits: {:x}", 1.0f64.to_bits());
    println!("from_bits: {}", f64::from_bits(0x4000000000000000));
    println!("integer powers: {} {}", 2f64.powi(-2), 10f64.powi(15));
}
