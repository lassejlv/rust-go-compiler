// Huffman encoder/decoder: frequency count, deterministic tree build with a
// BinaryHeap, canonical codes, bit packing into bytes, and decoding.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Debug)]
enum Tree {
    Leaf(u8),
    Node(Box<Tree>, Box<Tree>),
}

impl Tree {
    fn depth_map(&self, depth: u32, out: &mut BTreeMap<u8, u32>) {
        match self {
            Tree::Leaf(b) => {
                out.insert(*b, depth.max(1));
            }
            Tree::Node(l, r) => {
                l.depth_map(depth + 1, out);
                r.depth_map(depth + 1, out);
            }
        }
    }
    fn count(&self) -> (usize, usize) {
        match self {
            Tree::Leaf(_) => (1, 0),
            Tree::Node(l, r) => {
                let (a, b) = l.count();
                let (c, d) = r.count();
                (a + c, b + d + 1)
            }
        }
    }
}

fn build(freq: &BTreeMap<u8, usize>) -> Tree {
    // (weight, tiebreak id) -> tree index
    let mut heap = BinaryHeap::new();
    let mut store: Vec<Option<Tree>> = Vec::new();
    for (&b, &w) in freq {
        store.push(Some(Tree::Leaf(b)));
        heap.push(Reverse((w, store.len() - 1)));
    }
    while heap.len() > 1 {
        let Reverse((w1, i1)) = heap.pop().unwrap();
        let Reverse((w2, i2)) = heap.pop().unwrap();
        let a = store[i1].take().unwrap();
        let b = store[i2].take().unwrap();
        store.push(Some(Tree::Node(Box::new(a), Box::new(b))));
        heap.push(Reverse((w1 + w2, store.len() - 1)));
    }
    let Reverse((_, i)) = heap.pop().unwrap();
    store[i].take().unwrap()
}

// canonical code assignment from lengths
fn canonical(lengths: &BTreeMap<u8, u32>) -> BTreeMap<u8, (u32, u32)> {
    let mut syms: Vec<(u32, u8)> = lengths.iter().map(|(&s, &l)| (l, s)).collect();
    syms.sort();
    let mut code = 0u32;
    let mut prev_len = syms[0].0;
    let mut out = BTreeMap::new();
    for (i, &(len, s)) in syms.iter().enumerate() {
        if i > 0 {
            code = (code + 1) << (len - prev_len);
        }
        prev_len = len;
        out.insert(s, (code, len));
    }
    out
}

struct BitWriter { bytes: Vec<u8>, acc: u8, n: u8, total: usize }
impl BitWriter {
    fn new() -> Self { BitWriter { bytes: vec![], acc: 0, n: 0, total: 0 } }
    fn push(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.acc = (self.acc << 1) | ((code >> i) & 1) as u8;
            self.n += 1;
            self.total += 1;
            if self.n == 8 {
                self.bytes.push(self.acc);
                self.acc = 0;
                self.n = 0;
            }
        }
    }
    fn finish(mut self) -> (Vec<u8>, usize) {
        if self.n > 0 {
            self.bytes.push(self.acc << (8 - self.n));
        }
        (self.bytes, self.total)
    }
}

fn decode(bytes: &[u8], nbits: usize, codes: &BTreeMap<u8, (u32, u32)>) -> Vec<u8> {
    let table: BTreeMap<(u32, u32), u8> = codes.iter().map(|(&s, &c)| (c, s)).collect();
    let mut out = Vec::new();
    let (mut code, mut len) = (0u32, 0u32);
    for i in 0..nbits {
        let bit = (bytes[i / 8] >> (7 - i % 8)) & 1;
        code = (code << 1) | bit as u32;
        len += 1;
        if let Some(&s) = table.get(&(code, len)) {
            out.push(s);
            code = 0;
            len = 0;
        }
    }
    out
}

fn show(b: u8) -> String {
    match b {
        b' ' => "' '".to_string(),
        b'\n' => "\\n".to_string(),
        _ => (b as char).to_string(),
    }
}

fn run(text: &str) {
    let mut freq = BTreeMap::new();
    for b in text.bytes() {
        *freq.entry(b).or_insert(0usize) += 1;
    }
    let tree = build(&freq);
    let (leaves, internal) = tree.count();
    let mut lens = BTreeMap::new();
    tree.depth_map(0, &mut lens);
    let codes = canonical(&lens);
    let mut w = BitWriter::new();
    for b in text.bytes() {
        let (c, l) = codes[&b];
        w.push(c, l);
    }
    let (bytes, nbits) = w.finish();
    let back = decode(&bytes, nbits, &codes);
    println!("text len {} -> {} bits ({} bytes), leaves={} internal={}", text.len(), nbits, bytes.len(), leaves, internal);
    let mut by_len: Vec<(&u8, &(u32, u32))> = codes.iter().collect();
    by_len.sort_by(|a, b| (a.1).1.cmp(&(b.1).1).then(a.0.cmp(b.0)));
    for (s, (c, l)) in by_len.iter().take(6) {
        println!("  {:>3} freq={:<3} code={:0width$b}", show(**s), freq[*s], c, width = *l as usize);
    }
    let hex: Vec<String> = bytes.iter().take(12).map(|b| format!("{:02x}", b)).collect();
    println!("  head: {}", hex.join(" "));
    let weighted: usize = freq.iter().map(|(s, f)| f * lens[s] as usize).sum();
    println!("  roundtrip ok: {}, weighted={} ratio={:.3}", back == text.as_bytes(), weighted, nbits as f64 / (text.len() * 8) as f64);
}

fn main() {
    run("abracadabra");
    run("the quick brown fox jumps over the lazy dog\nthe end\n");
    run("aaaaaaaabbbbccd");
    run("mississippi river");
    let single = "zzzz";
    run(single);
}
