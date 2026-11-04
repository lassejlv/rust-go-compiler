// LZ77 + canonical Huffman compressor/decompressor with hand-written CRC32 and
// Adler-32, a bit writer/reader, and round-trip verification over generated data.
use std::collections::{BinaryHeap, HashMap};
use std::cmp::{Ordering, Reverse};
use std::error::Error;
use std::fmt;

const CRC_POLY: u32 = 0xEDB8_8320;

struct Crc32 {
    table: [u32; 256],
}

impl Crc32 {
    fn new() -> Self {
        let mut table = [0u32; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { CRC_POLY ^ (c >> 1) } else { c >> 1 };
            }
            *slot = c;
        }
        Crc32 { table }
    }
    fn checksum(&self, data: &[u8]) -> u32 {
        !data.iter().fold(!0u32, |c, &b| self.table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
    }
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    fn write(&mut self, value: u32, bits: u32) {
        debug_assert!(bits <= 32);
        self.acc |= (value as u64) << self.nbits;
        self.nbits += bits;
        while self.nbits >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }
    fn write_rev(&mut self, code: u32, len: u32) {
        // huffman codes are written MSB-first
        let mut r = 0;
        for i in 0..len {
            r |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.write(r, len);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u64,
    nbits: u32,
}

#[derive(Debug, PartialEq)]
enum DecodeError {
    Truncated(usize),
    BadCode,
    BadDistance { dist: usize, have: usize },
    ChecksumMismatch { want: u32, got: u32 },
    BadMagic([u8; 4]),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated(p) => write!(f, "truncated input at byte {}", p),
            DecodeError::BadCode => f.write_str("invalid huffman code"),
            DecodeError::BadDistance { dist, have } => write!(f, "distance {} exceeds output size {}", dist, have),
            DecodeError::ChecksumMismatch { want, got } => write!(f, "checksum mismatch: want {:08x}, got {:08x}", want, got),
            DecodeError::BadMagic(m) => write!(f, "bad magic {:?}", m),
        }
    }
}

impl Error for DecodeError {}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0, acc: 0, nbits: 0 }
    }
    fn refill(&mut self, need: u32) -> Result<(), DecodeError> {
        while self.nbits < need {
            let b = *self.data.get(self.pos).ok_or(DecodeError::Truncated(self.pos))?;
            self.acc |= (b as u64) << self.nbits;
            self.pos += 1;
            self.nbits += 8;
        }
        Ok(())
    }
    fn read(&mut self, bits: u32) -> Result<u32, DecodeError> {
        if bits == 0 {
            return Ok(0);
        }
        self.refill(bits)?;
        let v = (self.acc & ((1u64 << bits) - 1)) as u32;
        self.acc >>= bits;
        self.nbits -= bits;
        Ok(v)
    }
    fn bit(&mut self) -> Result<u32, DecodeError> {
        self.read(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Lit(u8),
    Match { len: u16, dist: u16 },
}

const WINDOW: usize = 4096;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;

fn lz77(data: &[u8]) -> Vec<Token> {
    let mut chains: HashMap<[u8; 3], Vec<usize>> = HashMap::new();
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let mut best = (0usize, 0usize);
        if i + MIN_MATCH <= data.len() {
            let key = [data[i], data[i + 1], data[i + 2]];
            if let Some(cands) = chains.get(&key) {
                for &c in cands.iter().rev().take(32) {
                    if i - c > WINDOW {
                        break;
                    }
                    let l = data[c..].iter().zip(&data[i..]).take(MAX_MATCH).take_while(|(a, b)| a == b).count();
                    if l > best.0 {
                        best = (l, i - c);
                        if l == MAX_MATCH {
                            break;
                        }
                    }
                }
            }
        }
        let step = if best.0 >= MIN_MATCH {
            out.push(Token::Match { len: best.0 as u16, dist: best.1 as u16 });
            best.0
        } else {
            out.push(Token::Lit(data[i]));
            1
        };
        for j in i..(i + step) {
            if j + MIN_MATCH <= data.len() {
                chains.entry([data[j], data[j + 1], data[j + 2]]).or_default().push(j);
            }
        }
        i += step;
    }
    out
}

