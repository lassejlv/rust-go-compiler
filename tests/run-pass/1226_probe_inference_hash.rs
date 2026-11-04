mod u01 {
    use std::collections::HashMap;
    fn mk() -> Vec<(String, u32)> { vec![("b".into(), 2), ("a".into(), 1)] }
    fn tally<I: IntoIterator<Item = S>, S: AsRef<str>>(it: I) -> usize { it.into_iter().map(|s| s.as_ref().len()).sum() }
    pub fn main() {
        let m: HashMap<_, _> = mk().into_iter().collect();
        let mut keys: Vec<_> = m.keys().collect();
        keys.sort();
        println!("{:?} {:?}", keys, m.get("a"));
        let mut v = Vec::new();
        v.push(1u8);
        let s = v.iter().map(|&x| x as u32).sum::<u32>();
        let x = 3i32;
        let y = x.pow(2);
        println!("{} {}", s, y);
        let mut ws = vec!["pear", "fig", "apple"];
        ws.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        println!("{:?}", ws);
        println!("{} {}", tally(vec!["ab", "c"]), tally(&["x".to_string()]));
        let parsed = "1 2 3".split(' ').map(|t| t.parse().unwrap()).collect::<Vec<u64>>();
        let total: u64 = parsed.iter().sum();
        println!("{}", total);
        let d: (i32, String, Vec<u8>) = Default::default();
        println!("{:?}", d);
        let f = |a, b| a + b;
        println!("{}", f(1.5, 2.0));
        let g = |v: &mut Vec<_>| v.push("s");
        let mut vv = vec![];
        g(&mut vv);
        println!("{:?}", vv);
        let n: i64 = (1..=20).product();
        println!("{}", n);
        let big = 1i64 << 40u64;
        println!("{}", big);
        let z = 255;
        let zz: u8 = z;
        println!("{}", zz);
        let idx = vec![10, 20, 30].iter().position(|&x| x == 20).map(|i| i + 1).unwrap_or_default();
        println!("{}", idx);
        let opt = None;
        let val: i16 = opt.unwrap_or(-1);
        println!("{}", val);
        let empty = Vec::new();
        let e2: &[u32] = &empty;
        println!("{}", e2.len());
        let r = (1..4).map(|x| x * 2).rev().collect::<std::collections::VecDeque<_>>();
        println!("{:?}", r);
        let hm: HashMap<String, Vec<i32>> = HashMap::new();
        println!("{:?}", hm.get("x").map(|v| v.len()).unwrap_or(0));
    }
}

mod u02 {
    struct Coll { items: Vec<i32> }
    impl<'a> IntoIterator for &'a Coll { type Item = &'a i32; type IntoIter = std::slice::Iter<'a, i32>; fn into_iter(self) -> Self::IntoIter { self.items.iter() } }
    impl<'a> IntoIterator for &'a mut Coll { type Item = &'a mut i32; type IntoIter = std::slice::IterMut<'a, i32>; fn into_iter(self) -> Self::IntoIter { self.items.iter_mut() } }
    impl IntoIterator for Coll { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }
    impl FromIterator<i32> for Coll { fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self { Coll { items: it.into_iter().collect() } } }
    impl Extend<i32> for Coll { fn extend<I: IntoIterator<Item = i32>>(&mut self, it: I) { for x in it { self.items.push(x) } } }
    impl std::ops::Deref for Coll { type Target = [i32]; fn deref(&self) -> &[i32] { &self.items } }
    impl std::ops::DerefMut for Coll { fn deref_mut(&mut self) -> &mut [i32] { &mut self.items } }
    pub fn main() {
        let mut c: Coll = (1..=4).collect();
        for x in &mut c { *x *= 3; }
        for x in &c { print!("{} ", x); }
        println!();
        c.extend(vec![100, 200]);
        println!("{} {:?} {:?}", c.len(), c.first(), c.iter().rev().take(2).collect::<Vec<_>>());
        c.sort_by(|a, b| b.cmp(a));
        c[0] = -1;
        println!("{:?}", &c[..3]);
        let s: i32 = c.into_iter().sum();
        println!("{}", s);
    }
}

mod u03 {
    use std::rc::Rc;
    use std::cell::RefCell;
    #[derive(Debug, Default)]
    struct Account { bal: i64, hist: Vec<i64> }
    impl Account { fn dep(&mut self, x: i64) { self.bal += x; self.hist.push(x); } fn total(&self) -> i64 { self.bal } }
    pub fn main() {
        let a = Rc::new(RefCell::new(Account::default()));
        a.borrow_mut().dep(5);
        let b = a.clone();
        b.borrow_mut().dep(10);
        println!("{} {:?}", a.borrow().total(), a.borrow().hist);
        let v: Vec<Rc<RefCell<Account>>> = (0..3).map(|_| Rc::new(RefCell::new(Account::default()))).collect();
        for (i, acc) in v.iter().enumerate() { acc.borrow_mut().dep(i as i64 * 7); }
        let sum: i64 = v.iter().map(|x| x.borrow().total()).sum();
        println!("{}", sum);
        let s = Rc::new(String::from("rc string"));
        println!("{} {}", s.len(), s.to_uppercase());
        let bx: Box<Vec<i32>> = Box::new(vec![3, 1, 2]);
        println!("{:?} {}", bx.iter().max(), bx.contains(&2));
        let nested = Rc::new(RefCell::new(vec![Rc::new(RefCell::new(1))]));
        *nested.borrow()[0].borrow_mut() += 41;
        println!("{}", nested.borrow()[0].borrow());
        let mut bb = Box::new(5);
        *bb += 1;
        let r: &mut i32 = &mut bb;
        *r *= 2;
        println!("{}", bb);
    }
}

