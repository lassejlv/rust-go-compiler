// Generic library-style code: HRTB bounds over borrowed collections
// (`for<'a> &'a C: IntoIterator<Item = &'a T>`), AsRef/Into/Borrow
// parameters, error type erasure with Box<dyn Error + Send + Sync> and
// downcasting, a sorted-vec set with From/FromIterator/Extend/IntoIterator
// impls, key-extractor generics, iterator returning `impl Iterator + 'a`,
// by_ref / peekable parsing, and const-generic defaults.
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{self, Display};
use std::hash::Hash;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;

fn total<C, T>(c: &C) -> T
where
    for<'a> &'a C: IntoIterator<Item = &'a T>,
    T: Copy + std::iter::Sum<T>,
{
    c.into_iter().copied().sum()
}

fn longest_len<C>(c: &C) -> usize
where
    for<'a> &'a C: IntoIterator,
    for<'a> <&'a C as IntoIterator>::Item: AsRef<str>,
{
    c.into_iter().map(|s| s.as_ref().chars().count()).max().unwrap_or(0)
}

fn count_by<I, K, F>(items: I, key: F) -> BTreeMap<K, usize>
where
    I: IntoIterator,
    K: Ord,
    F: Fn(&I::Item) -> K,
{
    let mut m = BTreeMap::new();
    for it in items {
        *m.entry(key(&it)).or_insert(0) += 1;
    }
    m
}

fn lookup<'m, K, Q, V>(m: &'m HashMap<K, V>, keys: &[&Q]) -> Vec<Option<&'m V>>
where
    K: Borrow<Q> + Hash + Eq,
    Q: Hash + Eq + ?Sized,
{
    keys.iter().map(|k| m.get(*k)).collect()
}

fn join_paths<P: AsRef<Path>>(base: P, parts: &[&str]) -> PathBuf {
    let mut p = base.as_ref().to_path_buf();
    for part in parts {
        p.push(part);
    }
    p
}

fn greet(name: impl Into<String>, times: usize) -> String {
    let n: String = name.into();
    vec![n; times].join(" & ")
}

// ---------- sorted vec set ----------
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SortedSet<T: Ord> {
    v: Vec<T>,
}

impl<T: Ord> SortedSet<T> {
    fn insert(&mut self, x: T) -> bool {
        match self.v.binary_search(&x) {
            Ok(_) => false,
            Err(i) => {
                self.v.insert(i, x);
                true
            }
        }
    }
    fn contains<Q: Ord + ?Sized>(&self, q: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.v.binary_search_by(|x| x.borrow().cmp(q)).is_ok()
    }
    fn range_iter<'a>(&'a self, lo: &'a T, hi: &'a T) -> impl Iterator<Item = &'a T> + 'a {
        self.v.iter().skip_while(move |x| *x < lo).take_while(move |x| *x <= hi)
    }
}

impl<T: Ord> FromIterator<T> for SortedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        let mut v: Vec<T> = it.into_iter().collect();
        v.sort();
        v.dedup();
        SortedSet { v }
    }
}

impl<T: Ord> Extend<T> for SortedSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for x in it {
            self.insert(x);
        }
    }
}

impl<'a, T: Ord + Clone> From<&'a [T]> for SortedSet<T> {
    fn from(s: &'a [T]) -> Self {
        s.iter().cloned().collect()
    }
}

impl<T: Ord> IntoIterator for SortedSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.v.into_iter()
    }
}

impl<'a, T: Ord> IntoIterator for &'a SortedSet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.v.iter()
    }
}

impl<T: Ord + Display> Display for SortedSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for (i, x) in self.v.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", x)?;
        }
        write!(f, "}}")
    }
}

// ---------- errors ----------
#[derive(Debug)]
struct ConfigError {
    key: String,
    reason: &'static str,
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "config key {:?}: {}", self.key, self.reason)
    }
}
impl Error for ConfigError {}

type BoxErr = Box<dyn Error + Send + Sync + 'static>;

fn parse_config(src: &str) -> Result<HashMap<String, i64>, BoxErr> {
    let mut m = HashMap::new();
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (k, v) = line.split_once('=').ok_or_else(|| format!("missing '=' in {:?}", line))?;
        let k = k.trim();
        if k.is_empty() {
            return Err(Box::new(ConfigError { key: k.to_string(), reason: "empty key" }));
        }
        let v: i64 = v.trim().parse()?;
        if m.insert(k.to_string(), v).is_some() {
            return Err(ConfigError { key: k.to_string(), reason: "duplicate" }.into());
        }
    }
    Ok(m)
}

fn describe_err(e: &BoxErr) -> String {
    if let Some(ce) = e.downcast_ref::<ConfigError>() {
        format!("ConfigError({}, {})", ce.key, ce.reason)
    } else if let Some(pe) = e.downcast_ref::<std::num::ParseIntError>() {
        format!("ParseIntError({})", pe)
    } else {
        format!("other({})", e)
    }
}

