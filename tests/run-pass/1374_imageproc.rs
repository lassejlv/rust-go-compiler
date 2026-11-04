// Image processing on a generated grayscale/RGB raster: a generic Image<P>
// over pixel types with a Pixel trait, convolution with separable and
// non-separable kernels (clamped borders), Sobel edges, Otsu thresholding,
// flood fill, connected-component labeling with union-find, morphology,
// bilinear resize, histogram equalization, run-length encoding, PGM output
// hashed with CRC-32, and ASCII previews.
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

trait Pixel: Copy + Default + Debug + PartialEq {
    const CHANNELS: usize;
    fn to_f32s(self) -> [f32; 3];
    fn from_f32s(v: [f32; 3]) -> Self;
    fn luma(self) -> u8 {
        let [r, g, b] = self.to_f32s();
        (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Gray(u8);
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Rgb(u8, u8, u8);

impl Pixel for Gray {
    const CHANNELS: usize = 1;
    fn to_f32s(self) -> [f32; 3] {
        [self.0 as f32; 3]
    }
    fn from_f32s(v: [f32; 3]) -> Self {
        Gray(v[0].round().clamp(0.0, 255.0) as u8)
    }
    fn luma(self) -> u8 {
        self.0
    }
}
impl Pixel for Rgb {
    const CHANNELS: usize = 3;
    fn to_f32s(self) -> [f32; 3] {
        [self.0 as f32, self.1 as f32, self.2 as f32]
    }
    fn from_f32s(v: [f32; 3]) -> Self {
        let c = |x: f32| x.round().clamp(0.0, 255.0) as u8;
        Rgb(c(v[0]), c(v[1]), c(v[2]))
    }
}

#[derive(Clone)]
struct Image<P> {
    w: usize,
    h: usize,
    px: Vec<P>,
}

impl<P: Pixel> Image<P> {
    fn new(w: usize, h: usize) -> Self {
        Image { w, h, px: vec![P::default(); w * h] }
    }
    fn from_fn(w: usize, h: usize, f: impl Fn(usize, usize) -> P) -> Self {
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                px.push(f(x, y));
            }
        }
        Image { w, h, px }
    }
    fn get_clamped(&self, x: isize, y: isize) -> P {
        let x = x.clamp(0, self.w as isize - 1) as usize;
        let y = y.clamp(0, self.h as isize - 1) as usize;
        self[(x, y)]
    }
    fn map<Q: Pixel>(&self, f: impl Fn(P) -> Q) -> Image<Q> {
        Image { w: self.w, h: self.h, px: self.px.iter().map(|&p| f(p)).collect() }
    }
    fn convolve(&self, k: &[f32], kw: usize, kh: usize) -> Image<P> {
        let (cx, cy) = ((kw / 2) as isize, (kh / 2) as isize);
        Image::from_fn(self.w, self.h, |x, y| {
            let mut acc = [0f32; 3];
            for j in 0..kh {
                for i in 0..kw {
                    let p = self.get_clamped(x as isize + i as isize - cx, y as isize + j as isize - cy).to_f32s();
                    let wgt = k[j * kw + i];
                    for c in 0..3 {
                        acc[c] += p[c] * wgt;
                    }
                }
            }
            P::from_f32s(acc)
        })
    }
    fn separable(&self, k: &[f32]) -> Image<P> {
        self.convolve(k, k.len(), 1).convolve(k, 1, k.len())
    }
    fn resize(&self, nw: usize, nh: usize) -> Image<P> {
        Image::from_fn(nw, nh, |x, y| {
            let fx = (x as f32 + 0.5) * self.w as f32 / nw as f32 - 0.5;
            let fy = (y as f32 + 0.5) * self.h as f32 / nh as f32 - 0.5;
            let (x0, y0) = (fx.floor() as isize, fy.floor() as isize);
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let s = |dx: isize, dy: isize| self.get_clamped(x0 + dx, y0 + dy).to_f32s();
            let (a, b, c, d) = (s(0, 0), s(1, 0), s(0, 1), s(1, 1));
            let mut out = [0f32; 3];
            for ch in 0..3 {
                let top = a[ch] * (1.0 - tx) + b[ch] * tx;
                let bot = c[ch] * (1.0 - tx) + d[ch] * tx;
                out[ch] = top * (1.0 - ty) + bot * ty;
            }
            P::from_f32s(out)
        })
    }
}

impl<P> Index<(usize, usize)> for Image<P> {
    type Output = P;
    fn index(&self, (x, y): (usize, usize)) -> &P {
        &self.px[y * self.w + x]
    }
}
impl<P> IndexMut<(usize, usize)> for Image<P> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut P {
        &mut self.px[y * self.w + x]
    }
}

