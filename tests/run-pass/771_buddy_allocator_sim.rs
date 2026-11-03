// Buddy allocator simulation over a 1 KiB arena: power-of-two free lists,
// splitting and coalescing, allocation handles with generation counters,
// a typed view writing u32s into the byte arena, and fragmentation stats.
use std::collections::BTreeSet;
use std::fmt;

const MIN_ORDER: u32 = 4; // 16 bytes
const MAX_ORDER: u32 = 10; // 1024 bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle { offset: usize, order: u32, gen: u32 }

#[derive(Debug, PartialEq)]
enum AllocError { TooLarge(usize), OutOfMemory(usize), DoubleFree(usize), Stale { offset: usize, gen: u32 } }

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AllocError::TooLarge(n) => write!(f, "request of {} bytes exceeds arena", n),
            AllocError::OutOfMemory(n) => write!(f, "no block for {} bytes", n),
            AllocError::DoubleFree(o) => write!(f, "double free at {:#x}", o),
            AllocError::Stale { offset, gen } => write!(f, "stale handle {:#x} gen {}", offset, gen),
        }
    }
}

struct Buddy {
    mem: Vec<u8>,
    free: Vec<BTreeSet<usize>>, // index = order
    live: Vec<Option<(u32, u32)>>, // per 16-byte slot: (order, gen) at block start
    gens: Vec<u32>,
}

impl Buddy {
    fn new() -> Self {
        let mut free = vec![BTreeSet::new(); (MAX_ORDER + 1) as usize];
        free[MAX_ORDER as usize].insert(0);
        let slots = 1 << (MAX_ORDER - MIN_ORDER);
        Buddy { mem: vec![0; 1 << MAX_ORDER], free, live: vec![None; slots], gens: vec![0; slots] }
    }
    fn order_for(n: usize) -> u32 { (n.max(1).next_power_of_two().trailing_zeros()).max(MIN_ORDER) }
    fn alloc(&mut self, n: usize) -> Result<Handle, AllocError> {
        let want = Self::order_for(n);
        if want > MAX_ORDER { return Err(AllocError::TooLarge(n)); }
        let mut o = want;
        while o <= MAX_ORDER && self.free[o as usize].is_empty() { o += 1; }
        if o > MAX_ORDER { return Err(AllocError::OutOfMemory(n)); }
        let off = self.free[o as usize].pop_first().unwrap();
        while o > want {
            o -= 1;
            self.free[o as usize].insert(off + (1 << o));
        }
        let slot = off >> MIN_ORDER;
        self.gens[slot] += 1;
        self.live[slot] = Some((want, self.gens[slot]));
        Ok(Handle { offset: off, order: want, gen: self.gens[slot] })
    }
    fn free(&mut self, h: Handle) -> Result<u32, AllocError> {
        let slot = h.offset >> MIN_ORDER;
        match self.live[slot] {
            None => return Err(AllocError::DoubleFree(h.offset)),
            Some((_, g)) if g != h.gen => return Err(AllocError::Stale { offset: h.offset, gen: h.gen }),
            _ => {}
        }
        self.live[slot] = None;
        let (mut off, mut o) = (h.offset, h.order);
        let mut merges = 0;
        while o < MAX_ORDER {
            let buddy = off ^ (1 << o);
            if !self.free[o as usize].remove(&buddy) { break; }
            off = off.min(buddy);
            o += 1;
            merges += 1;
        }
        self.free[o as usize].insert(off);
        Ok(merges)
    }
    fn write_u32s(&mut self, h: Handle, vals: &[u32]) {
        let cap = (1usize << h.order) / 4;
        for (i, v) in vals.iter().take(cap).enumerate() {
            self.mem[h.offset + i * 4..h.offset + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
    fn read_u32s(&self, h: Handle, n: usize) -> Vec<u32> {
        (0..n).map(|i| { let p = h.offset + i * 4; u32::from_le_bytes(self.mem[p..p + 4].try_into().unwrap()) }).collect()
    }
    fn free_map(&self) -> String {
        (MIN_ORDER..=MAX_ORDER).filter(|o| !self.free[*o as usize].is_empty())
            .map(|o| format!("{}:{:?}", 1 << o, self.free[o as usize].iter().map(|x| format!("{:#x}", x)).collect::<Vec<_>>()))
            .collect::<Vec<_>>().join(" ")
    }
    fn free_bytes(&self) -> usize { (0..=MAX_ORDER).map(|o| self.free[o as usize].len() << o).sum() }
    fn largest_free(&self) -> usize { (0..=MAX_ORDER).rev().find(|o| !self.free[*o as usize].is_empty()).map_or(0, |o| 1 << o) }
}

fn main() {
    let mut b = Buddy::new();
    println!("init: {}", b.free_map());
    let sizes = [100, 16, 1, 200, 64, 33, 500];
    let mut hs = vec![];
    for s in sizes {
        match b.alloc(s) {
            Ok(h) => { println!("alloc {:>3} -> off {:#05x} order {} | {}", s, h.offset, h.order, b.free_map()); hs.push(h); }
            Err(e) => println!("alloc {:>3} failed: {}", s, e),
        }
    }
    println!("free bytes {} largest {}", b.free_bytes(), b.largest_free());
    b.write_u32s(hs[0], &[0xDEADBEEF, 1, 2, 3]);
    b.write_u32s(hs[1], &[7, 8, 9, 10, 11]);
    println!("read back {:x?} {:?}", b.read_u32s(hs[0], 4), b.read_u32s(hs[1], 4));
    for i in [1, 2, 4] {
        let m = b.free(hs[i]);
        println!("free #{} ({:#x}) merges {:?} | {}", i, hs[i].offset, m, b.free_map());
    }
    println!("double free: {}", b.free(hs[1]).unwrap_err());
    let again = b.alloc(16).unwrap();
    println!("realloc same slot? {} gen {}", again.offset == hs[1].offset, again.gen);
    println!("stale: {}", b.free(hs[1]).map_err(|e| e.to_string()).unwrap_err());
    println!("too large: {:?}", b.alloc(2000));
    for h in [hs[0], hs[3], hs[5], again] {
        println!("free {:#x} -> {:?}", h.offset, b.free(h));
    }
    println!("end: {} free bytes {}", b.free_map(), b.free_bytes());
    let mut all = vec![];
    while let Ok(h) = b.alloc(16) { all.push(h); }
    println!("filled with {} 16-byte blocks; next: {:?}", all.len(), b.alloc(1).map_err(|e| e.to_string()));
    let total_merges: u32 = all.iter().rev().map(|h| b.free(*h).unwrap()).sum();
    println!("freed all, merges {} -> {}", total_merges, b.free_map());
}
