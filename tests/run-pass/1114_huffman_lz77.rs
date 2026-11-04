// Compression round trips: bit-level writer/reader, canonical Huffman coding,
// LZ77 with a hash-chain matcher, run-length encoding, and a combined
// LZ77+Huffman container with CRC32 checks.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

struct BitWriter {
    bytes: Vec<u8>,
    cur: u64,
    nbits: u32,
}

impl BitWriter {
    fn new() -> Self { BitWriter { bytes: Vec::new(), cur: 0, nbits: 0 } }
    fn write(&mut self, value: u64, bits: u32) {
        debug_assert!(bits <= 32);
        self.cur |= (value & ((1u64 << bits) - 1)) << self.nbits;
        self.nbits += bits;
        while self.nbits >= 8 {
            self.bytes.push(self.cur as u8);
            self.cur >>= 8;
            self.nbits -= 8;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 { self.bytes.push(self.cur as u8); }
        self.bytes
    }
}

struct BitReader<'a> { data: &'a [u8], pos: usize, cur: u64, nbits: u32 }

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self { BitReader { data, pos: 0, cur: 0, nbits: 0 } }
    fn read(&mut self, bits: u32) -> Option<u64> {
        while self.nbits < bits {
            let b = *self.data.get(self.pos)?;
            self.pos += 1;
            self.cur |= (b as u64) << self.nbits;
            self.nbits += 8;
        }
        let v = self.cur & ((1u64 << bits) - 1);
        self.cur >>= bits;
        self.nbits -= bits;
        Some(v)
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 { c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 }; }
        table[i as usize] = c;
    }
    !data.iter().fold(!0u32, |c, &b| table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
}

// ---------- Huffman ----------
#[derive(Debug)]
enum Tree { Leaf(u16), Node(Box<Tree>, Box<Tree>) }

fn code_lengths(freqs: &BTreeMap<u16, u64>, max_len: u8) -> BTreeMap<u16, u8> {
    let mut lens = BTreeMap::new();
    if freqs.is_empty() {
        return lens;
    }
    if freqs.len() == 1 {
        lens.insert(*freqs.keys().next().unwrap(), 1);
        return lens;
    }
    // heap of (weight, tiebreak, tree)
    let mut heap: BinaryHeap<Reverse<(u64, u32, usize)>> = BinaryHeap::new();
    let mut arena: Vec<Option<Tree>> = Vec::new();
    for (i, (&sym, &f)) in freqs.iter().enumerate() {
        arena.push(Some(Tree::Leaf(sym)));
        heap.push(Reverse((f, i as u32, arena.len() - 1)));
    }
    let mut tick = freqs.len() as u32;
    while heap.len() > 1 {
        let Reverse((f1, _, a)) = heap.pop().unwrap();
        let Reverse((f2, _, b)) = heap.pop().unwrap();
        let t = Tree::Node(Box::new(arena[a].take().unwrap()), Box::new(arena[b].take().unwrap()));
        arena.push(Some(t));
        heap.push(Reverse((f1 + f2, tick, arena.len() - 1)));
        tick += 1;
    }
    let Reverse((_, _, root)) = heap.pop().unwrap();
    fn walk(t: &Tree, depth: u8, out: &mut BTreeMap<u16, u8>) {
        match t {
            Tree::Leaf(s) => { out.insert(*s, depth.max(1)); }
            Tree::Node(l, r) => { walk(l, depth + 1, out); walk(r, depth + 1, out); }
        }
    }
    walk(arena[root].as_ref().unwrap(), 0, &mut lens);
    // length limiting (simple heuristic): clamp and fix Kraft inequality
    let mut changed = false;
    for l in lens.values_mut() {
        if *l > max_len { *l = max_len; changed = true; }
    }
    if changed {
        loop {
            let kraft: u64 = lens.values().map(|&l| 1u64 << (max_len - l)).sum();
            if kraft <= 1u64 << max_len { break; }
            // lengthen the shortest code that can be lengthened
            let (&s, _) = lens.iter().filter(|(_, &l)| l < max_len).max_by_key(|(&s, &l)| (l, Reverse(s))).unwrap();
            *lens.get_mut(&s).unwrap() += 1;
        }
    }
    lens
}

fn canonical(lens: &BTreeMap<u16, u8>) -> HashMap<u16, (u32, u8)> {
    let mut syms: Vec<(u8, u16)> = lens.iter().map(|(&s, &l)| (l, s)).collect();
    syms.sort();
    let mut codes = HashMap::new();
    let mut code = 0u32;
    let mut prev_len = 0u8;
    for (l, s) in syms {
        code <<= l - prev_len;
        codes.insert(s, (code, l));
        code += 1;
        prev_len = l;
    }
    codes
}

fn reverse_bits(v: u32, n: u8) -> u32 { v.reverse_bits() >> (32 - n as u32) }

