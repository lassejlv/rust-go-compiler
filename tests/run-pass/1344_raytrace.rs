// A small deterministic ray tracer: vector math with operator overloading,
// trait-object scene graph, materials, shadows, reflections; prints ASCII art
// and a hash of the PPM bytes.
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Index, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    const ZERO: V3 = V3 { x: 0.0, y: 0.0, z: 0.0 };
    const fn new(x: f64, y: f64, z: f64) -> Self {
        V3 { x, y, z }
    }
    fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn cross(self, o: V3) -> V3 {
        V3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn norm(self) -> V3 {
        self / self.len()
    }
    fn hadamard(self, o: V3) -> V3 {
        V3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    fn reflect(self, n: V3) -> V3 {
        self - n * (2.0 * self.dot(n))
    }
    fn clamp01(self) -> V3 {
        V3::new(self.x.clamp(0.0, 1.0), self.y.clamp(0.0, 1.0), self.z.clamp(0.0, 1.0))
    }
    fn max_component(self) -> f64 {
        self.x.max(self.y).max(self.z)
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl AddAssign for V3 {
    fn add_assign(&mut self, o: V3) {
        *self = *self + o;
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f64> for V3 {
    type Output = V3;
    fn mul(self, s: f64) -> V3 {
        V3::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Mul<V3> for f64 {
    type Output = V3;
    fn mul(self, v: V3) -> V3 {
        v * self
    }
}
impl Div<f64> for V3 {
    type Output = V3;
    fn div(self, s: f64) -> V3 {
        V3::new(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        V3::new(-self.x, -self.y, -self.z)
    }
}
impl Index<usize> for V3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("V3 index {} out of range", i),
        }
    }
}
impl fmt::Display for V3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let p = f.precision().unwrap_or(3);
        write!(f, "({:.*}, {:.*}, {:.*})", p, self.x, p, self.y, p, self.z)
    }
}

#[derive(Debug, Clone, Copy)]
struct Ray {
    o: V3,
    d: V3,
}

impl Ray {
    fn at(&self, t: f64) -> V3 {
        self.o + self.d * t
    }
}

#[derive(Debug, Clone, Copy)]
enum Material {
    Diffuse { color: V3 },
    Mirror { tint: V3, fuzz_free: bool },
    Checker { a: V3, b: V3, scale: f64 },
    Emissive(V3),
}

struct Hit {
    t: f64,
    p: V3,
    n: V3,
    mat: Material,
    id: usize,
}

trait Shape: fmt::Debug {
    fn intersect(&self, r: &Ray, tmin: f64, tmax: f64) -> Option<(f64, V3)>;
    fn material(&self) -> Material;
    fn name(&self) -> String {
        format!("{:?}", self).split_whitespace().next().unwrap_or("?").to_string()
    }
}

#[derive(Debug)]
struct Sphere {
    c: V3,
    r: f64,
    m: Material,
}

#[derive(Debug)]
struct Plane {
    p: V3,
    n: V3,
    m: Material,
}

#[derive(Debug)]
struct Triangle {
    a: V3,
    b: V3,
    c: V3,
    m: Material,
}

impl Shape for Sphere {
    fn intersect(&self, r: &Ray, tmin: f64, tmax: f64) -> Option<(f64, V3)> {
        let oc = r.o - self.c;
        let a = r.d.dot(r.d);
        let hb = oc.dot(r.d);
        let c = oc.dot(oc) - self.r * self.r;
        let disc = hb * hb - a * c;
        if disc < 0.0 {
            return None;
        }
        let sq = disc.sqrt();
        let mut t = (-hb - sq) / a;
        if t < tmin || t > tmax {
            t = (-hb + sq) / a;
            if t < tmin || t > tmax {
                return None;
            }
        }
        let n = (r.at(t) - self.c) / self.r;
        Some((t, n))
    }
    fn material(&self) -> Material {
        self.m
    }
}

impl Shape for Plane {
    fn intersect(&self, r: &Ray, tmin: f64, tmax: f64) -> Option<(f64, V3)> {
        let denom = self.n.dot(r.d);
        if denom.abs() < 1e-9 {
            return None;
        }
        let t = (self.p - r.o).dot(self.n) / denom;
        (tmin..tmax).contains(&t).then(|| (t, if denom < 0.0 { self.n } else { -self.n }))
    }
    fn material(&self) -> Material {
        self.m
    }
}

impl Shape for Triangle {
    fn intersect(&self, r: &Ray, tmin: f64, tmax: f64) -> Option<(f64, V3)> {
        // Moller-Trumbore
        let e1 = self.b - self.a;
        let e2 = self.c - self.a;
        let p = r.d.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        let s = r.o - self.a;
        let u = s.dot(p) * inv;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(e1);
        let v = r.d.dot(q) * inv;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = e2.dot(q) * inv;
        if t < tmin || t > tmax {
            return None;
        }
        let mut n = e1.cross(e2).norm();
        if n.dot(r.d) > 0.0 {
            n = -n;
        }
        Some((t, n))
    }
    fn material(&self) -> Material {
        self.m
    }
}

struct Light {
    pos: V3,
    color: V3,
}

struct Scene {
    shapes: Vec<Box<dyn Shape>>,
    lights: Vec<Light>,
    ambient: V3,
    stats: Stats,
}

#[derive(Default, Debug)]
struct Stats {
    rays: u64,
    shadow_rays: u64,
    hits: u64,
    max_depth_seen: u32,
}

impl Scene {
    fn hit(&self, r: &Ray, tmin: f64, tmax: f64) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        let mut closest = tmax;
        for (id, s) in self.shapes.iter().enumerate() {
            if let Some((t, n)) = s.intersect(r, tmin, closest) {
                closest = t;
                best = Some(Hit { t, p: r.at(t), n, mat: s.material(), id });
            }
        }
        best
    }

    fn occluded(&self, from: V3, to: V3) -> bool {
        let d = to - from;
        let dist = d.len();
        let r = Ray { o: from, d: d / dist };
        self.shapes.iter().any(|s| s.intersect(&r, 1e-4, dist - 1e-4).is_some())
    }

    fn trace(&mut self, r: &Ray, depth: u32) -> V3 {
        self.stats.rays += 1;
        self.stats.max_depth_seen = self.stats.max_depth_seen.max(depth);
        if depth > 4 {
            return V3::ZERO;
        }
        let h = match self.hit(r, 1e-4, f64::INFINITY) {
            Some(h) => h,
            None => {
                let t = 0.5 * (r.d.y + 1.0);
                return V3::new(1.0, 1.0, 1.0) * (1.0 - t) + V3::new(0.5, 0.7, 1.0) * t;
            }
        };
        self.stats.hits += 1;
        let base = match h.mat {
            Material::Diffuse { color } => color,
            Material::Checker { a, b, scale } => {
                let k = ((h.p.x * scale).floor() as i64 + (h.p.z * scale).floor() as i64).rem_euclid(2);
                if k == 0 {
                    a
                } else {
                    b
                }
            }
            Material::Emissive(c) => return c,
            Material::Mirror { tint, fuzz_free } => {
                let refl = Ray { o: h.p, d: r.d.reflect(h.n).norm() };
                let c = self.trace(&refl, depth + 1);
                return if fuzz_free { c.hadamard(tint) } else { (c.hadamard(tint) + tint * 0.1).clamp01() };
            }
        };
        let mut col = base.hadamard(self.ambient);
        let lights: Vec<(V3, V3)> = self.lights.iter().map(|l| (l.pos, l.color)).collect();
        for (lp, lc) in lights {
            self.stats.shadow_rays += 1;
            if self.occluded(h.p, lp) {
                continue;
            }
            let ldir = (lp - h.p).norm();
            let diff = h.n.dot(ldir).max(0.0);
            let half = (ldir - r.d).norm();
            let spec = h.n.dot(half).max(0.0).powi(32);
            col += base.hadamard(lc) * diff + lc * (spec * 0.3);
        }
        let _ = h.id;
        let _ = h.t;
        col
    }
}

struct Camera {
    origin: V3,
    lower_left: V3,
    horiz: V3,
    vert: V3,
}

impl Camera {
    fn new(from: V3, at: V3, up: V3, vfov_deg: f64, aspect: f64) -> Self {
        let theta = vfov_deg * PI / 180.0;
        let h = (theta / 2.0).tan();
        let vh = 2.0 * h;
        let vw = aspect * vh;
        let w = (from - at).norm();
        let u = up.cross(w).norm();
        let v = w.cross(u);
        let horiz = u * vw;
        let vert = v * vh;
        Camera { origin: from, lower_left: from - horiz / 2.0 - vert / 2.0 - w, horiz, vert }
    }
    fn ray(&self, s: f64, t: f64) -> Ray {
        Ray { o: self.origin, d: (self.lower_left + self.horiz * s + self.vert * t - self.origin).norm() }
    }
}

fn build_scene() -> Scene {
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    shapes.push(Box::new(Plane { p: V3::new(0.0, -1.0, 0.0), n: V3::new(0.0, 1.0, 0.0), m: Material::Checker { a: V3::new(0.9, 0.9, 0.9), b: V3::new(0.2, 0.2, 0.25), scale: 1.0 } }));
    shapes.push(Box::new(Sphere { c: V3::new(0.0, 0.0, -3.0), r: 1.0, m: Material::Diffuse { color: V3::new(0.8, 0.2, 0.2) } }));
    shapes.push(Box::new(Sphere { c: V3::new(2.1, 0.0, -3.5), r: 1.0, m: Material::Mirror { tint: V3::new(0.9, 0.9, 0.95), fuzz_free: true } }));
    shapes.push(Box::new(Sphere { c: V3::new(-2.0, -0.4, -2.5), r: 0.6, m: Material::Mirror { tint: V3::new(0.95, 0.8, 0.3), fuzz_free: false } }));
    shapes.push(Box::new(Sphere { c: V3::new(0.0, 3.0, -4.0), r: 0.3, m: Material::Emissive(V3::new(4.0, 4.0, 3.5)) }));
    shapes.push(Box::new(Triangle { a: V3::new(-1.0, -1.0, -5.0), b: V3::new(1.0, -1.0, -5.5), c: V3::new(0.0, 1.5, -5.2), m: Material::Diffuse { color: V3::new(0.2, 0.7, 0.3) } }));
    Scene {
        shapes,
        lights: vec![Light { pos: V3::new(5.0, 5.0, 0.0), color: V3::new(0.8, 0.8, 0.8) }, Light { pos: V3::new(-4.0, 3.0, -1.0), color: V3::new(0.3, 0.3, 0.5) }],
        ambient: V3::new(0.1, 0.1, 0.12),
        stats: Stats::default(),
    }
}

fn to_byte(x: f64) -> u8 {
    (x.clamp(0.0, 1.0).sqrt() * 255.0 + 0.5) as u8
}

fn main() {
    let (w, h) = (64usize, 32usize);
    let mut scene = build_scene();
    let cam = Camera::new(V3::new(0.0, 0.5, 1.0), V3::new(0.0, 0.0, -3.0), V3::new(0.0, 1.0, 0.0), 60.0, w as f64 / h as f64 * 0.5);
    let mut img = vec![V3::ZERO; w * h];
    for j in 0..h {
        for i in 0..w {
            let mut c = V3::ZERO;
            // 2x2 supersampling at fixed offsets
            for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let u = (i as f64 + dx) / w as f64;
                let v = ((h - 1 - j) as f64 + dy) / h as f64;
                c += scene.trace(&cam.ray(u, v), 0);
            }
            img[j * w + i] = c / 4.0;
        }
    }
    let ramp: Vec<char> = " .:-=+*#%@".chars().collect();
    for row in img.chunks(w) {
        let line: String = row
            .iter()
            .map(|c| {
                let lum = 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
                let idx = ((lum.clamp(0.0, 0.999)) * ramp.len() as f64) as usize;
                ramp[idx]
            })
            .collect();
        println!("|{}|", line);
    }
    let mut ppm = format!("P6\n{} {}\n255\n", w, h).into_bytes();
    for c in &img {
        ppm.extend_from_slice(&[to_byte(c.x), to_byte(c.y), to_byte(c.z)]);
    }
    let hash = ppm.iter().fold(0x811c9dc5u32, |h, &b| (h ^ b as u32).wrapping_mul(16777619));
    println!("ppm bytes={} fnv32={:08x}", ppm.len(), hash);
    let avg = img.iter().fold(V3::ZERO, |a, &c| a + c) / img.len() as f64;
    let brightest = img.iter().copied().fold(0.0f64, |m, c| m.max(c.max_component()));
    println!("avg={:.5} brightest={:.4} center={:.6}", avg, brightest, img[h / 2 * w + w / 2]);
    println!("{:?}", scene.stats);
    for s in &scene.shapes {
        print!("{} ", s.name());
    }
    println!();
    let v = V3::new(1.0, 2.0, 3.0);
    println!("{} {} {} {:.1} {}", v[0], v[1], v[2], v, -v);
    println!("{:e} {:.3e} {:?} {:?} {}", 1234.5f64, 0.000123f64, 0.1 + 0.2, 1e21f64, 1e-7f64);
    println!("{:?} {:?} {}", f64::NAN.max(1.0), (-0.0f64).to_bits(), f64::EPSILON);
}
