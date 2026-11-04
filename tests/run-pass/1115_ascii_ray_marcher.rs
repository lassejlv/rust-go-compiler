// An ASCII ray marcher: signed distance fields with smooth unions,
// soft shadows, ambient occlusion, a trait-object scene graph and
// a couple of camera angles, rendered to characters.
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
struct V { x: f64, y: f64, z: f64 }

const fn v(x: f64, y: f64, z: f64) -> V { V { x, y, z } }

impl Add for V { type Output = V; fn add(self, o: V) -> V { v(self.x + o.x, self.y + o.y, self.z + o.z) } }
impl Sub for V { type Output = V; fn sub(self, o: V) -> V { v(self.x - o.x, self.y - o.y, self.z - o.z) } }
impl Mul<f64> for V { type Output = V; fn mul(self, k: f64) -> V { v(self.x * k, self.y * k, self.z * k) } }
impl Neg for V { type Output = V; fn neg(self) -> V { v(-self.x, -self.y, -self.z) } }

impl V {
    fn dot(self, o: V) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn len(self) -> f64 { self.dot(self).sqrt() }
    fn norm(self) -> V { self * (1.0 / self.len()) }
    fn cross(self, o: V) -> V { v(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x) }
    fn abs(self) -> V { v(self.x.abs(), self.y.abs(), self.z.abs()) }
    fn max(self, k: f64) -> V { v(self.x.max(k), self.y.max(k), self.z.max(k)) }
    fn rot_y(self, a: f64) -> V { let (s, c) = a.sin_cos(); v(c * self.x + s * self.z, self.y, -s * self.x + c * self.z) }
}

trait Sdf {
    fn dist(&self, p: V) -> f64;
    fn name(&self) -> String;
}

struct Sphere { c: V, r: f64 }
struct BoxS { c: V, half: V, round: f64 }
struct Plane { y: f64 }
struct Torus { c: V, major: f64, minor: f64 }
struct SmoothUnion { a: Box<dyn Sdf>, b: Box<dyn Sdf>, k: f64 }
struct Subtract { a: Box<dyn Sdf>, b: Box<dyn Sdf> }
struct Rotated { inner: Box<dyn Sdf>, angle: f64, pivot: V }
struct Union(Vec<Box<dyn Sdf>>);