// symbol alphabet: 0..=255 literals, 256 = end, 257.. = length buckets
const END: usize = 256;
const NSYM: usize = 256 + 1 + 16;

fn len_bucket(len: usize) -> (usize, u32, u32) {
    // bucket b covers lengths [3 + 16*b, 3 + 16*(b+1)), extra 4 bits
    let b = (len - MIN_MATCH) / 16;
    (257 + b, 4, ((len - MIN_MATCH) % 16) as u32)
}

#[derive(Debug, PartialEq, Eq)]
struct HNode {
    freq: u64,
    order: usize,
    kind: HKind,
}

#[derive(Debug, PartialEq, Eq)]
enum HKind {
    Leaf(usize),
    Inner(Box<HNode>, Box<HNode>),
}

impl Ord for HNode {
    fn cmp(&self, o: &Self) -> Ordering {
        self.freq.cmp(&o.freq).then_with(|| self.order.cmp(&o.order))
    }
}
impl PartialOrd for HNode {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn code_lengths(freqs: &[u64], max_len: u8) -> Vec<u8> {
    let mut heap = BinaryHeap::new();
    let mut order = 0;
    for (s, &f) in freqs.iter().enumerate() {
        if f > 0 {
            heap.push(Reverse(HNode { freq: f, order, kind: HKind::Leaf(s) }));
            order += 1;
        }
    }
    let mut lens = vec![0u8; freqs.len()];
    if heap.len() == 1 {
        if let Some(Reverse(HNode { kind: HKind::Leaf(s), .. })) = heap.pop() {
            lens[s] = 1;
        }
        return lens;
    }
    while heap.len() > 1 {
        let Reverse(a) = heap.pop().unwrap();
        let Reverse(b) = heap.pop().unwrap();
        heap.push(Reverse(HNode { freq: a.freq + b.freq, order, kind: HKind::Inner(Box::new(a), Box::new(b)) }));
        order += 1;
    }
    fn walk(n: &HNode, d: u8, lens: &mut [u8]) {
        match &n.kind {
            HKind::Leaf(s) => lens[*s] = d.max(1),
            HKind::Inner(a, b) => {
                walk(a, d + 1, lens);
                walk(b, d + 1, lens);
            }
        }
    }
    if let Some(Reverse(root)) = heap.pop() {
        walk(&root, 0, &mut lens);
    }
    // length limiting: crude but deterministic — flatten too-long codes and fix Kraft sum
    if lens.iter().any(|&l| l > max_len) {
        for l in lens.iter_mut() {
            if *l > max_len {
                *l = max_len;
            }
        }
        loop {
            let kraft: u64 = lens.iter().filter(|&&l| l > 0).map(|&l| 1u64 << (max_len - l)).sum();
            if kraft <= 1u64 << max_len {
                break;
            }
            // lengthen the shortest code that is still shorter than max
            let (idx, _) = lens.iter().enumerate().filter(|(_, &l)| l > 0 && l < max_len).max_by_key(|(i, &l)| (l, *i)).unwrap();
            lens[idx] += 1;
        }
    }
    lens
}

fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let max = *lens.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; max + 1];
    for &l in lens {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; max + 2];
    let mut code = 0;
    for bits in 1..=max {
        code = (code + bl_count[bits - 1]) << 1;
        next[bits] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                0
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                c
            }
        })
        .collect()
}

struct Decoder {
    // (len, code) -> symbol, via sorted table
    table: HashMap<(u8, u32), u16>,
    max_len: u8,
}