fn huff_encode(symbols: &[u16], w: &mut BitWriter) -> BTreeMap<u16, u8> {
    let mut freqs = BTreeMap::new();
    for &s in symbols { *freqs.entry(s).or_insert(0u64) += 1; }
    let lens = code_lengths(&freqs, 15);
    let codes = canonical(&lens);
    w.write(lens.len() as u64, 16);
    for (&s, &l) in &lens { w.write(s as u64, 9); w.write(l as u64, 4); }
    w.write(symbols.len() as u64, 32);
    for s in symbols {
        let (c, l) = codes[s];
        w.write(reverse_bits(c, l) as u64, l as u32);
    }
    lens
}

fn huff_decode(r: &mut BitReader) -> Option<Vec<u16>> {
    let n = r.read(16)? as usize;
    let mut lens = BTreeMap::new();
    for _ in 0..n { let s = r.read(9)? as u16; let l = r.read(4)? as u8; lens.insert(s, l); }
    let codes = canonical(&lens);
    let lookup: HashMap<(u32, u8), u16> = codes.iter().map(|(&s, &(c, l))| ((c, l), s)).collect();
    let count = r.read(32)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let mut code = 0u32;
        let mut len = 0u8;
        loop {
            code = (code << 1) | r.read(1)? as u32;
            len += 1;
            if let Some(&s) = lookup.get(&(code, len)) { out.push(s); break; }
            if len > 15 { return None; }
        }
    }
    Some(out)
}

// ---------- LZ77 ----------
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok { Lit(u8), Match { dist: u16, len: u16 } }

fn lz77(data: &[u8], window: usize, min_len: usize, max_len: usize) -> Vec<Tok> {
    let mut head: HashMap<[u8; 3], usize> = HashMap::new();
    let mut prev: Vec<usize> = vec![usize::MAX; data.len()];
    let mut out = Vec::new();
    let mut i = 0;
    let insert = |pos: usize, head: &mut HashMap<[u8; 3], usize>, prev: &mut Vec<usize>| {
        if pos + 3 <= data.len() {
            let key = [data[pos], data[pos + 1], data[pos + 2]];
            if let Some(old) = head.insert(key, pos) { prev[pos] = old; }
        }
    };
    while i < data.len() {
        let mut best = (0usize, 0usize);
        if i + 3 <= data.len() {
            let key = [data[i], data[i + 1], data[i + 2]];
            let mut cand = head.get(&key).copied();
            let mut chain = 0;
            while let Some(c) = cand {
                if i - c > window || chain > 64 { break; }
                let l = data[c..].iter().zip(&data[i..]).take(max_len).take_while(|(a, b)| a == b).count();
                if l > best.1 { best = (i - c, l); }
                chain += 1;
                cand = if prev[c] == usize::MAX { None } else { Some(prev[c]) };
            }
        }
        if best.1 >= min_len {
            out.push(Tok::Match { dist: best.0 as u16, len: best.1 as u16 });
            for k in i..i + best.1 { insert(k, &mut head, &mut prev); }
            i += best.1;
        } else {
            out.push(Tok::Lit(data[i]));
            insert(i, &mut head, &mut prev);
            i += 1;
        }
    }
    out
}

fn unlz77(toks: &[Tok]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match { dist, len } => {
                let d = dist as usize;
                if d == 0 || d > out.len() { return Err(format!("bad distance {} at {}", d, out.len())); }
                let start = out.len() - d;
                for k in 0..len as usize { let b = out[start + k]; out.push(b); } // overlapping copy
            }
        }
    }
    Ok(out)
}

fn rle(data: &[u8]) -> Vec<(u8, usize)> {
    let mut v: Vec<(u8, usize)> = Vec::new();
    for &b in data {
        match v.last_mut() {
            Some((last, n)) if *last == b => *n += 1,
            _ => v.push((b, 1)),
        }
    }
    v
}

// container: symbols 0..255 literals, 256+k = match with len bucket; extra bits raw
fn compress(data: &[u8]) -> Vec<u8> {
    let toks = lz77(data, 4096, 3, 258);
    let mut syms = Vec::new();
    let mut extras: Vec<(u16, u16)> = Vec::new();
    for t in &toks {
        match t {
            Tok::Lit(b) => syms.push(*b as u16),
            Tok::Match { dist, len } => { syms.push(256 + (*len).min(255)); extras.push((*dist, *len)); }
        }
    }
    let mut w = BitWriter::new();
    w.write(crc32(data) as u64, 32);
    huff_encode(&syms, &mut w);
    w.write(extras.len() as u64, 32);
    for (d, l) in extras { w.write(d as u64, 12); w.write(l as u64, 9); }
    w.finish()
}

