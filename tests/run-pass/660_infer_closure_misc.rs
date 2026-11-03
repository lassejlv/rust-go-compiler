// closure-adapter chains (canonical query caching), closure inference
// through blocks, unsize coercion of generic call results, impl-Trait item
// types, literal keys for map indexing, Clone for closures, and a few std
// details (Vec::clone_from, Flatten::size_hint, RangeInclusive/CharIndices
// Debug)
use std::collections::{BTreeMap, HashMap};

struct G<T> {
    data: Vec<T>,
}
impl<T: Default + Clone> G<T> {
    fn new(n: usize) -> G<T> {
        G { data: vec![T::default(); n] }
    }
    fn items_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }
}

fn id<T>(t: T) -> T {
    t
}
fn total(v: &[u8]) -> u32 {
    v.iter().map(|&x| x as u32).sum()
}

struct Noisy(u32);
impl Clone for Noisy {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Noisy(self.0)
    }
    fn clone_from(&mut self, src: &Self) {
        println!("clone_from {} <- {}", self.0, src.0);
        self.0 = src.0;
    }
}
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    // long chains of closure adapters
    let a = [1, 2, 3, 4, 5];
    let s: i32 = a
        .iter()
        .copied()
        .map(|x| x + 1)
        .filter(|x| *x > 0)
        .map(|x| x + 1)
        .skip_while(|x| *x < 0)
        .map(|x| x + 1)
        .filter(|x| *x > 0)
        .map(|x| x + 1)
        .take_while(|x| *x < 100)
        .map(|x| x + 1)
        .inspect(|_| {})
        .map(|x| x + 1)
        .sum();
    println!("{}", s);
    let v: Vec<String> = a
        .iter()
        .map(|x| x * 2)
        .map(|x| x.to_string())
        .filter(|s| !s.is_empty())
        .map(|s| s + "!")
        .map(|s| s.len())
        .map(|n| format!("<{}>", n))
        .collect();
    println!("{:?}", v);

    // closure parameter types from the expected type through a block
    let f: Box<dyn Fn(&mut Vec<i32>)> = Box::new({
        let k = 5;
        move |l| l.push(k)
    });
    let mut out = vec![];
    f(&mut out);
    let g: Box<dyn Fn(&str) -> usize> = Box::new({
        let _u = 0;
        { |s| s.len() }
    });
    println!("{:?} {}", out, g("abcd"));

    // `&generic(array)` coerces to a slice afterwards
    println!("{} {}", total(&id([10u8, 20, 30, 40])), total(&std::hint::black_box([1u8, 2])));

    // impl Iterator<Item = &mut T> with T still being inferred
    let mut gr = G::new(3);
    for v in gr.items_mut() {
        *v = 7u64;
    }
    println!("{:?}", gr.data);

    // integer literal keys index maps with non-i32 keys
    let mut hm: HashMap<u32, String> = HashMap::new();
    hm.insert(1, String::from("one"));
    let mut bm: BTreeMap<u64, &str> = BTreeMap::new();
    bm.insert(2, "two");
    println!("{} {}", hm[&1], bm[&2]);

    // closures are Clone when their captures are
    let names = vec![String::from("a"), String::from("b")];
    let h = move || names.len();
    let h2 = h.clone();
    println!("{} {}", h(), h2());
    let n = Noisy(3);
    let c = move || {
        let m = &n;
        m.0 + 1
    };
    let c2 = c.clone();
    println!("{} {}", c(), c2());
    let k = 5;
    let add = move |x: i32| x + k;
    let add2 = add.clone();
    let r = &k;
    let by_ref = || *r + 1;
    let by_ref2 = by_ref.clone();
    println!("{} {} {}", add2(1), by_ref(), by_ref2());
    drop(c);
    drop(c2);

    // Vec::clone_from reuses the shared prefix
    let src = vec![Noisy(1), Noisy(2)];
    let mut dst = vec![Noisy(9)];
    dst.clone_from(&src);
    let mut dst2 = vec![Noisy(7), Noisy(8), Noisy(6)];
    dst2.clone_from(&src);
    println!("{}", dst2.len());
    drop(dst);
    drop(dst2);
    drop(src);

    // Flatten / FlatMap size hints
    let nested = [[1, 2], [3, 4]];
    let mut fl = nested.iter().flatten();
    println!("{:?}", fl.size_hint());
    fl.next();
    println!("{:?}", fl.size_hint());
    let opts = vec![Some(1), None, Some(3)];
    println!("{:?}", opts.iter().flatten().size_hint());
    let vv = vec![vec![1, 2], vec![3]];
    let mut fv = vv.iter().flatten();
    println!("{:?}", fv.size_hint());
    fv.next();
    fv.next();
    println!("{:?}", fv.size_hint());
    let mut fm = (0..3).flat_map(|x| [x, x]);
    fm.next();
    println!("{:?}", fm.size_hint());

    // RangeInclusive after iteration, CharIndices
    let mut ri = 1..=3;
    for _ in ri.by_ref() {}
    println!("{:?} {}", ri, ri.is_empty());
    let mut rb = 5u8..=7;
    rb.next_back();
    println!("{:?}", rb);
    let mut rw = 255u8..=255;
    rw.next();
    println!("{:?} {}", rw, rw.is_empty());
    let mut rn = 1..=5;
    println!("{:?}", rn.nth(7));
    println!("{:?}", rn);
    let mut ci = "héllo".char_indices();
    ci.next();
    ci.next();
    println!("{:?}", ci);
}
