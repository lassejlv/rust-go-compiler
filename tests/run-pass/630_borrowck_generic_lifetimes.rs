// Generic functions with explicit lifetimes and bounds.
use std::collections::HashMap;

fn longest<'a>(xs: &'a [String]) -> Option<&'a String> {
    let mut best: Option<&'a String> = None;
    for x in xs {
        match best {
            Some(b) if b.len() >= x.len() => {}
            _ => best = Some(x),
        }
    }
    best
}

fn index<'a>(words: &[&'a str]) -> HashMap<&'a str, usize> {
    let mut m = HashMap::new();
    for (i, w) in words.iter().enumerate() {
        m.entry(*w).or_insert(i);
    }
    m
}

struct Split<'a> {
    rest: &'a str,
}

impl<'a> Iterator for Split<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        match self.rest.find(',') {
            Some(i) => {
                let head = &self.rest[..i];
                self.rest = &self.rest[i + 1..];
                Some(head)
            }
            None => {
                let head = self.rest;
                self.rest = "";
                Some(head)
            }
        }
    }
}

fn main() {
    let v = vec![String::from("ab"), String::from("abcd"), String::from("x")];
    let l = longest(&v).cloned();
    let src = String::from("b,a,c,a");
    let parts: Vec<&str> = Split { rest: &src }.collect();
    let idx = index(&parts);
    let mut keys: Vec<_> = idx.iter().collect();
    keys.sort();
    println!("{:?} {:?} {:?}", l, parts, keys);
}