fn sobel(img: &Image<Gray>) -> Image<Gray> {
    Image::from_fn(img.w, img.h, |x, y| {
        let g = |dx: isize, dy: isize| img.get_clamped(x as isize + dx, y as isize + dy).0 as i32;
        let gx = -g(-1, -1) - 2 * g(-1, 0) - g(-1, 1) + g(1, -1) + 2 * g(1, 0) + g(1, 1);
        let gy = -g(-1, -1) - 2 * g(0, -1) - g(1, -1) + g(-1, 1) + 2 * g(0, 1) + g(1, 1);
        Gray((((gx * gx + gy * gy) as f64).sqrt() / 4.0).min(255.0) as u8)
    })
}

fn histogram(img: &Image<Gray>) -> [u32; 256] {
    let mut h = [0u32; 256];
    for p in &img.px {
        h[p.0 as usize] += 1;
    }
    h
}

fn otsu(h: &[u32; 256]) -> u8 {
    let total: u64 = h.iter().map(|&c| c as u64).sum();
    let sum_all: f64 = h.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
    let (mut wb, mut sumb, mut best, mut thr) = (0u64, 0f64, -1f64, 0u8);
    for t in 0..256 {
        wb += h[t] as u64;
        if wb == 0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0 {
            break;
        }
        sumb += t as f64 * h[t] as f64;
        let mb = sumb / wb as f64;
        let mf = (sum_all - sumb) / wf as f64;
        let between = wb as f64 * wf as f64 * (mb - mf) * (mb - mf);
        if between > best {
            best = between;
            thr = t as u8;
        }
    }
    thr
}

fn equalize(img: &Image<Gray>) -> Image<Gray> {
    let h = histogram(img);
    let mut cdf = [0u32; 256];
    let mut acc = 0;
    for i in 0..256 {
        acc += h[i];
        cdf[i] = acc;
    }
    let min = *cdf.iter().find(|&&c| c > 0).unwrap();
    let n = img.px.len() as u32;
    img.map(|p| Gray((((cdf[p.0 as usize] - min) as f64 / (n - min).max(1) as f64) * 255.0).round() as u8))
}

fn flood_fill(img: &mut Image<Gray>, x: usize, y: usize, to: u8) -> usize {
    let from = img[(x, y)].0;
    if from == to {
        return 0;
    }
    let mut q = VecDeque::from([(x, y)]);
    let mut n = 0;
    img[(x, y)] = Gray(to);
    while let Some((cx, cy)) = q.pop_front() {
        n += 1;
        let neighbors = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
        for (nx, ny) in neighbors {
            if nx < img.w && ny < img.h && img[(nx, ny)].0 == from {
                img[(nx, ny)] = Gray(to);
                q.push_back((nx, ny));
            }
        }
    }
    n
}

fn find(p: &mut Vec<usize>, x: usize) -> usize {
    let mut r = x;
    while p[r] != r {
        r = p[r];
    }
    let mut c = x;
    while p[c] != r {
        let n = p[c];
        p[c] = r;
        c = n;
    }
    r
}

fn components(bin: &Image<Gray>) -> (Vec<u32>, HashMap<u32, (usize, (usize, usize, usize, usize))>) {
    let (w, h) = (bin.w, bin.h);
    let mut labels = vec![0usize; w * h];
    let mut parent = vec![0usize];
    for y in 0..h {
        for x in 0..w {
            if bin[(x, y)].0 == 0 {
                continue;
            }
            let left = if x > 0 { labels[y * w + x - 1] } else { 0 };
            let up = if y > 0 { labels[(y - 1) * w + x] } else { 0 };
            labels[y * w + x] = match (left, up) {
                (0, 0) => {
                    parent.push(parent.len());
                    parent.len() - 1
                }
                (l, 0) | (0, l) => l,
                (l, u) => {
                    let (a, b) = (find(&mut parent, l), find(&mut parent, u));
                    if a != b {
                        parent[a.max(b)] = a.min(b);
                    }
                    a.min(b)
                }
            };
        }
    }
    let mut remap: HashMap<usize, u32> = HashMap::new();
    let mut stats: HashMap<u32, (usize, (usize, usize, usize, usize))> = HashMap::new();
    let mut out = vec![0u32; w * h];
    for y in 0..h {
        for x in 0..w {
            let l = labels[y * w + x];
            if l == 0 {
                continue;
            }
            let root = find(&mut parent, l);
            let next = remap.len() as u32 + 1;
            let id = *remap.entry(root).or_insert(next);
            out[y * w + x] = id;
            let e = stats.entry(id).or_insert((0, (x, y, x, y)));
            e.0 += 1;
            let bb = &mut e.1;
            bb.0 = bb.0.min(x);
            bb.1 = bb.1.min(y);
            bb.2 = bb.2.max(x);
            bb.3 = bb.3.max(y);
        }
    }
    (out, stats)
}

fn morph(bin: &Image<Gray>, dilate: bool) -> Image<Gray> {
    Image::from_fn(bin.w, bin.h, |x, y| {
        let mut any = false;
        let mut all = true;
        for dy in -1..=1isize {
            for dx in -1..=1isize {
                let on = bin.get_clamped(x as isize + dx, y as isize + dy).0 > 0;
                any |= on;
                all &= on;
            }
        }
        Gray(if (dilate && any) || (!dilate && all) { 255 } else { 0 })
    })
}

