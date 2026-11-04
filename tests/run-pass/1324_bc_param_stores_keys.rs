// Valid stores into `&mut` parameters (loans of what outlives the call,
// promoted constants, reborrows through references), lookups with
// `&&str` keys (K: Borrow<Q> with K = Q), and type aliases with lifetimes
// in signatures.
use std::collections::{BTreeMap, HashMap, HashSet};

const X: i32 = 5;
static Y: i32 = 6;

struct S<'a> {
    items: Vec<&'a str>,
    nums: Vec<&'a i32>,
}

impl<'a> S<'a> {
    fn add(&mut self, x: &'a str) {
        let t = x;
        self.items.push(t);
    }
    fn add_nums(&mut self) {
        self.nums.push(&X);
        self.nums.push(&7);
        self.nums.push(&Y);
    }
}

fn from_slice<'a>(v: &mut Vec<&'a str>, src: &'a [String]) {
    for s in src.iter() {
        v.push(s.as_str());
    }
    let first = &src[0];
    v.push(first);
}

fn split_into<'a>(v: &mut Vec<&'a str>, s: &'a str) {
    let parts: Vec<&str> = s.split(',').collect();
    for p in &parts {
        v.push(p);
    }
}

fn set_opt<'a>(o: &mut Option<&'a str>, s: &'a str) {
    let local = s;
    o.replace(local);
}

fn swap_local(a: &mut Vec<i32>) {
    let mut b = vec![9];
    std::mem::swap(a, &mut b);
    println!("swapped out {:?}", b);
}

fn count<'a>(m: &mut HashMap<&'a str, usize>, words: &[&'a str]) {
    for w in words {
        *m.entry(w).or_insert(0) += 1;
    }
    for w in words {
        if let Some(c) = m.get_mut(w) {
            *c += 10;
        }
    }
}

fn lookup(m: &HashMap<&str, i32>) -> Option<i32> {
    let s = String::from("a");
    let r = m.get(&s.as_str()).copied();
    let present = m.contains_key(&s.as_str());
    println!("present {}", present);
    r
}

fn remove_keys<'a>(m: &mut BTreeMap<&'a str, i32>, keys: &[&'a str]) {
    for k in keys {
        m.remove(k);
    }
}

fn set_contains(set: &HashSet<&str>) -> bool {
    let s = String::from("x");
    set.contains(&s.as_str())
}

type Pair<'a> = (&'static str, &'a str);
type Row<'a> = Vec<Pair<'a>>;

fn first_static<'a>(p: Pair<'a>) -> &'static str {
    p.0
}

fn row_keys<'a>(r: &Row<'a>) -> Vec<&'static str> {
    r.iter().map(|p| p.0).collect()
}

fn main() {
    let mut s = S { items: vec![], nums: vec![] };
    s.add("x");
    s.add_nums();
    println!("{:?} {:?}", s.items, s.nums);

    let src = vec![String::from("p"), String::from("q")];
    let mut v = Vec::new();
    from_slice(&mut v, &src);
    split_into(&mut v, "a,b");
    println!("{:?}", v);

    let mut o = None;
    set_opt(&mut o, "z");
    println!("{:?}", o);

    let mut a = vec![1];
    swap_local(&mut a);
    println!("{:?}", a);

    let mut m = HashMap::new();
    count(&mut m, &["a", "b", "a"]);
    let mut c: Vec<_> = m.iter().collect();
    c.sort();
    println!("{:?}", c);

    let mut hm = HashMap::new();
    hm.insert("a", 1);
    println!("{:?}", lookup(&hm));

    let mut bm = BTreeMap::new();
    bm.insert("k", 1);
    bm.insert("l", 2);
    remove_keys(&mut bm, &["k"]);
    println!("{:?}", bm);

    let mut set = HashSet::new();
    set.insert("x");
    println!("{}", set_contains(&set));

    let owned = String::from("tmp");
    let row: Row = vec![("r1", owned.as_str()), ("r2", "")];
    let k = row_keys(&row);
    let f = first_static(row[0]);
    drop(row);
    drop(owned);
    println!("{:?} {}", k, f);
}
