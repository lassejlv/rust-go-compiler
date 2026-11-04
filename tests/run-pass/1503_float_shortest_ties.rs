// Shortest round-trip float formatting: when two shortest candidates are
// equally close to the exact value, std rounds up (printf rounds half-even).
fn h(acc: u64, s: &str) -> u64 {
    let mut a = acc;
    for b in s.bytes() {
        a = (a ^ b as u64).wrapping_mul(0x100000001b3);
    }
    a
}

fn main() {
    let f32s = [-3213467.25f32, 1171787.25, -273056.125, -1966589.25, 2097152.25, 0.5, 9.5];
    for v in f32s {
        println!("{} {:?} {:e} {:E} {:.1}", v, v, v, v, v);
    }
    let f64s = [2140756099613572.25f64, 919935858426680.25, 1396036721069038.25, 287338874024166.4, 4503599627370495.5, 1e23, 5e-324];
    for v in f64s {
        println!("{} {:?} {:e} {:.1}", v, v, v, v);
    }
    // 9.5f32 * 2^k style ties across magnitudes
    for k in 20..26 {
        let v = (1u32 << k) as f32 + 0.25 * ((1u32 << (k - 20)) as f32);
        println!("{} {:?}", v, -v);
    }
    let mut x: u64 = 0x12345678;
    let mut acc = [0u64; 2];
    for i in 0..3000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let f = if i % 3 == 0 { f64::from_bits(x) } else { (x >> 11) as f64 / (1u64 << 53) as f64 * 10f64.powi((i % 40) as i32 - 20) };
        let g = f32::from_bits((x >> 32) as u32);
        acc[0] = h(acc[0], &format!("{} {:?} {:e}", f, f, f));
        acc[1] = h(acc[1], &format!("{} {:?} {:e}", g, g, g));
        let back: f64 = format!("{:?}", f).parse().unwrap();
        assert!(back == f || (back.is_nan() && f.is_nan()));
    }
    println!("{:?}", acc);
}