impl Decoder {
    fn new(lens: &[u8]) -> Self {
        let codes = canonical_codes(lens);
        let table = lens.iter().zip(&codes).enumerate().filter(|(_, (&l, _))| l > 0).map(|(s, (&l, &c))| ((l, c), s as u16)).collect();
        Decoder { table, max_len: *lens.iter().max().unwrap_or(&0) }
    }
    fn decode(&self, r: &mut BitReader) -> Result<usize, DecodeError> {
        let mut code = 0u32;
        for len in 1..=self.max_len {
            code = (code << 1) | r.bit()?;
            if let Some(&s) = self.table.get(&(len, code)) {
                return Ok(s as usize);
            }
        }
        Err(DecodeError::BadCode)
    }
}

const MAGIC: [u8; 4] = *b"LZHF";

fn compress(data: &[u8], crc: &Crc32) -> Vec<u8> {
    let toks = lz77(data);
    let mut freqs = vec![0u64; NSYM];
    let mut dist_freqs = vec![0u64; 13];
    for t in &toks {
        match *t {
            Token::Lit(b) => freqs[b as usize] += 1,
            Token::Match { len, dist } => {
                freqs[len_bucket(len as usize).0] += 1;
                dist_freqs[dist_bucket(dist as usize).0] += 1;
            }
        }
    }
    freqs[END] += 1;
    let lens = code_lengths(&freqs, 15);
    let dlens = code_lengths(&dist_freqs, 15);
    let codes = canonical_codes(&lens);
    let dcodes = canonical_codes(&dlens);
    let mut w = BitWriter::default();
    for &m in MAGIC.iter() {
        w.write(m as u32, 8);
    }
    w.write(data.len() as u32, 32);
    w.write(crc.checksum(data), 32);
    for &l in lens.iter().chain(dlens.iter()) {
        w.write(l as u32, 4);
    }
    for t in &toks {
        match *t {
            Token::Lit(b) => w.write_rev(codes[b as usize], lens[b as usize] as u32),
            Token::Match { len, dist } => {
                let (s, eb, ev) = len_bucket(len as usize);
                w.write_rev(codes[s], lens[s] as u32);
                w.write(ev, eb);
                let (ds, deb, dev) = dist_bucket(dist as usize);
                w.write_rev(dcodes[ds], dlens[ds] as u32);
                w.write(dev, deb);
            }
        }
    }
    w.write_rev(codes[END], lens[END] as u32);
    w.finish()
}

fn dist_bucket(d: usize) -> (usize, u32, u32) {
    // bucket b covers [2^b, 2^(b+1)), extra b bits
    let b = (usize::BITS - 1 - d.leading_zeros()) as usize;
    (b, b as u32, (d - (1 << b)) as u32)
}