fn decompress(buf: &[u8]) -> Result<Vec<u8>, String> {
    let mut r = BitReader::new(buf);
    let crc = r.read(32).ok_or("truncated header")? as u32;
    let syms = huff_decode(&mut r).ok_or("bad huffman stream")?;
    let ne = r.read(32).ok_or("truncated extras")? as usize;
    let mut extras = Vec::with_capacity(ne);
    for _ in 0..ne { extras.push((r.read(12).ok_or("trunc")? as u16, r.read(9).ok_or("trunc")? as u16)); }
    let mut ex = extras.into_iter();
    let toks: Vec<Tok> = syms.iter().map(|&s| if s < 256 { Ok(Tok::Lit(s as u8)) } else {
        ex.next().map(|(dist, len)| Tok::Match { dist, len }).ok_or("missing extra".to_string())
    }).collect::<Result<_, _>>()?;
    let out = unlz77(&toks)?;
    if crc32(&out) != crc { return Err(format!("crc mismatch {:08x} != {:08x}", crc32(&out), crc)); }
    Ok(out)
}

fn main() {
    println!("crc32(\"\") = {:08x}", crc32(b""));
    println!("crc32(\"123456789\") = {:08x}", crc32(b"123456789"));
    println!("crc32(fox) = {:08x}", crc32(b"The quick brown fox jumps over the lazy dog"));

    let mut w = BitWriter::new();
    for (v, b) in [(1u64, 1u32), (5, 3), (0x1ff, 9), (0, 2), (0xdeadbeef, 32), (3, 2)] { w.write(v, b); }
    let bytes = w.finish();
    println!("bits packed: {:02x?}", bytes);
    let mut r = BitReader::new(&bytes);
    let back: Vec<Option<u64>> = [1, 3, 9, 2, 32, 2, 8].iter().map(|&b| r.read(b)).collect();
    println!("bits read: {:x?}", back);

    let text = "abracadabra abracadabra abracadabra! she sells sea shells by the sea shore";
    let syms: Vec<u16> = text.bytes().map(|b| b as u16).collect();
    let mut w = BitWriter::new();
    let lens = huff_encode(&syms, &mut w);
    let enc = w.finish();
    let mut by_len: BTreeMap<u8, Vec<char>> = BTreeMap::new();
    for (s, l) in &lens { by_len.entry(*l).or_default().push(*s as u8 as char); }
    println!("huffman code lengths: {:?}", by_len);
    let dec = huff_decode(&mut BitReader::new(&enc)).unwrap();
    println!("huffman: {} bytes -> {} bytes, round trip {}", text.len(), enc.len(), dec == syms);

    let skew: Vec<u16> = (0..40u16).flat_map(|i| std::iter::repeat(i).take(1usize << (i % 20).min(12))).collect();
    let mut freqs = BTreeMap::new();
    for &s in &skew { *freqs.entry(s).or_insert(0u64) += 1; }
    let limited = code_lengths(&freqs, 7);
    let kraft: f64 = limited.values().map(|&l| 0.5f64.powi(l as i32)).sum();
    println!("length-limited max={} kraft={:.4} n={}", limited.values().max().unwrap(), kraft, limited.len());
    let mut w = BitWriter::new();
    huff_encode(&skew, &mut w);
    let enc = w.finish();
    println!("skewed: {} symbols -> {} bytes, ok {}", skew.len(), enc.len(), huff_decode(&mut BitReader::new(&enc)).as_deref() == Some(&skew[..]));

    let toks = lz77(b"aaaaaaaaaabcabcabcabcabcx", 32, 3, 16);
    println!("lz77 tokens: {:?}", toks);
    println!("unlz77: {:?}", String::from_utf8(unlz77(&toks).unwrap()));
    println!("bad stream: {:?}", unlz77(&[Tok::Lit(1), Tok::Match { dist: 5, len: 2 }]));
    println!("rle: {:?}", rle(b"aaabccddddde"));

    let mut corpus: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("one", b"x".to_vec()),
        ("text", "It was the best of times, it was the worst of times, it was the age of wisdom, it was the age of foolishness, it was the epoch of belief, it was the epoch of incredulity".as_bytes().to_vec()),
        ("zeros", vec![0u8; 5000]),
        ("utf8", "héllo wörld — ünïcödé ✓ ".repeat(20).into_bytes()),
    ];
    let mut x: u32 = 1;
    corpus.push(("noise", (0..3000).map(|_| { x ^= x << 13; x ^= x >> 17; x ^= x << 5; (x >> 24) as u8 }).collect()));
    corpus.push(("pattern", (0..4000u32).map(|i| ((i * i) % 7 + (i / 100) % 3) as u8 + b'a').collect()));
    println!("{:<8} {:>6} {:>6} {:>7} {}", "name", "in", "out", "ratio", "ok");
    for (name, data) in &corpus {
        let c = compress(data);
        let d = decompress(&c);
        let ok = d.as_ref().map(|v| v == data).unwrap_or(false);
        let ratio = if data.is_empty() { 0.0 } else { c.len() as f64 / data.len() as f64 };
        println!("{:<8} {:>6} {:>6} {:>7.3} {}", name, data.len(), c.len(), ratio, ok);
    }
    let mut c = compress(b"hello hello hello hello");
    let n = c.len();
    c[n - 2] ^= 0x10;
    println!("corrupted: {:?}", decompress(&c).map(|v| v.len()));
    println!("truncated: {:?}", decompress(&c[..3]).map(|v| v.len()));
}
