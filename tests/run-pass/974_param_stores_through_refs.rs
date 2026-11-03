// Valid stores of parameters into places behind references of other
// parameters (checked against the lifetimes declared there).

use std::cell::RefCell;
use std::collections::HashMap;

fn a<'a, 'b: 'a>(v: &mut &'a str, s: &'b str) {
    *v = s;
}
fn b<'a>(v: &mut Vec<&'a str>, s: &'a str) {
    v[0] = s;
}
fn c(v: &mut Vec<&str>, s: &'static str) {
    v[0] = s;
}
struct Two<'a, 'b> {
    x: &'a str,
    y: &'b str,
}
fn d<'a, 'b>(t: &mut Two<'a, 'b>, s: &'b str, r: &'a str) {
    t.y = s;
    t.x = r;
}
struct W<'a> {
    items: Vec<&'a str>,
    cur: &'a str,
    map: HashMap<&'a str, &'a str>,
}
impl<'a> W<'a> {
    fn set(&mut self, x: &'a str) {
        self.items[0] = x;
        self.cur = x;
        *self.map.get_mut("k").unwrap() = x;
        if let Some(v) = self.map.get_mut("k") {
            *v = x;
        }
        let last = self.items.last_mut().unwrap();
        *last = x;
    }
    fn replace(&mut self, other: W<'a>) {
        *self = other;
    }
}
fn e<'a, 'b>(v: &'b mut Vec<&'a str>, w: &'b mut &'a str, s: &'a str) {
    v[0] = s;
    *w = s;
}
fn f<'a>(g: &RefCell<&'a str>, s: &'a str) {
    *g.borrow_mut() = s;
}
fn g<T: Clone>(v: &mut Vec<T>, x: T) {
    v[0] = x;
}
fn h<'a>(slot: &mut &'a mut i32, r: &'a mut i32) {
    *slot = r;
}
fn i<'a>(v: &mut [&'a str; 2], s: &'a str) {
    v[1] = s;
}
fn j<'a, 'b>(v: &mut (&'a str, &'b str), s: &'b str) {
    v.1 = s;
}
fn k<'s>(v: &mut Vec<&'s str>, s: &'s str) {
    for x in v.iter_mut() {
        *x = s;
    }
}
struct Holder<'a> {
    slot: &'a mut &'a str,
}
fn l<'a>(h: Holder<'a>, s: &'a str) {
    *h.slot = s;
}
fn m<'a>(b: &mut Box<&'a str>, s: &'a str) {
    **b = s;
}
fn n<'a>(o: &mut Option<&'a str>, s: &'a str) {
    if let Some(x) = o {
        *x = s;
    }
}

fn main() {
    let s1 = String::from("one");
    let mut x: &str = "x";
    a(&mut x, &s1);
    let mut v = vec!["p", "q"];
    b(&mut v, &s1);
    c(&mut v, "st");
    let mut t = Two { x: "x", y: "y" };
    d(&mut t, "yy", "xx");
    let mut w = W { items: vec!["i"], cur: "c", map: HashMap::new() };
    w.map.insert("k", "v");
    w.set("new");
    w.replace(W { items: vec!["r"], cur: "rc", map: HashMap::new() });
    let mut ww: &str = "w";
    e(&mut v, &mut ww, "e");
    let cell = RefCell::new("c");
    f(&cell, "f");
    let mut gv = vec![1, 2];
    g(&mut gv, 7);
    let mut n1 = 1;
    let mut n2 = 2;
    let mut slot = &mut n1;
    h(&mut slot, &mut n2);
    *slot += 1;
    let mut arr = ["a", "b"];
    i(&mut arr, "i");
    let mut tup = ("a", "b");
    j(&mut tup, "j");
    k(&mut v, "k");
    let mut hs = "h";
    l(Holder { slot: &mut hs }, "l");
    let mut bx = Box::new("b");
    m(&mut bx, "m");
    let mut o = Some("o");
    n(&mut o, "n");
    println!("{} {:?} {} {} {} {:?} {:?} {}", x, v, t.x, t.y, w.cur, w.items, cell.borrow(), ww);
    println!("{:?} {} {} {:?} {:?} {} {:?}", gv, n2, n1, arr, tup, bx, o);
}
