#[derive(Clone, Copy, Debug)]
struct Body { pos: [f64; 3], vel: [f64; 3], mass: f64 }

const PI: f64 = std::f64::consts::PI;
const SOLAR_MASS: f64 = 4.0 * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;

fn bodies() -> Vec<Body> {
    vec![
        Body { pos: [0.0; 3], vel: [0.0; 3], mass: SOLAR_MASS },
        Body { pos: [4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01], vel: [1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR], mass: 9.54791938424326609e-04 * SOLAR_MASS },
        Body { pos: [8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01], vel: [-2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR], mass: 2.85885980666130812e-04 * SOLAR_MASS },
        Body { pos: [1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01], vel: [2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR], mass: 4.36624404335156298e-05 * SOLAR_MASS },
        Body { pos: [1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01], vel: [2.68067772490389322e-03 * DAYS_PER_YEAR, 1.62824170038242295e-03 * DAYS_PER_YEAR, -9.51592254519715870e-05 * DAYS_PER_YEAR], mass: 5.15138902046611451e-05 * SOLAR_MASS },
    ]
}

fn offset_momentum(b: &mut [Body]) {
    let mut p = [0.0; 3];
    for x in b.iter() { for k in 0..3 { p[k] += x.vel[k] * x.mass; } }
    for k in 0..3 { b[0].vel[k] = -p[k] / SOLAR_MASS; }
}

fn energy(b: &[Body]) -> f64 {
    let mut e = 0.0;
    for (i, x) in b.iter().enumerate() {
        e += 0.5 * x.mass * x.vel.iter().map(|v| v * v).sum::<f64>();
        for y in &b[i + 1..] {
            let d: f64 = (0..3).map(|k| (x.pos[k] - y.pos[k]).powi(2)).sum::<f64>().sqrt();
            e -= x.mass * y.mass / d;
        }
    }
    e
}

fn advance(b: &mut [Body], dt: f64) {
    let n = b.len();
    for i in 0..n {
        let (left, right) = b.split_at_mut(i + 1);
        let bi = &mut left[i];
        for bj in right.iter_mut() {
            let d = [bi.pos[0] - bj.pos[0], bi.pos[1] - bj.pos[1], bi.pos[2] - bj.pos[2]];
            let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            let mag = dt / (d2 * d2.sqrt());
            for k in 0..3 { bi.vel[k] -= d[k] * bj.mass * mag; bj.vel[k] += d[k] * bi.mass * mag; }
        }
    }
    for x in b.iter_mut() { for k in 0..3 { x.pos[k] += dt * x.vel[k]; } }
}

#[derive(Clone, Copy, Debug)]
struct P32 { x: f32, y: f32, vx: f32, vy: f32 }

fn bounce_sim(steps: usize) -> Vec<P32> {
    let mut ps: Vec<P32> = (0..5).map(|i| P32 { x: i as f32 * 1.5, y: 10.0 + i as f32, vx: 0.7 - i as f32 * 0.3, vy: 0.0 }).collect();
    let g = -9.81f32; let dt = 0.01f32;
    for _ in 0..steps {
        for p in ps.iter_mut() {
            p.vy += g * dt;
            p.x += p.vx * dt; p.y += p.vy * dt;
            if p.y < 0.0 { p.y = -p.y; p.vy = -p.vy * 0.8; }
            if p.x < 0.0 || p.x > 8.0 { p.vx = -p.vx; p.x = p.x.clamp(0.0, 8.0); }
        }
    }
    ps
}

fn main() {
    let mut b = bodies();
    offset_momentum(&mut b);
    println!("{:.9}", energy(&b));
    for step in 1..=1000 { advance(&mut b, 0.01); if step % 250 == 0 { println!("step {:>4}: {:.9}", step, energy(&b)); } }
    for x in &b { println!("{:>12.6} {:>12.6} {:>12.6}", x.pos[0], x.pos[1], x.pos[2]); }
    let ps = bounce_sim(500);
    for p in &ps { println!("{:?}", p); }
    let s32: f32 = (1..=1000).map(|i| 1.0f32 / i as f32).sum();
    let s64: f64 = (1..=1000).map(|i| 1.0f64 / i as f64).sum();
    let s32r: f32 = (1..=1000).rev().map(|i| 1.0f32 / i as f32).sum();
    println!("{} {} {} {}", s32, s32r, s64, s32 as f64 - s64);
    let mut kahan = 0.0f32; let mut c = 0.0f32;
    for i in 1..=1000 { let y = 1.0f32 / i as f32 - c; let t = kahan + y; c = (t - kahan) - y; kahan = t; }
    println!("kahan {}", kahan);
    let v: f32 = 16777216.0; println!("{} {} {}", v + 1.0, v + 2.0, (v as f64 + 1.0) as f32);
    println!("{} {} {}", 0.1f32 + 0.2f32 == 0.3f32, 0.1f64 + 0.2 == 0.3, (0.1f32 + 0.2f32) as f64);
    println!("{:?}", (0..8).map(|i| (i as f32 * 0.7).sin()).collect::<Vec<f32>>());
    println!("{:?}", [1.0f32, 2.0, 3.0].map(|x| x.sqrt().ln().exp()));
}
