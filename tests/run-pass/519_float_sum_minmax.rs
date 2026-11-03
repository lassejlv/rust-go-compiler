// sums/products of floats, min_by/max_by with partial_cmp, total_cmp sorting, statistics
fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}
fn variance(v: &[f64]) -> f64 {
    let m = mean(v);
    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 0 { (s[n / 2 - 1] + s[n / 2]) / 2.0 } else { s[n / 2] }
}

#[derive(Debug, Clone)]
struct City {
    name: &'static str,
    lat: f64,
    temp: f32,
}

fn main() {
    let v = [2.5, 3.25, -1.0, 8.0, 4.75, 0.1, 0.2];
    println!("{}", v.iter().sum::<f64>());
    println!("{}", v.iter().product::<f64>());
    println!("{:.4} {:.4} {}", mean(&v), variance(&v), median(&v));
    let f32s = [0.1f32, 0.2, 0.3];
    println!("{} {}", f32s.iter().sum::<f32>(), f32s.iter().map(|&x| x as f64).sum::<f64>());
    println!("{}", 0.1f64 + 0.2);
    println!("{}", (0..10).map(|_| 0.1f64).sum::<f64>());
    let mx = v.iter().cloned().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let mn = v.iter().copied().min_by(|a, b| a.total_cmp(b)).unwrap();
    println!("{} {}", mx, mn);
    let cities = vec![
        City { name: "Oslo", lat: 59.91, temp: -3.5 },
        City { name: "Cairo", lat: 30.04, temp: 24.25 },
        City { name: "Quito", lat: -0.18, temp: 14.0 },
        City { name: "Perth", lat: -31.95, temp: 24.25 },
    ];
    let hottest = cities.iter().max_by(|a, b| a.temp.partial_cmp(&b.temp).unwrap()).unwrap();
    let coldest = cities.iter().min_by(|a, b| a.temp.total_cmp(&b.temp)).unwrap();
    println!("hot={} cold={}", hottest.name, coldest.name);
    let southmost = cities.iter().min_by(|a, b| a.lat.partial_cmp(&b.lat).unwrap()).map(|c| c.name);
    println!("{:?}", southmost);
    let mut sorted = cities.clone();
    sorted.sort_by(|a, b| b.temp.partial_cmp(&a.temp).unwrap().then_with(|| a.name.cmp(b.name)));
    println!("{:?}", sorted.iter().map(|c| c.name).collect::<Vec<_>>());
    let mut weird = vec![3.0, f64::NAN, -0.0, 0.0, f64::INFINITY, -7.5, f64::NEG_INFINITY, 1e-300];
    weird.sort_by(f64::total_cmp);
    println!("{:?}", weird);
    let maxnan = [1.0f64, f64::NAN, 2.0].iter().cloned().fold(f64::MIN, f64::max);
    let minnan = [f64::NAN, 5.0].iter().cloned().fold(f64::NAN, f64::min);
    println!("{} {}", maxnan, minnan);
    println!("{} {}", f64::NAN.max(1.0), 1.0f64.min(f64::NAN));
    println!("{:?}", [1.5f64, 2.5, 3.5].iter().map(|x| x.round()).collect::<Vec<_>>());
    println!("{:?}", [-1.5f64, -2.5, 2.5].iter().map(|x| x.round_ties_even()).collect::<Vec<_>>());
    println!("{:?}", [-1.7f64, 1.7].map(|x| (x.trunc(), x.fract(), x.floor(), x.ceil())));
    println!("{} {}", 7.5f64.clamp(0.0, 5.0), (-2.0f64).clamp(-1.0, 1.0));
    println!("{} {} {}", 2f64.powf(0.5), 10f64.ln(), 1000f64.log10());
    println!("{} {}", 1e16f64 + 1.0 - 1e16, (1e16f64 - 1e16) + 1.0);
    let kahan = {
        let mut sum = 0.0f64;
        let mut c = 0.0f64;
        for _ in 0..1000 {
            let y = 0.001 - c;
            let t = sum + y;
            c = (t - sum) - y;
            sum = t;
        }
        sum
    };
    let naive: f64 = (0..1000).map(|_| 0.001).sum();
    println!("{} {}", kahan, naive);
    let pi_approx: f64 = (0..10000).map(|k| if k % 2 == 0 { 4.0 } else { -4.0 } / (2 * k + 1) as f64).sum();
    println!("{:.6}", pi_approx);
    println!("{}", f32::EPSILON);
    println!("{} {}", (0.1f32 + 0.2f32) as f64, 16777217.0f32);
    println!("{} {}", f64::MAX, f64::MIN_POSITIVE);
    println!("{:?} {:?}", 1.0f64, 0.30000000000000004f64);
    println!("{} {}", (2.0f64).sqrt().mul_add(2.0, 1.0), 9f64.hypot(12.0));
}
