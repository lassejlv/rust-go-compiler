#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok { Lit(u8), Match { dist: u16, len: u16 } }

fn compress(data: &[u8], window: usize, min_len: usize) -> Vec<Tok> {
    let mut out = vec![];
    let mut i = 0;
    while i < data.len() {
        let start = i.saturating_sub(window);
        let mut best = (0usize, 0usize);
        for j in start..i {
            let mut l = 0;
            while i + l < data.len() && data[j + l] == data[i + l] && l < 258 { l += 1; }
            if l > best.1 { best = (i - j, l); }
        }
        if best.1 >= min_len { out.push(Tok::Match { dist: best.0 as u16, len: best.1 as u16 }); i += best.1; } else { out.push(Tok::Lit(data[i])); i += 1; }
    }
    out
}

fn decompress(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = vec![];
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match { dist, len } => { let s = out.len() - dist as usize; for k in 0..len as usize { let b = out[s + k]; out.push(b); } }
        }
    }
    out
}

struct BitWriter { bytes: Vec<u8>, cur: u64, n: u32 }
impl BitWriter {
    fn new() -> Self { BitWriter { bytes: vec![], cur: 0, n: 0 } }
    fn put(&mut self, v: u64, bits: u32) { self.cur |= (v & ((1u64 << bits) - 1)) << self.n; self.n += bits; while self.n >= 8 { self.bytes.push(self.cur as u8); self.cur >>= 8; self.n -= 8; } }
    fn finish(mut self) -> Vec<u8> { if self.n > 0 { self.bytes.push(self.cur as u8); } self.bytes }
}
struct BitReader<'a> { b: &'a [u8], pos: usize }
impl<'a> BitReader<'a> {
    fn get(&mut self, bits: u32) -> Option<u64> {
        let mut v = 0u64;
        for k in 0..bits as usize { let byte = *self.b.get((self.pos + k) / 8)?; v |= (((byte >> ((self.pos + k) % 8)) & 1) as u64) << k; }
        self.pos += bits as usize;
        Some(v)
    }
}

fn encode(toks: &[Tok]) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.put(toks.len() as u64, 24);
    for t in toks { match *t { Tok::Lit(b) => { w.put(0, 1); w.put(b as u64, 8); } Tok::Match { dist, len } => { w.put(1, 1); w.put(dist as u64, 12); w.put(len as u64 - 3, 8); } } }
    w.finish()
}
fn decode(b: &[u8]) -> Option<Vec<Tok>> {
    let mut r = BitReader { b, pos: 0 };
    let n = r.get(24)?;
    (0..n).map(|_| Some(if r.get(1)? == 0 { Tok::Lit(r.get(8)? as u8) } else { Tok::Match { dist: r.get(12)? as u16, len: r.get(8)? as u16 + 3 } })).collect()
}

fn main() {
    let inputs: Vec<Vec<u8>> = vec![
        b"abracadabra abracadabra abracadabra".to_vec(),
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
        b"no repeats here!".to_vec(),
        "the rain in spain falls mainly on the plain; the rain in spain stays mainly in the plain".bytes().collect(),
        (0..600u32).map(|i| (i * i % 251) as u8).collect(),
        vec![],
    ];
    for inp in &inputs {
        let toks = compress(inp, 4095, 3);
        let enc = encode(&toks);
        let dec = decode(&enc).expect("decode");
        let back = decompress(&dec);
        let matches = toks.iter().filter(|t| matches!(t, Tok::Match { .. })).count();
        println!("in={:>4} toks={:>4} matches={:>3} bytes={:>4} ratio={:.3} ok={}", inp.len(), toks.len(), matches, enc.len(), enc.len() as f64 / inp.len().max(1) as f64, back == *inp && dec == toks);
    }
    let t = compress(b"abcabcabcabcX", 100, 3);
    println!("{:?}", t);
    let mut w = BitWriter::new();
    for (v, b) in [(1u64, 1u32), (0b101, 3), (0xABC, 12), (u32::MAX as u64, 32), (0, 7)] { w.put(v, b); }
    let bytes = w.finish();
    println!("{:02x?}", bytes);
    let mut r = BitReader { b: &bytes, pos: 0 };
    println!("{:?}", [1u32, 3, 12, 32, 7, 8].iter().map(|&b| r.get(b)).collect::<Vec<_>>());
}
