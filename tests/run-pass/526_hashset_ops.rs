// HashSet operations with derived Hash on structs/enums, set algebra, sorted output
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Tag {
    Red,
    Green,
    Custom(String),
    Pair(u8, u8),
}

#[derive(Debug, Clone, Eq)]
struct Caseless(String);
impl PartialEq for Caseless {
    fn eq(&self, o: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&o.0)
    }
}
impl Hash for Caseless {
    fn hash<H: Hasher>(&self, h: &mut H) {
        for b in self.0.bytes() {
            h.write_u8(b.to_ascii_lowercase());
        }
    }
}

fn sorted<T: Ord + Clone>(s: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = s.into_iter().collect();
    v.sort();
    v
}

fn main() {
    let a: HashSet<i32> = (1..=10).collect();
    let b: HashSet<i32> = (0..=20).step_by(3).collect();
    println!("i {:?}", sorted(a.intersection(&b).copied()));
    println!("d {:?}", sorted(a.difference(&b).copied()));
    println!("d2 {:?}", sorted(b.difference(&a).copied()));
    println!("s {:?}", sorted(a.symmetric_difference(&b).copied()));
    println!("u {:?}", sorted(a.union(&b).copied()));
    println!("{:?}", sorted(&a & &b));
    println!("{:?}", sorted(&a | &b).len());
    println!("{:?}", sorted(&a ^ &b));
    println!("{:?}", sorted(&a - &b));
    let small: HashSet<i32> = [3, 6, 9].into();
    println!("{} {} {}", small.is_subset(&a), small.is_subset(&b), a.is_superset(&small));
    println!("{} {}", small.is_disjoint(&[1, 2].into()), a.is_disjoint(&b));

    let mut tags: HashSet<Tag> = HashSet::new();
    println!("{}", tags.insert(Tag::Red));
    println!("{}", tags.insert(Tag::Custom("x".into())));
    println!("{}", tags.insert(Tag::Red));
    println!("{}", tags.insert(Tag::Pair(1, 2)));
    println!("{}", tags.insert(Tag::Pair(2, 1)));
    println!("{}", tags.insert(Tag::Custom("x".to_string())));
    tags.insert(Tag::Green);
    println!("{:?}", sorted(tags.iter().cloned()));
    println!("{}", tags.contains(&Tag::Pair(2, 1)));
    println!("{:?}", tags.get(&Tag::Custom("x".into())));
    println!("{:?} {:?}", tags.take(&Tag::Green), tags.take(&Tag::Green));
    println!("{}", tags.remove(&Tag::Red));
    println!("{:?}", tags.replace(Tag::Pair(1, 2)));
    println!("{}", tags.len());

    let mut names: HashSet<Caseless> = HashSet::new();
    for n in ["Alice", "bob", "ALICE", "Bob", "carol", "aLiCe"] {
        let fresh = names.insert(Caseless(n.to_string()));
        println!("{} -> {}", n, fresh);
    }
    println!("{}", names.contains(&Caseless("CAROL".into())));
    let mut uniq: Vec<String> = names.into_iter().map(|c| c.0.to_lowercase()).collect();
    uniq.sort();
    println!("{:?}", uniq);

    let text1 = "the quick brown fox jumps over the lazy dog";
    let text2 = "the lazy cat sleeps over the warm mat";
    let w1: HashSet<&str> = text1.split(' ').collect();
    let w2: HashSet<&str> = text2.split(' ').collect();
    let common: BTreeSet<&str> = w1.intersection(&w2).copied().collect();
    let only1: BTreeSet<_> = w1.difference(&w2).collect();
    println!("{:?}", common);
    println!("{:?}", only1);
    let jaccard = common.len() as f64 / w1.union(&w2).count() as f64;
    println!("{:.4}", jaccard);
    let mut seen = HashSet::new();
    let firsts: Vec<char> = "programming".chars().filter(|c| seen.insert(*c)).collect();
    println!("{:?}", firsts);
    let dup = [1, 5, 2, 5, 3, 1].iter().find(|x| !seen.insert(x.to_string().chars().next().unwrap()));
    println!("{:?}", dup);
    let mut hs: HashSet<(i32, i32)> = HashSet::new();
    let (mut x, mut y) = (0, 0);
    hs.insert((x, y));
    for m in "NNESSWWWNE".chars() {
        match m {
            'N' => y += 1,
            'S' => y -= 1,
            'E' => x += 1,
            _ => x -= 1,
        }
        hs.insert((x, y));
    }
    println!("visited {} {:?}", hs.len(), sorted(hs.iter().filter(|p| p.0 < 0).copied()));
    let mut ret: HashSet<i32> = (1..20).collect();
    ret.retain(|x| x % 4 == 1);
    let ex: Vec<i32> = sorted(ret.extract_if(|x| *x > 10));
    println!("{:?} {:?}", ex, sorted(ret.drain()));
}
