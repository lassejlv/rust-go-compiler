// Structs with several lifetime parameters: setters storing into one
// lifetime's slots, getters reading another's.
mod m0 {
use std::collections::HashMap;
struct Ctx<'src, 'arena> { src: &'src str, names: &'arena mut Vec<String>, map: HashMap<&'src str, usize> }
impl<'src, 'arena> Ctx<'src, 'arena> {
    fn word(&self, i: usize) -> Option<&'src str> { self.src.split(' ').nth(i) }
    fn intern(&mut self, w: &'src str) -> usize {
        if let Some(&i) = self.map.get(w) { return i; }
        self.names.push(w.to_uppercase());
        let i = self.names.len() - 1;
        self.map.insert(w, i);
        i
    }
    fn name(&self, i: usize) -> &str { &self.names[i] }
    fn split_pair(&self) -> (&'src str, &'src str) { self.src.split_once(' ').unwrap_or((self.src, "")) }
}
fn longest_two<'a, 'b>(xs: &'a [&'b str]) -> (&'b str, &'a [&'b str]) { let i = xs.iter().enumerate().max_by_key(|(_, s)| s.len()).map(|(i, _)| i).unwrap_or(0); (xs[i], &xs[..i]) }
fn pick<'a, T>(v: &'a mut Vec<T>, pred: impl Fn(&T) -> bool) -> Option<&'a mut T> { v.iter_mut().find(|x| pred(x)) }
pub fn main_0() {
    let text = String::from("the cat saw the dog");
    let mut arena = Vec::new();
    let w2;
    let pair;
    {
        let mut ctx = Ctx { src: &text, names: &mut arena, map: HashMap::new() };
        let ids: Vec<usize> = text.split(' ').map(|w| ctx.intern(w)).collect();
        w2 = ctx.word(2).unwrap();
        pair = ctx.split_pair();
        println!("{:?} {}", ids, ctx.name(1));
    }
    println!("{} {:?} {:?}", w2, pair, arena);
    let words: Vec<&str> = text.split(' ').collect();
    let (l, before) = longest_two(&words);
    println!("{} {:?}", l, before);
    let mut nums = vec![1, 5, 8, 3];
    if let Some(x) = pick(&mut nums, |&x| x > 4) { *x *= 10; }
    let first_big = pick(&mut nums, |&x| x > 40).map(|x| *x);
    println!("{:?} {:?}", nums, first_big);
}

}
mod m1 {
#[derive(Debug, Default)]
struct Two<'a, 'b> { a: Vec<&'a str>, b: Option<&'b str>, n: usize }
impl<'a, 'b> Two<'a, 'b> {
    fn set_b(&mut self, b: &'b str) { self.b = Some(b); }
    fn add_a(&mut self, a: &'a str) -> &mut Self { self.a.push(a); self.n += 1; self }
    fn first_a(&self) -> Option<&'a str> { self.a.first().copied() }
    fn get_b(&self) -> Option<&'b str> { self.b }
    fn swap_into(self) -> Two<'b, 'a> where 'a: 'b, 'b: 'a { Two { a: self.b.into_iter().collect(), b: self.a.first().copied(), n: self.n } }
}
fn build<'s>(long: &'s str) -> Two<'static, 's> {
    let mut t = Two::default();
    t.add_a("lit").add_a("lit2");
    t.set_b(long);
    t
}
fn keep_b<'x>(src: &'x str) -> Option<&'x str> {
    let local = String::from("temp");
    let mut t: Two<'_, 'x> = Two::default();
    t.add_a(&local);
    t.set_b(src);
    let r = t.get_b();
    println!("{:?}", t.first_a());
    r
}
pub fn main_1() {
    let s = String::from("long-lived");
    let t = build(&s);
    println!("{:?} {:?} {:?}", t, t.first_a(), t.get_b());
    println!("{:?}", keep_b(&s));
    let u: Two<'_, '_> = Two { a: vec!["x"], b: Some("y"), n: 1 }.swap_into();
    println!("{:?}", u);
}

}
mod m2 {
fn zipped<'a, 'b>(xs: &'a [String], ys: &'b [&'static str]) -> Vec<(&'a str, &'static str)> { xs.iter().map(|s| s.as_str()).zip(ys.iter().copied()).collect() }
fn pairs<'a>(xs: &'a [String]) -> Vec<(&'a str, &'static str)> { let mut v = Vec::new(); for x in xs { v.push((x.as_str(), if x.len() > 2 { "long" } else { "short" })); } v }
fn choose<'a, 'b>(x: &'a str, y: &'b str) -> (&'a str, &'b str) { let c = |n: usize| (&x[..n.min(x.len())], &y[..n.min(y.len())]); c(2) }
struct Holder<'a> { parts: Vec<(&'static str, &'a str)> }
impl<'a> Holder<'a> {
    fn add(&mut self, tag: &'static str, s: &'a str) { self.parts.push((tag, s)); }
    fn tags(&self) -> Vec<&'static str> { let mut v = Vec::new(); for p in &self.parts { v.push(p.0); } v }
}
fn tags_only(s: &str) -> Vec<&'static str> {
    let mut h = Holder { parts: vec![] };
    h.add("a", s);
    h.add("b", &s[1..]);
    h.tags()
}
pub fn main_2() {
    let owned = vec![String::from("abc"), String::from("d")];
    println!("{:?}", zipped(&owned, &["x", "y"]));
    println!("{:?}", pairs(&owned));
    let l = String::from("left");
    println!("{:?}", choose(&l, "rightside"));
    let t;
    { let tmp = String::from("temp"); t = tags_only(&tmp); }
    println!("{:?}", t);
}

}
fn main() {
    m2::main_2();
    m0::main_0();
    m1::main_1();
}
