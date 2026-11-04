// edition:2015
// Edition 2015: 2018+ keywords usable as field/function/module/macro names,
// raw identifiers, `::std` absolute paths, nested use groups with `self`,
// and borrow patterns accepted by NLL (which also applies to 2015 today).
use std::collections::{self, HashMap};
use std::fmt::{self as format_mod, Write};

mod async {
    pub fn await(x: u32) -> u32 { x + 1 }
    pub struct dyn { pub try: u32 }
}

#[derive(Debug, Default)]
struct Config {
    async: bool,
    await: u32,
    dyn: &'static str,
    r#match: i8,
}

fn try(v: &[i32]) -> Option<i32> { v.iter().cloned().max() }

macro_rules! async {
    ($e:expr) => { format!("<async {}>", $e) };
}

fn first_word(s: &mut String) -> &str {
    // NLL: the mutable borrow ends before the shared reborrow
    s.push_str(" tail");
    let end = s.find(' ').unwrap_or(s.len());
    &s[..end]
}

fn longest_run(v: &mut Vec<i32>) -> usize {
    let mut best = 0;
    let mut cur = 0;
    let mut prev: Option<i32> = None;
    for x in v.iter() {
        cur = if prev == Some(*x) { cur + 1 } else { 1 };
        if cur > best { best = cur; }
        prev = Some(*x);
    }
    v.push(best as i32); // allowed: iteration borrow already ended
    best
}

fn main() {
    let c = Config { async: true, await: 7, dyn: "yes", r#match: -1 };
    println!("{:?}", c);
    println!("{} {} {} {}", c.async, c.await, c.dyn, c.r#match);
    let d = async::dyn { try: async::await(41) };
    println!("{}", d.try);
    println!("{:?} {:?}", try(&[3, 9, 2]), try(&[]));
    println!("{}", async!(1 + 2));

    let mut m: HashMap<&str, Vec<u32>> = ::std::collections::HashMap::new();
    m.entry("a").or_insert_with(Vec::new).push(1);
    m.entry("a").or_insert_with(Vec::new).push(2);
    let set: collections::BTreeSet<u8> = [3u8, 1, 2].iter().cloned().collect();
    println!("{:?} {:?}", m.get("a"), set);

    let mut out = String::new();
    write!(out, "{}-{}", 1, 2).unwrap();
    let r: format_mod::Result = Ok(());
    println!("{} {:?}", out, r);

    let mut s = String::from("hello world");
    let w = first_word(&mut s).to_string();
    println!("{} | {}", w, s);
    let mut v = vec![1, 1, 2, 2, 2, 3, 2, 2];
    let n = longest_run(&mut v);
    println!("{} {:?}", n, v);

    let mut data = vec![5, 3, 8];
    let total: i32 = data.iter().sum();
    data.push(total);
    let last = data.last_mut().unwrap();
    *last *= 2;
    println!("{:?}", data);
    let x = ::std::cmp::max(1, 2);
    println!("{}", x);
}