impl Sdf for Sphere {
    fn dist(&self, p: V) -> f64 { (p - self.c).len() - self.r }
    fn name(&self) -> String { format!("sphere(r={})", self.r) }
}
impl Sdf for BoxS {
    fn dist(&self, p: V) -> f64 {
        let q = (p - self.c).abs() - self.half;
        q.max(0.0).len() + q.x.max(q.y.max(q.z)).min(0.0) - self.round
    }
    fn name(&self) -> String { format!("box({:.1}x{:.1}x{:.1})", self.half.x * 2.0, self.half.y * 2.0, self.half.z * 2.0) }
}
impl Sdf for Plane {
    fn dist(&self, p: V) -> f64 { p.y - self.y }
    fn name(&self) -> String { "plane".into() }
}
impl Sdf for Torus {
    fn dist(&self, p: V) -> f64 {
        let q = p - self.c;
        let xz = (q.x * q.x + q.z * q.z).sqrt() - self.major;
        (xz * xz + q.y * q.y).sqrt() - self.minor
    }
    fn name(&self) -> String { format!("torus({}, {})", self.major, self.minor) }
}
impl Sdf for SmoothUnion {
    fn dist(&self, p: V) -> f64 {
        let (d1, d2) = (self.a.dist(p), self.b.dist(p));
        let h = (0.5 + 0.5 * (d2 - d1) / self.k).clamp(0.0, 1.0);
        d2 * (1.0 - h) + d1 * h - self.k * h * (1.0 - h)
    }
    fn name(&self) -> String { format!("smooth({}, {})", self.a.name(), self.b.name()) }
}
impl Sdf for Subtract {
    fn dist(&self, p: V) -> f64 { self.a.dist(p).max(-self.b.dist(p)) }
    fn name(&self) -> String { format!("({} - {})", self.a.name(), self.b.name()) }
}
impl Sdf for Rotated {
    fn dist(&self, p: V) -> f64 { self.inner.dist((p - self.pivot).rot_y(-self.angle) + self.pivot) }
    fn name(&self) -> String { format!("rot{:.0}({})", self.angle.to_degrees(), self.inner.name()) }
}
impl Sdf for Union {
    fn dist(&self, p: V) -> f64 { self.0.iter().map(|s| s.dist(p)).fold(f64::INFINITY, f64::min) }
    fn name(&self) -> String { format!("union[{}]", self.0.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")) }
}

struct Stats { rays: u64, steps: u64, hits: u64, max_steps: u32 }

fn march(scene: &dyn Sdf, ro: V, rd: V, stats: &mut Stats) -> Option<(f64, u32)> {
    let mut t = 0.0;
    stats.rays += 1;
    for i in 0..120u32 {
        let d = scene.dist(ro + rd * t);
        stats.steps += 1;
        if d < 1e-3 {
            stats.hits += 1;
            stats.max_steps = stats.max_steps.max(i);
            return Some((t, i));
        }
        t += d;
        if t > 40.0 { break; }
    }
    None
}

fn normal(scene: &dyn Sdf, p: V) -> V {
    let e = 1e-4;
    v(
        scene.dist(p + v(e, 0.0, 0.0)) - scene.dist(p - v(e, 0.0, 0.0)),
        scene.dist(p + v(0.0, e, 0.0)) - scene.dist(p - v(0.0, e, 0.0)),
        scene.dist(p + v(0.0, 0.0, e)) - scene.dist(p - v(0.0, 0.0, e)),
    ).norm()
}

fn soft_shadow(scene: &dyn Sdf, p: V, l: V, k: f64) -> f64 {
    let mut res: f64 = 1.0;
    let mut t = 0.02;
    for _ in 0..48 {
        let h = scene.dist(p + l * t);
        if h < 1e-4 { return 0.0; }
        res = res.min(k * h / t);
        t += h.clamp(0.01, 0.5);
        if t > 20.0 { break; }
    }
    res.clamp(0.0, 1.0)
}

fn ao(scene: &dyn Sdf, p: V, n: V) -> f64 {
    let mut occ = 0.0;
    let mut w = 1.0;
    for i in 1..=5 {
        let h = 0.04 * i as f64;
        occ += (h - scene.dist(p + n * h)) * w;
        w *= 0.7;
    }
    (1.0 - 2.5 * occ).clamp(0.0, 1.0)
}

const RAMP: &[u8] = b" .:-=+*#%@";

fn render(scene: &dyn Sdf, eye: V, target: V, w: usize, h: usize) -> (Vec<String>, Stats) {
    let fwd = (target - eye).norm();
    let right = fwd.cross(v(0.0, 1.0, 0.0)).norm();
    let up = right.cross(fwd);
    let light = v(-0.6, 0.8, -0.4).norm();
    let mut stats = Stats { rays: 0, steps: 0, hits: 0, max_steps: 0 };
    let mut rows = Vec::with_capacity(h);
    for j in 0..h {
        let mut row = String::with_capacity(w);
        for i in 0..w {
            let u = (i as f64 + 0.5) / w as f64 * 2.0 - 1.0;
            let vv = 1.0 - (j as f64 + 0.5) / h as f64 * 2.0;
            let rd = (fwd * 1.5 + right * (u * w as f64 / h as f64 * 0.5) + up * vv).norm();
            let c = match march(scene, eye, rd, &mut stats) {
                None => if vv > 0.3 { b' ' } else { b'`' },
                Some((t, _)) => {
                    let p = eye + rd * t;
                    let n = normal(scene, p);
                    let diff = n.dot(light).max(0.0);
                    let sh = soft_shadow(scene, p + n * 0.01, light, 8.0);
                    let occ = ao(scene, p, n);
                    let mut lum = (0.08 + 0.92 * diff * sh) * (0.4 + 0.6 * occ);
                    if p.y < -0.99 {
                        let checker = ((p.x * 1.5).floor() as i64 + (p.z * 1.5).floor() as i64).rem_euclid(2);
                        lum *= if checker == 0 { 0.85 } else { 0.45 };
                    }
                    let idx = ((lum.clamp(0.0, 0.9999)) * RAMP.len() as f64) as usize;
                    RAMP[idx]
                }
            };
            row.push(c as char);
        }
        rows.push(row.trim_end().to_string());
    }
    (rows, stats)
}

fn main() {
    let scene = Union(vec![
        Box::new(Plane { y: -1.0 }),
        Box::new(SmoothUnion {
            a: Box::new(Sphere { c: v(0.0, 0.0, 0.0), r: 1.0 }),
            b: Box::new(Sphere { c: v(0.9, 0.5, 0.2), r: 0.6 }),
            k: 0.4,
        }),
        Box::new(Subtract {
            a: Box::new(BoxS { c: v(-2.4, -0.3, 0.8), half: v(0.7, 0.7, 0.7), round: 0.05 }),
            b: Box::new(Sphere { c: v(-2.4, -0.3, 0.8), r: 0.9 }),
        }),
        Box::new(Rotated {
            inner: Box::new(Torus { c: v(2.4, -0.6, 1.0), major: 0.8, minor: 0.25 }),
            angle: 0.5,
            pivot: v(2.4, -0.6, 1.0),
        }),
    ]);
    println!("scene: {}", scene.name());
    for (eye, label) in [(v(0.0, 1.2, -5.5), "front"), (v(4.0, 3.0, -4.0), "high corner")] {
        let (rows, st) = render(&scene, eye, v(0.0, -0.2, 0.5), 72, 26);
        println!("--- {} ---", label);
        for r in &rows { println!("{}", r); }
        let hist: Vec<(char, usize)> = RAMP.iter().map(|&c| (c as char, rows.iter().map(|r| r.bytes().filter(|&b| b == c).count()).sum())).filter(|&(_, n)| n > 0).collect();
        println!("rays={} hits={} avg steps={:.2} max steps={}", st.rays, st.hits, st.steps as f64 / st.rays as f64, st.max_steps);
        println!("histogram: {:?}", hist);
    }
    let probe = [v(0.0, 0.0, 0.0), v(0.0, 2.0, 0.0), v(-2.4, -0.3, 0.8), v(2.4, -0.6, 1.0), v(10.0, -1.0, 10.0)];
    for p in probe {
        println!("dist({:>5.1},{:>5.1},{:>5.1}) = {:>8.4}", p.x, p.y, p.z, scene.dist(p));
    }
}