fn rle(img: &Image<Gray>) -> Vec<(u8, u32)> {
    let mut out: Vec<(u8, u32)> = Vec::new();
    for p in &img.px {
        match out.last_mut() {
            Some((v, n)) if *v == p.0 => *n += 1,
            _ => out.push((p.0, 1)),
        }
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB88320 } else { c >> 1 };
        }
    }
    !c
}

fn pgm(img: &Image<Gray>) -> Vec<u8> {
    let mut v = format!("P5\n{} {}\n255\n", img.w, img.h).into_bytes();
    v.extend(img.px.iter().map(|p| p.0));
    v
}

fn ascii(img: &Image<Gray>, step: usize) -> String {
    let ramp = b" .:-=+*#%@";
    let mut s = String::new();
    for y in (0..img.h).step_by(step * 2) {
        for x in (0..img.w).step_by(step) {
            s.push(ramp[img[(x, y)].0 as usize * (ramp.len() - 1) / 255] as char);
        }
        s.push('\n');
    }
    s
}

fn main() {
    let (w, h) = (96, 64);
    // scene: gradient background, two discs, a ring, a bar, and noise
    let scene: Image<Rgb> = Image::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let in_disc = |cx: f32, cy: f32, r: f32| (fx - cx).powi(2) + (fy - cy).powi(2) <= r * r;
        let noise = ((x * 7 + y * 13) % 11) as u8;
        if in_disc(25.0, 30.0, 12.0) {
            Rgb(220, 40 + noise, 40)
        } else if in_disc(70.0, 20.0, 9.0) && !in_disc(70.0, 20.0, 5.0) {
            Rgb(30, 200, 60 + noise)
        } else if (60..90).contains(&x) && (45..52).contains(&y) {
            Rgb(240, 240, 240 - noise)
        } else {
            Rgb((x * 2) as u8 / 3 + noise, (y * 2) as u8 / 3, 60)
        }
    });
    let gray = scene.map(|p| Gray(p.luma()));
    println!("{}", ascii(&gray, 3));
    let blur = gray.separable(&[1.0 / 16.0, 4.0 / 16.0, 6.0 / 16.0, 4.0 / 16.0, 1.0 / 16.0]);
    let sharpen = gray.convolve(&[0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0], 3, 3);
    let edges = sobel(&blur);
    let hist = histogram(&gray);
    let t = otsu(&hist);
    let bin = gray.map(|p| Gray(if p.0 > t { 255 } else { 0 }));
    let opened = morph(&morph(&bin, false), true);
    let (labels, stats) = components(&opened);
    let mut comps: Vec<_> = stats.iter().map(|(id, (n, bb))| (*n, *id, *bb)).collect();
    comps.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    println!("otsu threshold {} foreground {} components {}", t, bin.px.iter().filter(|p| p.0 > 0).count(), comps.len());
    for (n, id, bb) in comps.iter().take(5) {
        println!("  component {} area {} bbox {:?}", id, n, bb);
    }
    println!("labels checksum {}", labels.iter().enumerate().map(|(i, &l)| (i as u64 + 1) * l as u64).sum::<u64>());
    let mut filled = bin.clone();
    let n = flood_fill(&mut filled, 0, 0, 128);
    println!("flood fill from corner recolored {} px; second fill {}", n, flood_fill(&mut filled, 0, 0, 128));
    let eq = equalize(&gray);
    let small = gray.resize(24, 16);
    let big = small.resize(48, 32);
    let rgb_small = scene.resize(12, 8);
    println!("{}", ascii(&small, 1));
    for (name, img) in [("gray", &gray), ("blur", &blur), ("sharpen", &sharpen), ("edges", &edges), ("bin", &bin), ("opened", &opened), ("eq", &eq), ("small", &small), ("big", &big)] {
        let data = pgm(img);
        let mean = img.px.iter().map(|p| p.0 as u32).sum::<u32>() as f32 / img.px.len() as f32;
        println!("{:<8} {}x{} crc {:08x} mean {:7.3} runs {}", name, img.w, img.h, crc32(&data), mean, rle(img).len());
    }
    println!("rgb small row0 {:?}", &rgb_small.px[..4]);
    let h2 = histogram(&eq);
    let nonzero = h2.iter().filter(|&&c| c > 0).count();
    let peak = h2.iter().enumerate().max_by_key(|(i, &c)| (c, std::cmp::Reverse(*i))).unwrap();
    println!("equalized: {} distinct levels, peak {:?}, channels {} {}", nonzero, peak, Gray::CHANNELS, Rgb::CHANNELS);
    let blank: Image<Gray> = Image::new(4, 4);
    println!("blank otsu {} components {}", otsu(&histogram(&blank)), components(&blank).1.len());
}