mod u04 {
    trait Greet { fn hi(&self) -> String { "greet-hi".into() } }
    trait Wave { fn hi(&self) -> String { "wave-hi".into() } }
    struct P;
    impl Greet for P {}
    impl Wave for P {}
    trait Ext { fn ext(&self) -> String; }
    impl<T: std::fmt::Debug> Ext for T { fn ext(&self) -> String { format!("ext({:?})", self) } }
    trait ByVal { fn which(self) -> &'static str; }
    impl ByVal for &i32 { fn which(self) -> &'static str { "&i32" } }
    impl ByVal for i32 { fn which(self) -> &'static str { "i32" } }
    trait ByRef { fn kind(&self) -> &'static str; }
    impl ByRef for i32 { fn kind(&self) -> &'static str { "ref-i32" } }
    impl<T> ByRef for Vec<T> { fn kind(&self) -> &'static str { "vec" } }
    impl<T> ByRef for [T] { fn kind(&self) -> &'static str { "slice" } }
    pub fn main() {
        let p = P;
        println!("{} {}", Greet::hi(&p), <P as Wave>::hi(&p));
        println!("{} {} {}", 5.ext(), "s".ext(), vec![1].ext());
        let x = 5;
        println!("{} {} {}", x.which(), (&x).which(), (&&x).which());
        let v = vec![1];
        println!("{} {} {}", v.kind(), v[..].kind(), (&v).kind());
        println!("{}", x.kind());
        let arr = [1, 2];
        println!("{}", arr.kind());
    }
}

mod u05 {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    #[derive(Debug, Clone, Eq)]
    struct Caseless(String);
    impl PartialEq for Caseless { fn eq(&self, o: &Self) -> bool { self.0.eq_ignore_ascii_case(&o.0) } }
    impl Hash for Caseless { fn hash<H: Hasher>(&self, h: &mut H) { self.0.to_ascii_lowercase().hash(h) } }
    pub fn main() {
        let mut m = HashMap::new();
        m.insert(Caseless("Key".into()), 1);
        *m.entry(Caseless("KEY".into())).or_insert(0) += 10;
        m.insert(Caseless("other".into()), 2);
        println!("{} {:?}", m.len(), m.get(&Caseless("key".into())));
        let mut s = std::collections::HashSet::new();
        println!("{} {} {}", s.insert("a"), s.insert("a"), s.len());
        let h1 = { let mut h = std::collections::hash_map::DefaultHasher::new(); 42u32.hash(&mut h); h.finish() };
        let h2 = { let mut h = std::collections::hash_map::DefaultHasher::new(); 42u32.hash(&mut h); h.finish() };
        println!("{}", h1 == h2);
        let mut hm: HashMap<String, i32> = HashMap::new();
        hm.insert("x".to_string(), 1);
        println!("{:?} {:?}", hm.get("x"), hm.get_key_value("x"));
        if let Some(v) = hm.get_mut("x") { *v += 1; }
        println!("{:?} {:?}", hm.remove_entry("x"), hm.is_empty());
        let mut counts: HashMap<char, usize> = HashMap::with_capacity(8);
        "hello".chars().for_each(|c| *counts.entry(c).or_default() += 1);
        let mut cv: Vec<_> = counts.iter().collect();
        cv.sort();
        println!("{:?}", cv);
        counts.retain(|_, v| *v > 1);
        println!("{:?}", counts);
        let merged: HashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().chain([("a", 5)]).collect();
        let mut mv: Vec<_> = merged.into_iter().collect();
        mv.sort();
        println!("{:?}", mv);
        let hs1: std::collections::HashSet<i32> = (1..5).collect();
        let hs2: std::collections::HashSet<i32> = (3..8).collect();
        let mut i: Vec<_> = hs1.intersection(&hs2).copied().collect(); i.sort();
        let mut d: Vec<_> = (&hs1 - &hs2).into_iter().collect(); d.sort();
        println!("{:?} {:?}", i, d);
        let mut dr: Vec<_> = hm.drain().collect();
        dr.sort();
        println!("{:?}", dr);
    }
}

fn main() {
    println!("== u01");
    u01::main();
    println!("== u02");
    u02::main();
    println!("== u03");
    u03::main();
    println!("== u04");
    u04::main();
    println!("== u05");
    u05::main();
}
