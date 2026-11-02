// FxHash-like hasher used through BuildHasherDefault, as in rustc-hash / collections crates.
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: u64) {
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add_to_hash(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let rem = chunks.remainder();
        if rem.len() >= 4 {
            self.add_to_hash(u32::from_le_bytes(rem[..4].try_into().unwrap()) as u64);
            for b in &rem[4..] {
                self.add_to_hash(*b as u64);
            }
        } else {
            for b in rem {
                self.add_to_hash(*b as u64);
            }
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as u64);
    }
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i);
    }
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

type FxBuildHasher = BuildHasherDefault<FxHasher>;
type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;
type FxHashSet<T> = HashSet<T, FxBuildHasher>;

fn fx_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut h = FxHasher::default();
    t.hash(&mut h);
    h.finish()
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
enum Key {
    Id(u32),
    Name(String),
    Pair(u8, u8),
}

fn main() {
    println!("{:016x}", fx_hash(&0u64));
    println!("{:016x}", fx_hash(&1u64));
    println!("{:016x}", fx_hash(&12345u32));
    println!("{:016x}", fx_hash(&255u8));
    println!("{:016x}", fx_hash(b"abcdefghijk".as_slice()));
    println!("{:016x}", fx_hash("hello"));
    println!("{:016x}", fx_hash(&Point { x: 1, y: -1 }));
    println!("{:016x}", fx_hash(&(1u8, 2u32, 3u64)));
    println!("{:016x}", fx_hash(&Key::Name("zed".into())));

    let bh = FxBuildHasher::default();
    println!("{:016x}", bh.hash_one(42u64));
    let mut h1 = bh.build_hasher();
    h1.write_u32(7);
    println!("{:016x}", h1.finish());

    let mut m: FxHashMap<Point, &str> = FxHashMap::default();
    m.insert(Point { x: 0, y: 0 }, "origin");
    m.insert(Point { x: 1, y: 2 }, "a");
    m.insert(Point { x: -5, y: 3 }, "b");
    m.insert(Point { x: 1, y: 2 }, "a2");
    let mut entries: Vec<_> = m.iter().collect();
    entries.sort();
    println!("{:?}", entries);
    println!("{:?} {:?}", m.get(&Point { x: 1, y: 2 }), m.get(&Point { x: 9, y: 9 }));

    let mut counts: FxHashMap<Key, usize> = HashMap::with_hasher(Default::default());
    let keys = vec![Key::Id(1), Key::Name("a".into()), Key::Id(1), Key::Pair(1, 2), Key::Name("a".into()), Key::Id(2)];
    for k in keys {
        *counts.entry(k).or_insert(0) += 1;
    }
    let mut cv: Vec<_> = counts.into_iter().collect();
    cv.sort();
    println!("{:?}", cv);

    let mut set: FxHashSet<u64> = FxHashSet::default();
    for i in 0..1000u64 {
        set.insert(i * i % 97);
    }
    let mut sv: Vec<_> = set.iter().copied().collect();
    sv.sort_unstable();
    println!("{} squares mod 97, first {:?}", sv.len(), &sv[..8]);

    // check distribution across 16 buckets
    let mut buckets = [0u32; 16];
    for i in 0..4096u32 {
        buckets[(fx_hash(&i) >> 60) as usize] += 1;
    }
    println!("{:?}", buckets);
}
