// sort_unstable* and select_nth_unstable* use std's algorithms (ipnsort /
// introselect, with std's small-sort choice per type), so the order of equal
// elements and the number of comparisons match rustc.
use std::cell::Cell;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn h(acc: u64, v: u64) -> u64 {
    (acc ^ v).wrapping_mul(0x100000001b3)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NoCopy(u32);

#[derive(Clone, Copy)]
struct Big {
    k: u32,
    pad: [u8; 100],
}

#[derive(Debug, Clone)]
struct Person {
    name: &'static str,
    age: u32,
}

fn main() {
    // the classic: equal keys keep a std-specific order
    let mut people: Vec<Person> = [
        ("ann", 31), ("bob", 25), ("cat", 31), ("dan", 40), ("eve", 25), ("fay", 31), ("gus", 25), ("hal", 40),
        ("ivy", 31), ("jon", 25), ("kim", 40), ("lea", 31), ("max", 25), ("ned", 31), ("oli", 40), ("pam", 25),
        ("quin", 31), ("rex", 40), ("sue", 25), ("tom", 31), ("uma", 40), ("vic", 25), ("wes", 31), ("xia", 40),
    ]
    .iter()
    .map(|&(name, age)| Person { name, age })
    .collect();
    people.sort_unstable_by_key(|p| p.age);
    println!("{:?}", people.iter().map(|p| p.name).collect::<Vec<_>>());
    let mut words: Vec<&str> = "the quick brown fox jumps over the lazy dog and runs far away from the big bad wolf in the dark forest at night".split(' ').collect();
    words.sort_unstable_by_key(|w| w.len());
    println!("{:?}", words);
    let mut nums: Vec<i32> = (0..40).map(|i| (i * 37) % 23 - 11).collect();
    let (lo, mid, hi) = nums.select_nth_unstable(20);
    println!("{:?} {} {:?}", lo, mid, hi);

    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut acc = [0u64; 7];
    let mut cmps = [0u64; 3];
    for round in 0..60 {
        let len = (rng.next() % 200) as usize;
        let modk = 1 + rng.next() % (if round % 3 == 0 { 5 } else { 1000 });
        let mut base: Vec<(u32, u32)> = (0..len).map(|i| ((rng.next() % modk) as u32, i as u32)).collect();
        if round % 5 == 1 {
            base.sort_by_key(|p| p.0);
        }
        if round % 5 == 2 {
            base.sort_by_key(|p| std::cmp::Reverse(p.0));
        }

        let mut v = base.clone();
        let mut n = 0u64;
        v.sort_unstable_by(|a, b| {
            n += 1;
            a.0.cmp(&b.0)
        });
        cmps[0] += n;
        for p in &v {
            acc[0] = h(acc[0], p.1 as u64);
        }

        let mut v: Vec<(u64, u64)> = base.iter().map(|p| (p.0 as u64, p.1 as u64)).collect();
        v.sort_unstable_by_key(|p| p.0);
        for p in &v {
            acc[1] = h(acc[1], p.1);
        }

        let mut v: Vec<(NoCopy, u32)> = base.iter().map(|p| (NoCopy(p.0), p.1)).collect();
        v.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for p in &v {
            acc[2] = h(acc[2], p.1 as u64);
        }

        let mut v: Vec<Big> = base.iter().map(|p| Big { k: p.0, pad: [(p.1 % 256) as u8; 100] }).collect();
        let mut n = 0u64;
        v.sort_unstable_by(|a, b| {
            n += 1;
            a.k.cmp(&b.k)
        });
        cmps[1] += n;
        for p in &v {
            acc[3] = h(acc[3], p.pad[0] as u64);
        }

        let mut v: Vec<(String, u32)> = base.iter().map(|p| (format!("k{}", p.0), p.1)).collect();
        v.sort_unstable();
        v.sort_unstable_by(|a, b| a.0.len().cmp(&b.0.len()));
        for p in &v {
            acc[4] = h(acc[4], p.1 as u64);
        }

        let mut v: Vec<(Cell<u32>, u32)> = base.iter().map(|p| (Cell::new(p.0), p.1)).collect();
        v.sort_unstable_by(|a, b| a.0.get().cmp(&b.0.get()));
        for p in &v {
            acc[5] = h(acc[5], p.1 as u64);
        }

        if len > 0 {
            let idx = (rng.next() % len as u64) as usize;
            let mut v = base.clone();
            let mut n = 0u64;
            v.select_nth_unstable_by(idx, |a, b| {
                n += 1;
                a.0.cmp(&b.0)
            });
            cmps[2] += n;
            for p in &v {
                acc[6] = h(acc[6], p.1 as u64);
            }
            let mut w: Vec<NoCopy> = base.iter().map(|p| NoCopy(p.0 * 1000 + p.1)).collect();
            w.select_nth_unstable_by_key(idx, |x| x.0 / 3000);
            for p in &w {
                acc[6] = h(acc[6], p.0 as u64);
            }
        }
    }
    println!("{:?}", acc);
    println!("{:?}", cmps);
}
