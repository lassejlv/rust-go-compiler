use std::borrow::ToOwned;
use std::collections::HashMap;
use std::fmt::Display;

fn apply<F>(data: &[i32], f: F) -> Option<i32> where F: for<'a> Fn(&'a [i32]) -> Option<&'a i32> { f(data).copied() }
fn pick<'d, F>(data: &'d [String], f: F) -> Vec<&'d str> where F: for<'a> Fn(&'a String) -> &'a str { data.iter().map(|s| f(s)).collect() }
fn by_key<T, K: Ord, F: Fn(&T) -> K>(mut v: Vec<T>, f: F) -> Vec<T> { v.sort_by_key(|x| f(x)); v }
fn owned_copy<T: Clone>(x: &T) -> <T as ToOwned>::Owned { x.to_owned() }
fn display_len<T: Display>(t: T) -> usize { <T as ToString>::to_string(&t).len() }

struct Index<'a> { words: HashMap<&'a str, Vec<usize>> }
impl<'a> Index<'a> {
    fn build(text: &'a str) -> Self {
        let mut words: HashMap<&'a str, Vec<usize>> = HashMap::new();
        for (i, w) in text.split_whitespace().enumerate() { words.entry(w).or_default().push(i); }
        Index { words }
    }
    fn lookup<'q>(&self, q: &'q str) -> Option<&[usize]> { self.words.get(q).map(|v| v.as_slice()) }
    fn most_common(&self) -> Option<(&'a str, usize)> {
        self.words.iter().map(|(w, v)| (*w, v.len())).max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
    }
    fn filter_by<P>(&self, p: P) -> Vec<&'a str> where P: Fn(&&'a str) -> bool {
        let mut v: Vec<&'a str> = self.words.keys().copied().filter(p).collect();
        v.sort();
        v
    }
}

fn longest_common_prefix<'a>(xs: &[&'a str]) -> &'a str {
    let first = match xs.first() { Some(f) => *f, None => return "" };
    let n = xs.iter().fold(first.len(), |n, s| first.bytes().zip(s.bytes()).take(n).take_while(|(a, b)| a == b).count());
    &first[..n]
}

fn make_getter<'a, T, U: ?Sized>(f: impl Fn(&T) -> &U + 'a) -> Box<dyn for<'x> Fn(&'x T) -> &'x U + 'a> { Box::new(move |t| f(t)) }

fn main() {
    let d = [3, 9, 2];
    println!("{:?} {:?} {:?}", apply(&d, |s| s.iter().max()), apply(&d, |s| s.first()), apply(&d, <[i32]>::last));
    println!("{:?}", apply(&[], |s: &[i32]| s.get(1)));
    let names = vec!["Alice Smith".to_string(), "Bob".to_string()];
    println!("{:?} {:?}", pick(&names, |s| s.split(' ').next().unwrap()), pick(&names, String::as_str));
    println!("{:?}", by_key(vec!["ccc", "a", "bb"], |s| s.len()));
    println!("{:?}", by_key(vec![(2, 'x'), (1, 'y')], |p| p.0));
    println!("{:?} {}", owned_copy(&vec![1]), display_len(3.25));
    let text = String::from("the cat saw the dog and the cat ran");
    let idx = Index::build(&text);
    println!("{:?} {:?} {:?}", idx.lookup("cat"), idx.lookup("zebra"), idx.most_common());
    println!("{:?}", idx.filter_by(|w| w.len() == 3 && w.starts_with('t') || w.starts_with('d')));
    println!("{:?} {:?}", longest_common_prefix(&["interview", "internet", "interval"]), longest_common_prefix(&[]));
    let g = make_getter(|p: &(String, u8)| p.0.as_str());
    let g2 = make_getter::<Vec<i32>, [i32]>(|v| &v[1..]);
    let pair = ("hello".to_string(), 1);
    println!("{} {:?}", g(&pair), g2(&vec![1, 2, 3]));
}
