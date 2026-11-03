// More valid stores through references of parameters (where-clauses,
// trait impls, RefCell guards, slices, nested references).

use std::collections::HashMap;
fn a<'a, 'b>(v: &mut Vec<&'a str>, s: &'b str)
where
    'b: 'a,
{
    v[0] = s;
}
struct Parser<'src> {
    src: &'src str,
    pos: usize,
    names: HashMap<String, &'src str>,
}
impl<'src> Parser<'src> {
    fn reset(&mut self, src: &'src str) {
        self.src = src;
        self.pos = 0;
    }
    fn alias(&mut self, k: &str, v: &'src str) {
        if let Some(slot) = self.names.get_mut(k) {
            *slot = v;
        }
    }
}
trait Visit<'a> {
    fn visit(&mut self, n: &'a String);
}
struct Last<'a> {
    last: Option<&'a String>,
    cur: &'a String,
}
impl<'a> Visit<'a> for Last<'a> {
    fn visit(&mut self, n: &'a String) {
        self.cur = n;
    }
}
struct Gen<'a, T> {
    r: &'a T,
}
impl<'a, T> Gen<'a, T> {
    fn set(&mut self, x: &'a T) {
        self.r = x;
    }
}
fn nested<'a, 'b>(outer: &'b mut Vec<&'a str>, inner: &'b mut &'a str, s: &'a str) {
    *inner = s;
    outer[0] = *inner;
}
fn refcells<'a>(v: &std::cell::RefCell<Vec<&'a str>>, s: &'a str) {
    v.borrow_mut()[0] = s;
}
fn sl<'a>(v: &mut [&'a str], s: &'a str) {
    v[0] = s;
    if let Some(f) = v.first_mut() {
        *f = s;
    }
}
fn main() {
    let mut v = vec!["a"];
    a(&mut v, "b");
    let mut p = Parser { src: "x", pos: 1, names: HashMap::new() };
    p.names.insert("k".to_string(), "v");
    p.reset("y");
    p.alias("k", "w");
    let s1 = String::from("s1");
    let s2 = String::from("s2");
    let mut l = Last { last: None, cur: &s1 };
    l.visit(&s2);
    let mut g = Gen { r: &1 };
    g.set(&2);
    let mut inner = "i";
    nested(&mut v, &mut inner, "n");
    let c = std::cell::RefCell::new(vec!["c"]);
    refcells(&c, "rc");
    let mut arr = ["q", "r"];
    sl(&mut arr, "sl");
    println!("{:?} {} {} {:?} {} {:?} {} {} {:?} {:?}", v, p.src, p.pos, p.names, l.cur, l.last, g.r, inner, c.borrow(), arr);
}
