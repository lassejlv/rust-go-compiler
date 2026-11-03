// A tiny LSM-style key/value store: memtable (BTreeMap), write-ahead log
// encoded in a Vec<u8>, immutable sorted runs, compaction and recovery.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Entry {
    Put(Vec<u8>),
    Tombstone,
}

#[derive(Debug)]
enum WalError {
    Truncated(usize),
    BadTag(u8, usize),
    BadChecksum { at: usize, want: u32, got: u32 },
}

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WalError::Truncated(at) => write!(f, "truncated record at {}", at),
            WalError::BadTag(t, at) => write!(f, "bad tag {:#04x} at {}", t, at),
            WalError::BadChecksum { at, want, got } => {
                write!(f, "checksum mismatch at {}: want {:08x} got {:08x}", at, want, got)
            }
        }
    }
}

fn checksum(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn get_u32(buf: &[u8], at: usize) -> Option<u32> {
    let s = buf.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

struct Wal {
    buf: Vec<u8>,
}

impl Wal {
    fn new() -> Self {
        Wal { buf: Vec::new() }
    }
    fn append(&mut self, key: &[u8], e: &Entry) {
        let mut rec = Vec::new();
        match e {
            Entry::Put(v) => {
                rec.push(1u8);
                put_u32(&mut rec, key.len() as u32);
                rec.extend_from_slice(key);
                put_u32(&mut rec, v.len() as u32);
                rec.extend_from_slice(v);
            }
            Entry::Tombstone => {
                rec.push(2u8);
                put_u32(&mut rec, key.len() as u32);
                rec.extend_from_slice(key);
            }
        }
        let c = checksum(&rec);
        self.buf.extend_from_slice(&rec);
        put_u32(&mut self.buf, c);
    }
    fn replay(buf: &[u8]) -> Result<Vec<(Vec<u8>, Entry)>, WalError> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < buf.len() {
            let start = i;
            let tag = buf[i];
            i += 1;
            let klen = get_u32(buf, i).ok_or(WalError::Truncated(start))? as usize;
            i += 4;
            let key = buf.get(i..i + klen).ok_or(WalError::Truncated(start))?.to_vec();
            i += klen;
            let entry = match tag {
                1 => {
                    let vlen = get_u32(buf, i).ok_or(WalError::Truncated(start))? as usize;
                    i += 4;
                    let v = buf.get(i..i + vlen).ok_or(WalError::Truncated(start))?.to_vec();
                    i += vlen;
                    Entry::Put(v)
                }
                2 => Entry::Tombstone,
                t => return Err(WalError::BadTag(t, start)),
            };
            let want = get_u32(buf, i).ok_or(WalError::Truncated(start))?;
            let got = checksum(&buf[start..i]);
            if want != got {
                return Err(WalError::BadChecksum { at: start, want, got });
            }
            i += 4;
            out.push((key, entry));
        }
        Ok(out)
    }
}

struct Store {
    mem: BTreeMap<Vec<u8>, Entry>,
    runs: Vec<Vec<(Vec<u8>, Entry)>>,
    wal: Wal,
    limit: usize,
}

impl Store {
    fn new(limit: usize) -> Self {
        Store { mem: BTreeMap::new(), runs: Vec::new(), wal: Wal::new(), limit }
    }
    fn put(&mut self, k: &str, v: &str) {
        let e = Entry::Put(v.as_bytes().to_vec());
        self.wal.append(k.as_bytes(), &e);
        self.mem.insert(k.as_bytes().to_vec(), e);
        self.maybe_flush();
    }
    fn delete(&mut self, k: &str) {
        self.wal.append(k.as_bytes(), &Entry::Tombstone);
        self.mem.insert(k.as_bytes().to_vec(), Entry::Tombstone);
        self.maybe_flush();
    }
    fn maybe_flush(&mut self) {
        if self.mem.len() >= self.limit {
            let run: Vec<_> = std::mem::take(&mut self.mem).into_iter().collect();
            println!("  flush run #{} with {} entries", self.runs.len(), run.len());
            self.runs.push(run);
            self.wal = Wal::new();
        }
    }
    fn get(&self, k: &str) -> Option<String> {
        let key = k.as_bytes();
        let found = self.mem.get(key).cloned().or_else(|| {
            self.runs.iter().rev().find_map(|run| {
                run.binary_search_by(|(rk, _)| rk.as_slice().cmp(key)).ok().map(|i| run[i].1.clone())
            })
        });
        match found {
            Some(Entry::Put(v)) => Some(String::from_utf8(v).unwrap()),
            _ => None,
        }
    }
    fn compact(&mut self) {
        let mut merged: BTreeMap<Vec<u8>, Entry> = BTreeMap::new();
        for run in self.runs.drain(..) {
            for (k, e) in run {
                merged.insert(k, e);
            }
        }
        merged.retain(|_, e| *e != Entry::Tombstone);
        let run: Vec<_> = merged.into_iter().collect();
        println!("  compacted into one run of {}", run.len());
        self.runs.push(run);
    }
    fn scan(&self, from: &str, to: &str) -> Vec<(String, String)> {
        let mut view: BTreeMap<Vec<u8>, Entry> = BTreeMap::new();
        for run in &self.runs {
            for (k, e) in run {
                view.insert(k.clone(), e.clone());
            }
        }
        for (k, e) in &self.mem {
            view.insert(k.clone(), e.clone());
        }
        view.range(from.as_bytes().to_vec()..to.as_bytes().to_vec())
            .filter_map(|(k, e)| match e {
                Entry::Put(v) => Some((String::from_utf8_lossy(k).into_owned(), String::from_utf8_lossy(v).into_owned())),
                Entry::Tombstone => None,
            })
            .collect()
    }
}

fn main() {
    let mut s = Store::new(4);
    let fruits = ["apple", "banana", "cherry", "date", "elder", "fig", "grape", "honeydew", "kiwi", "lemon"];
    for (i, f) in fruits.iter().enumerate() {
        s.put(f, &format!("v{}", i * i));
    }
    s.delete("cherry");
    s.put("banana", "yellow");
    s.delete("kiwi");
    for k in ["apple", "banana", "cherry", "kiwi", "lemon", "zzz"] {
        println!("get {:>8} -> {:?}", k, s.get(k));
    }
    println!("scan b..h: {:?}", s.scan("b", "h"));
    println!("runs before compact: {:?}", s.runs.iter().map(|r| r.len()).collect::<Vec<_>>());
    s.compact();
    println!("scan all: {}", s.scan("", "~").iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(","));

    // WAL encoding/recovery
    let mut w = Wal::new();
    w.append(b"x", &Entry::Put(b"1".to_vec()));
    w.append(b"yy", &Entry::Tombstone);
    w.append(b"zzz", &Entry::Put(vec![0xff, 0x00]));
    println!("wal bytes = {}", w.buf.len());
    match Wal::replay(&w.buf) {
        Ok(recs) => {
            for (k, e) in &recs {
                println!("  rec {:?} {:?}", String::from_utf8_lossy(k), e);
            }
        }
        Err(e) => println!("  error: {}", e),
    }
    let mut corrupt = w.buf.clone();
    corrupt[3] ^= 0x40;
    match Wal::replay(&corrupt) {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("corrupt: {}", e),
    }
    let truncated = &w.buf[..w.buf.len() - 2];
    match Wal::replay(truncated) {
        Ok(_) => println!("unexpected ok"),
        Err(e) => println!("truncated: {}", e),
    }
    let mut badtag = w.buf.clone();
    badtag[0] = 9;
    if let Err(e) = Wal::replay(&badtag) {
        println!("badtag: {} / {:?}", e, e);
    }
}
