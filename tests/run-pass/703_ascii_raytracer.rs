// A small ray tracer that renders spheres and a plane as ASCII art.
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
struct V3 { x: f64, y: f64, z: f64 }

impl V3 {
    const ZERO: V3 = V3 { x: 0.0, y: 0.0, z: 0.0 };
    fn new(x: f64, y: f64, z: f64) -> Self { V3 { x, y, z } }
    fn dot(self, o: V3) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn len(self) -> f64 { self.dot(self).sqrt() }
    fn norm(self) -> V3 { self * (1.0 / self.len()) }
    fn cross(self, o: V3) -> V3 {
        V3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
}
impl Add for V3 { type Output = V3; fn add(self, o: V3) -> V3 { V3::new(self.x + o.x, self.y + o.y, self.z + o.z) } }
impl Sub for V3 { type Output = V3; fn sub(self, o: V3) -> V3 { V3::new(self.x - o.x, self.y - o.y, self.z - o.z) } }
impl Mul<f64> for V3 { type Output = V3; fn mul(self, k: f64) -> V3 { V3::new(self.x * k, self.y * k, self.z * k) } }
impl Neg for V3 { type Output = V3; fn neg(self) -> V3 { self * -1.0 } }

struct Ray { o: V3, d: V3 }

struct Hit { t: f64, p: V3, n: V3, refl: f64, id: usize }

trait Shape {
    fn intersect(&self, r: &Ray) -> Option<(f64, V3)>;
    fn reflectivity(&self) -> f64 { 0.0 }
    fn name(&self) -> String;
}

struct Sphere { c: V3, r: f64, refl: f64 }
struct Plane { p: V3, n: V3 }

impl Shape for Sphere {
    fn intersect(&self, r: &Ray) -> Option<(f64, V3)> {
        let oc = r.o - self.c;
        let b = oc.dot(r.d);
        let c = oc.dot(oc) - self.r * self.r;
        let disc = b * b - c;
        if disc < 0.0 { return None; }
        let sq = disc.sqrt();
        let t = if -b - sq > 1e-6 { -b - sq } else if -b + sq > 1e-6 { -b + sq } else { return None };
        let p = r.o + r.d * t;
        Some((t, (p - self.c).norm()))
    }
    fn reflectivity(&self) -> f64 { self.refl }
    fn name(&self) -> String { format!("sphere(r={})", self.r) }
}

impl Shape for Plane {
    fn intersect(&self, r: &Ray) -> Option<(f64, V3)> {
        let denom = self.n.dot(r.d);
        if denom.abs() < 1e-9 { return None; }
        let t = (self.p - r.o).dot(self.n) / denom;
        if t > 1e-6 { Some((t, self.n)) } else { None }
    }
    fn name(&self) -> String { "plane".to_string() }
}

struct Scene { shapes: Vec<Box<dyn Shape>>, light: V3 }

impl Scene {
    fn hit(&self, r: &Ray) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        for (id, s) in self.shapes.iter().enumerate() {
            if let Some((t, n)) = s.intersect(r) {
                if best.as_ref().map_or(true, |b| t < b.t) {
                    best = Some(Hit { t, p: r.o + r.d * t, n, refl: s.reflectivity(), id });
                }
            }
        }
        best
    }
    fn shade(&self, r: &Ray, depth: u32) -> f64 {
        let h = match self.hit(r) { Some(h) => h, None => return 0.05 };
        let to_l = (self.light - h.p).norm();
        let shadow = Ray { o: h.p + h.n * 1e-4, d: to_l };
        let lit = match self.hit(&shadow) {
            Some(sh) if sh.t < (self.light - h.p).len() => 0.0,
            _ => h.n.dot(to_l).max(0.0),
        };
        let checker = if h.id == 0 {
            let k = (h.p.x.floor() as i64 + h.p.z.floor() as i64).rem_euclid(2);
            if k == 0 { 1.0 } else { 0.5 }
        } else { 1.0 };
        let base = 0.1 + 0.9 * lit * checker;
        if h.refl > 0.0 && depth < 3 {
            let d = r.d - h.n * (2.0 * r.d.dot(h.n));
            let rr = Ray { o: h.p + h.n * 1e-4, d: d.norm() };
            base * (1.0 - h.refl) + self.shade(&rr, depth + 1) * h.refl
        } else {
            base
        }
    }
}

fn main() {
    let scene = Scene {
        shapes: vec![
            Box::new(Plane { p: V3::new(0.0, -1.0, 0.0), n: V3::new(0.0, 1.0, 0.0) }),
            Box::new(Sphere { c: V3::new(0.0, 0.0, 5.0), r: 1.0, refl: 0.3 }),
            Box::new(Sphere { c: V3::new(-2.2, -0.3, 6.0), r: 0.7, refl: 0.0 }),
            Box::new(Sphere { c: V3::new(2.0, 0.5, 7.0), r: 1.2, refl: 0.6 }),
        ],
        light: V3::new(-3.0, 5.0, 0.0),
    };
    for s in &scene.shapes {
        print!("{} ", s.name());
    }
    println!();
    let eye = V3::new(0.0, 0.5, -1.0);
    let target = V3::new(0.0, 0.0, 5.0);
    let fwd = (target - eye).norm();
    let right = V3::new(0.0, 1.0, 0.0).cross(fwd).norm();
    let up = fwd.cross(right);
    let ramp: Vec<char> = " .:-=+*#%@".chars().collect();
    let (w, h) = (60, 24);
    let mut hist = [0usize; 10];
    let mut total = 0.0;
    for j in 0..h {
        let mut line = String::new();
        for i in 0..w {
            let u = (i as f64 + 0.5) / w as f64 * 2.0 - 1.0;
            let v = 1.0 - (j as f64 + 0.5) / h as f64 * 2.0;
            let d = (fwd + right * (u * 1.2) + up * (v * 0.6)).norm();
            let b = scene.shade(&Ray { o: eye, d }, 0).clamp(0.0, 0.999);
            total += b;
            let idx = (b * ramp.len() as f64) as usize;
            hist[idx] += 1;
            line.push(ramp[idx]);
        }
        println!("|{}|", line);
    }
    println!("histogram: {:?}", hist);
    println!("mean brightness: {:.4}", total / (w * h) as f64);
    println!("zero={:?} neg={:?}", V3::ZERO, -V3::new(1.0, -2.0, 0.5));
}
