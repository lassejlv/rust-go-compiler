use std::fmt::Debug;

// Byte source abstraction layered on Iterator with a fixed Item
trait ByteSource: Iterator<Item = u8> {
    fn name(&self) -> String;
}

struct Repeat { b: u8, n: usize }
impl Iterator for Repeat {
    type Item = u8;
    fn next(&mut self) -> Option<u8> { if self.n == 0 { None } else { self.n -= 1; Some(self.b) } }
}
impl ByteSource for Repeat { fn name(&self) -> String { format!("repeat({})", self.b) } }

struct FromStr { s: Vec<u8>, i: usize }
impl Iterator for FromStr {
    type Item = u8;
    fn next(&mut self) -> Option<u8> { let r = self.s.get(self.i).copied(); self.i += 1; r }
}
impl ByteSource for FromStr { fn name(&self) -> String { "str".to_string() } }

fn checksum(src: &mut dyn ByteSource) -> u32 {
    src.map(|b| b as u32).fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b))
}

fn drain_all(srcs: Vec<Box<dyn ByteSource>>) -> Vec<(String, Vec<u8>)> {
    srcs.into_iter().map(|mut s| { let n = s.name(); let v: Vec<u8> = s.by_ref().collect(); (n, v) }).collect()
}

// Key-value store abstraction with supertrait binding
trait Lookup { type Out; fn lookup(&self, k: &str) -> Option<Self::Out>; }
trait TextLookup: Lookup<Out = String> + Debug {
    fn lookup_upper(&self, k: &str) -> String { self.lookup(k).map(|s| s.to_uppercase()).unwrap_or_default() }
}
#[derive(Debug)]
struct Pairs(Vec<(&'static str, &'static str)>);
impl Lookup for Pairs {
    type Out = String;
    fn lookup(&self, k: &str) -> Option<String> { self.0.iter().find(|(a, _)| *a == k).map(|(_, b)| b.to_string()) }
}
impl TextLookup for Pairs {}

fn describe(t: &dyn TextLookup, keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| {
        let raw: Option<String> = t.lookup(k);
        let len = raw.as_ref().map(|s| s.len()).unwrap_or(0);
        format!("{}={:?}/{}/{}", k, raw, t.lookup_upper(k), len)
    }).collect()
}

fn main() {
    let mut r = Repeat { b: 7, n: 3 };
    println!("{}", checksum(&mut r));
    let mut f = FromStr { s: b"hello".to_vec(), i: 0 };
    println!("{}", checksum(&mut f));
    let srcs: Vec<Box<dyn ByteSource>> = vec![Box::new(Repeat { b: 1, n: 2 }), Box::new(FromStr { s: b"ab".to_vec(), i: 0 })];
    for (n, v) in drain_all(srcs) { println!("{} {:?}", n, v); }
    let mut bx: Box<dyn ByteSource> = Box::new(FromStr { s: vec![3, 4, 5], i: 0 });
    let first = bx.next();
    let rest: u32 = bx.map(u32::from).sum();
    println!("{:?} {}", first, rest);
    let p = Pairs(vec![("a", "apple"), ("b", "bear")]);
    for line in describe(&p, &["a", "b", "z"]) { println!("{}", line); }
    let boxed: Box<dyn TextLookup> = Box::new(p);
    println!("{:?}", boxed);
}
