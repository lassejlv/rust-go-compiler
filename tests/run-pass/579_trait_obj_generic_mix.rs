// mixing generics with trait objects: generic structs holding Box<dyn Trait<T>>, dyn with generic traits, Deref/Borrow/AsRef, IntoIterator for &T
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

trait Transform<T> {
    fn apply(&self, t: T) -> T;
    fn name(&self) -> String {
        "anon".to_string()
    }
}

struct AddN(i64);
impl Transform<i64> for AddN {
    fn apply(&self, t: i64) -> i64 {
        t + self.0
    }
    fn name(&self) -> String {
        format!("add{}", self.0)
    }
}
struct Upper;
impl Transform<String> for Upper {
    fn apply(&self, t: String) -> String {
        t.to_uppercase()
    }
}
impl<T, F: Fn(T) -> T> Transform<T> for F {
    fn apply(&self, t: T) -> T {
        self(t)
    }
}

impl dyn Transform<i64> + Send {
    fn twice(&self, t: i64) -> i64 {
        self.apply(self.apply(t))
    }
}

struct Chain<T> {
    steps: Vec<Box<dyn Transform<T>>>,
}
impl<T: Debug + Clone> Chain<T> {
    fn run(&self, init: T) -> (T, Vec<String>) {
        let mut log = vec![];
        let out = self.steps.iter().fold(init, |acc, s| {
            let next = s.apply(acc.clone());
            log.push(format!("{}: {:?} -> {:?}", s.name(), acc, next));
            next
        });
        (out, log)
    }
}

struct Stack<T> {
    items: Vec<T>,
}
impl<T> Deref for Stack<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.items
    }
}
impl<T> DerefMut for Stack<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.items
    }
}
impl<'a, T> IntoIterator for &'a Stack<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Rev<std::slice::Iter<'a, T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter().rev()
    }
}
impl<T> FromIterator<T> for Stack<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        Stack { items: it.into_iter().collect() }
    }
}
impl<T> Extend<T> for Stack<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for x in it {
            self.items.push(x);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct Key(String);
impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        &self.0
    }
}

fn total_len<S: AsRef<str>>(items: &[S]) -> usize {
    items.iter().map(|s| s.as_ref().len()).sum()
}

fn print_all<I>(it: I) -> String
where
    I: IntoIterator,
    I::Item: Debug,
{
    it.into_iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(" ")
}

fn largest<'a, T: PartialOrd + ?Sized>(xs: &[&'a T]) -> Option<&'a T> {
    xs.iter().copied().fold(None, |m: Option<&T>, x| match m {
        Some(y) if y >= x => Some(y),
        _ => Some(x),
    })
}

fn main() {
    let c = Chain::<i64> { steps: vec![Box::new(AddN(5)), Box::new(|x: i64| x * 3), Box::new(AddN(-1))] };
    let (v, log) = c.run(2);
    println!("{} {:?}", v, log);
    let s = Chain::<String> { steps: vec![Box::new(Upper), Box::new(|s: String| s + "!"), Box::new(|s: String| s.chars().rev().collect())] };
    println!("{:?}", s.run("abc".into()));
    let mut st: Stack<i32> = (1..=5).collect();
    st.extend(vec![10, 20]);
    println!("len {} first {:?} last {:?}", st.len(), st.first(), st.last());
    st.sort_by(|a, b| b.cmp(a));
    st[0] = -1;
    println!("{:?} contains 3: {}", &st[..], st.contains(&3));
    println!("{}", print_all(&st));
    let mut ordered = vec![];
    for x in &st {
        ordered.push(*x);
    }
    println!("{:?} {}", ordered, st.iter().sum::<i32>());
    let mut m: HashMap<Key, u32> = HashMap::new();
    m.insert(Key("alpha".into()), 1);
    m.insert(Key("beta".into()), 2);
    println!("{:?} {:?} {}", m.get("alpha"), m.get("gamma"), m.contains_key("beta"));
    println!("{} {}", total_len(&["ab", "cde"]), total_len(&vec![String::from("xyz")]));
    println!("{}", print_all(vec![Some(1), None]));
    println!("{}", print_all("hi".chars()));
    println!("{:?} {:?}", largest(&[&3, &9, &2]), largest::<str>(&["pear", "apple", "zoo"]));
    println!("{:?}", largest::<[i32]>(&[&[1, 2][..], &[1, 3], &[0, 9, 9]]));
    let dyns: Vec<&dyn Transform<i64>> = vec![&AddN(1), &AddN(2)];
    println!("{}", dyns.iter().fold(0, |acc, t| t.apply(acc)));
    let sendable: Box<dyn Transform<i64> + Send> = Box::new(AddN(7));
    println!("{}", sendable.twice(1));
    let boxed_str: Box<str> = "boxed str".into();
    let bs: &str = &boxed_str;
    println!("{} {}", bs.len(), boxed_str.to_uppercase());
}
