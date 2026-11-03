// A closure called through a type parameter bound `F: FnMut(&X) -> K`:
// the argument's reference is higher-ranked, so the result K (a type of
// the caller) cannot hold its borrow.

use std::collections::HashMap;

struct Keyed<K, F> {
    f: F,
    seen: Vec<K>,
}

impl<K: PartialEq + Clone, F: FnMut(&String) -> K> Keyed<K, F> {
    fn add(&mut self, s: String) -> K {
        let k = (self.f)(&s);
        drop(s);
        if !self.seen.contains(&k) {
            self.seen.push(k.clone());
        }
        k
    }
}

fn first_key<'a, I, K, F>(items: I, mut f: F) -> Option<K>
where
    I: Iterator<Item = &'a String>,
    F: for<'r> FnMut(&'r &'a String) -> K,
{
    let mut out = None;
    for it in items {
        let k = f(&it);
        if out.is_none() {
            out = Some(k);
        }
    }
    out
}

fn group<T, K, F>(v: Vec<T>, key: F) -> HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq,
    F: Fn(&T) -> K,
{
    let mut m: HashMap<K, Vec<T>> = HashMap::new();
    for x in v {
        let k = key(&x);
        m.entry(k).or_default().push(x);
    }
    m
}

fn call_once<F: FnOnce(&Vec<u8>) -> &'static str>(f: F) -> &'static str {
    let r;
    {
        let v = vec![1, 2, 3];
        r = f(&v);
    }
    r
}

fn main() {
    let mut k = Keyed { f: |s: &String| if s.len() > 2 { "long" } else { "short" }, seen: Vec::new() };
    println!("{} {}", k.add("abc".to_string()), k.add("de".to_string()));
    println!("{:?}", k.seen);

    let names = vec!["x".to_string(), "yy".to_string()];
    let k = first_key(names.iter(), |s| s.as_str());
    println!("{:?}", k);

    let m = group(vec!["a1", "b2", "a3"], |x| &x[..1]);
    let mut ks: Vec<_> = m.iter().map(|(k, v)| (*k, v.len())).collect();
    ks.sort();
    println!("{:?}", ks);

    println!("{}", call_once(|v| if v.is_empty() { "empty" } else { "full" }));
}