// ---------- peekable tokenizer ----------
#[derive(Debug, PartialEq)]
enum Tok {
    Num(u64),
    Word(String),
    Sym(char),
}

struct Lexer<'a> {
    it: Peekable<Chars<'a>>,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Tok;
    fn next(&mut self) -> Option<Tok> {
        while self.it.next_if(|c| c.is_whitespace()).is_some() {}
        let c = *self.it.peek()?;
        Some(if c.is_ascii_digit() {
            let mut n = 0u64;
            while let Some(d) = self.it.next_if(char::is_ascii_digit) {
                n = n * 10 + d.to_digit(10).unwrap() as u64;
            }
            Tok::Num(n)
        } else if c.is_alphabetic() {
            let mut s = String::new();
            while let Some(ch) = self.it.next_if(|c| c.is_alphanumeric() || *c == '_') {
                s.push(ch);
            }
            Tok::Word(s)
        } else {
            self.it.next();
            Tok::Sym(c)
        })
    }
}

struct Ring<T, const N: usize = 4> {
    items: [Option<T>; N],
    at: usize,
}

impl<T: Copy, const N: usize> Default for Ring<T, N> {
    fn default() -> Self {
        Ring { items: [None; N], at: 0 }
    }
}

impl<T: Copy, const N: usize> Ring<T, N> {
    fn push(&mut self, x: T) {
        self.items[self.at % N] = Some(x);
        self.at += 1;
    }
    fn latest(&self) -> Vec<T> {
        (0..N.min(self.at)).map(|i| self.items[(self.at - 1 - i) % N].unwrap()).collect()
    }
}

fn main() {
    let v = vec![3, 1, 4, 1, 5];
    let arr = [2.5f64, 0.5];
    let set: SortedSet<i64> = [9, 2, 7, 2].into_iter().collect();
    println!("totals {} {} {}", total(&v), total(&arr), total(&set));
    let words = vec!["héllo", "hi", "wörld!"];
    let owned: Vec<String> = words.iter().map(|s| s.to_uppercase()).collect();
    println!("longest {} {}", longest_len::<Vec<&str>>(&words), longest_len::<Vec<String>>(&owned));
    let by_len = count_by(words.iter().chain(["yo", "abc"].iter()), |w| w.len());
    let by_parity = count_by(0..10, |x| x % 3);
    println!("{:?} {:?}", by_len, by_parity);
    let mut m: HashMap<String, u32> = HashMap::new();
    m.insert("alpha".to_string(), 1);
    m.insert("beta".to_string(), 2);
    println!("{:?}", lookup(&m, &["beta", "gamma", "alpha"]));
    println!("{:?} {:?}", join_paths("/tmp", &["a", "b.txt"]), join_paths(PathBuf::from("rel"), &[]).to_str());
    println!("{} | {}", greet("ann", 2), greet(String::from("bob"), 3));

    let mut s: SortedSet<String> = SortedSet::default();
    for w in "the quick brown fox jumps over the lazy dog".split(' ') {
        s.insert(w.to_string());
    }
    s.extend(["ant".to_string(), "fox".to_string()]);
    println!("{} contains fox {} cat {}", s, s.contains("fox"), s.contains("cat"));
    let lo = "d".to_string();
    let hi = "q".to_string();
    println!("{:?}", s.range_iter(&lo, &hi).collect::<Vec<_>>());
    let nums = SortedSet::from(&[5, 3, 9, 3, 1][..]);
    let doubled: Vec<i32> = (&nums).into_iter().map(|x| x * 2).collect();
    println!("{} {:?} {:?}", nums, doubled, nums.clone().into_iter().rev().collect::<Vec<_>>());
    for x in &nums {
        print!("{} ", x);
    }
    println!();

    for src in ["a = 1\nb=2", "a=1\na=2", "x 5", " = 3", "k = twelve", ""] {
        match parse_config(src) {
            Ok(m) => {
                let mut kv: Vec<_> = m.into_iter().collect();
                kv.sort();
                println!("ok {:?}", kv);
            }
            Err(e) => println!("err {} -> {}", e, describe_err(&e)),
        }
    }
    let e: BoxErr = "plain".into();
    println!("{} / {}", describe_err(&e), e.source().is_none());

    let toks: Vec<Tok> = Lexer { it: "let x_1 = 42 + y*(7)".chars().peekable() }.collect();
    println!("{:?}", toks);
    let mut lx = Lexer { it: "a b c 1 2 3".chars().peekable() };
    let first_two: Vec<Tok> = lx.by_ref().take(2).collect();
    let rest: Vec<Tok> = lx.collect();
    println!("{:?} then {:?}", first_two, rest);

    let mut r: Ring<u8> = Ring::default();
    let mut r8: Ring<char, 8> = Ring::default();
    for i in 0..6 {
        r.push(i);
        r8.push((b'a' + i) as char);
    }
    println!("{:?} {:?}", r.latest(), r8.latest());
}