fn decompress(buf: &[u8], crc: &Crc32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut r = BitReader::new(buf);
    let mut magic = [0u8; 4];
    for m in magic.iter_mut() {
        *m = r.read(8)? as u8;
    }
    if magic != MAGIC {
        return Err(Box::new(DecodeError::BadMagic(magic)));
    }
    let n = r.read(32)? as usize;
    let want = r.read(32)?;
    let mut lens = vec![0u8; NSYM];
    let mut dlens = vec![0u8; 13];
    for l in lens.iter_mut().chain(dlens.iter_mut()) {
        *l = r.read(4)? as u8;
    }
    let dec = Decoder::new(&lens);
    let ddec = Decoder::new(&dlens);
    let mut out: Vec<u8> = Vec::with_capacity(n);
    loop {
        let s = dec.decode(&mut r)?;
        if s < 256 {
            out.push(s as u8);
        } else if s == END {
            break;
        } else {
            let b = s - 257;
            let len = MIN_MATCH + b * 16 + r.read(4)? as usize;
            let db = ddec.decode(&mut r)?;
            let dist = (1usize << db) + r.read(db as u32)? as usize;
            if dist > out.len() {
                return Err(DecodeError::BadDistance { dist, have: out.len() }.into());
            }
            let start = out.len() - dist;
            for k in 0..len {
                let b = out[start + k];
                out.push(b);
            }
        }
    }
    let got = crc.checksum(&out);
    if got != want || out.len() != n {
        return Err(Box::new(DecodeError::ChecksumMismatch { want, got }));
    }
    Ok(out)
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), vec![]));
    v.push(("one".into(), b"a".to_vec()));
    v.push(("abc-repeat".into(), b"abc".repeat(500)));
    let words = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do"];
    let mut rng = Lcg(42);
    let mut text = String::new();
    for i in 0..2000 {
        if i > 0 {
            text.push(if i % 17 == 0 { '\n' } else { ' ' });
        }
        text.push_str(words[(rng.next() % words.len() as u32) as usize]);
    }
    v.push(("lorem".into(), text.into_bytes()));
    let noise: Vec<u8> = (0..3000).map(|_| (rng.next() & 0xff) as u8).collect();
    v.push(("noise".into(), noise));
    let skew: Vec<u8> = (0..5000).map(|_| { let r = rng.next() % 100; if r < 70 { b'x' } else if r < 90 { b'y' } else { b'a' + (r % 26) as u8 } }).collect();
    v.push(("skewed".into(), skew));
    let mut src = Vec::new();
    for i in 0..300u32 {
        src.extend_from_slice(format!("fn f{}(x: u32) -> u32 {{ x.wrapping_mul({}) ^ {} }}\n", i, i * 7 + 1, i % 13).as_bytes());
    }
    v.push(("code".into(), src));
    v.push(("zeros".into(), vec![0u8; 10000]));
    v
}

fn main() -> Result<(), Box<dyn Error>> {
    let crc = Crc32::new();
    println!("crc32(\"123456789\") = {:08x}", crc.checksum(b"123456789"));
    println!("adler32(\"Wikipedia\") = {:08x}", adler32(b"Wikipedia"));
    println!("fnv1a(\"hello\") = {:016x}", fnv1a(b"hello"));

    let mut total_in = 0usize;
    let mut total_out = 0usize;
    for (name, data) in corpus() {
        let toks = lz77(&data);
        let nmatch = toks.iter().filter(|t| matches!(t, Token::Match { .. })).count();
        let c = compress(&data, &crc);
        let d = decompress(&c, &crc)?;
        assert_eq!(d, data, "round trip failed for {}", name);
        total_in += data.len();
        total_out += c.len();
        let ratio = if data.is_empty() { 0.0 } else { c.len() as f64 / data.len() as f64 };
        println!(
            "{:<12} in={:>6} out={:>6} ratio={:>6.3} tokens={:>5} matches={:>5} crc={:08x} adler={:08x} fnv={:016x}",
            name,
            data.len(),
            c.len(),
            ratio,
            toks.len(),
            nmatch,
            crc.checksum(&data),
            adler32(&data),
            fnv1a(&c)
        );
    }
    println!("total {} -> {}", total_in, total_out);

    // corruption detection
    let data = b"the quick brown fox jumps over the lazy dog. the quick brown fox!".to_vec();
    let good = compress(&data, &crc);
    let mut errors = Vec::new();
    for i in [0usize, 5, 9, 20, good.len() - 1] {
        let mut bad = good.clone();
        bad[i] ^= 0x10;
        match decompress(&bad, &crc) {
            Ok(d) => errors.push(format!("flip@{}: ok? same={}", i, d == data)),
            Err(e) => errors.push(format!("flip@{}: {}", i, e)),
        }
    }
    match decompress(&good[..good.len() / 2], &crc) {
        Ok(_) => errors.push("trunc: ok?".into()),
        Err(e) => errors.push(format!("trunc: {}", e)),
    }
    for e in &errors {
        println!("{}", e);
    }
    let lens = code_lengths(&[1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536], 7);
    println!("limited lens {:?}", lens);
    println!("codes {:?}", canonical_codes(&[2, 1, 3, 3]));
    Ok(())
}
