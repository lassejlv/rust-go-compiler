// Values built from references of different lifetimes, stored in tuples:
// let annotations mixing 'static and other lifetimes, Vec::push of tuples
// into vectors returned with distinct lifetimes per element.
mod m0 {
type Splitter = Box<dyn for<'a> Fn(&'a str) -> Vec<&'a str>>;
fn words(s: &str) -> Vec<&str> { s.split_whitespace().collect() }
fn first<'a>(s: &'a str) -> &'a str { s.split(',').next().unwrap_or("") }
fn apply_all<'a>(fs: &[fn(&str) -> &str], s: &'a str) -> Vec<&'a str> { fs.iter().map(|f| f(s)).collect() }
fn compose<F, G>(f: F, g: G) -> impl for<'a> Fn(&'a str) -> &'a str where F: for<'a> Fn(&'a str) -> &'a str, G: for<'a> Fn(&'a str) -> &'a str { move |s| g(f(s)) }
fn trim_both(s: &str) -> &str { s.trim() }
fn longest_by<'a, F: Fn(&str) -> usize>(xs: &[&'a str], key: F) -> Option<&'a str> { xs.iter().copied().max_by_key(|s| key(s)) }
struct Tokenizer<'s> { src: &'s str, rules: Vec<(&'static str, fn(char) -> bool)> }
impl<'s> Tokenizer<'s> {
    fn tokens(&self) -> Vec<(&'static str, &'s str)> {
        let mut out = vec![];
        let mut rest = self.src;
        while let Some(c) = rest.chars().next() {
            if c.is_whitespace() { rest = &rest[c.len_utf8()..]; continue; }
            let (name, f) = self.rules.iter().find(|(_, f)| f(c)).copied().unwrap_or(("other", |_| false));
            let end = if name == "other" { c.len_utf8() } else { rest.find(|ch: char| !f(ch)).unwrap_or(rest.len()) };
            out.push((name, &rest[..end]));
            rest = &rest[end..];
        }
        out
    }
}
pub fn main_0() {
    let sp: Splitter = Box::new(|s| s.split('-').collect());
    let sp2: Splitter = Box::new(words);
    let text = String::from("a-b c-d");
    println!("{:?} {:?}", sp(&text), sp2(&text));
    let fs: [fn(&str) -> &str; 3] = [first, trim_both, |s| &s[..1]];
    println!("{:?}", apply_all(&fs, " x, y"));
    let c = compose(trim_both, first);
    let owned = String::from("  k1,k2  ");
    println!("{:?}", c(&owned));
    println!("{:?}", longest_by(&["aa", "b", "cccc"], |s| s.len()));
    let t = Tokenizer { src: "foo  42+bar_9 ==", rules: vec![("ident", |c| c.is_alphabetic() || c == '_'), ("num", |c| c.is_ascii_digit()), ("op", |c| "+=".contains(c))] };
    println!("{:?}", t.tokens());
}

}
mod m1 {
struct T<'s> { src: &'s str }
impl<'s> T<'s> {
    fn a(&self) -> Vec<(&'static str, &'s str)> {
        let mut out = Vec::new();
        out.push(("x", self.src));
        out
    }
    fn b(&self) -> Vec<&'s str> {
        let mut out = Vec::new();
        out.push(self.src);
        out
    }
    fn c(&self) -> Vec<(u8, &'s str)> {
        let mut out = Vec::new();
        out.push((1, self.src));
        out
    }
    fn d(&self) -> Option<(&'static str, &'s str)> {
        let mut out = None;
        out = Some(("x", self.src));
        out
    }
}
fn e<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    let mut out = Vec::new();
    out.push(("x", src));
    out
}
pub fn main_1() {
    let t = T { src: "foo" };
    println!("{:?} {:?} {:?} {:?} {:?}", t.a(), t.b(), t.c(), t.d(), e("q"));
}

}
mod m2 {
fn e1<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    vec![("x", src)]
}
fn e2<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    let mut out: Vec<(&'static str, &'s str)> = Vec::new();
    out.push(("x", src));
    out
}
fn e3<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    let mut out = Vec::new();
    let t: (&'static str, &'s str) = ("x", src);
    out.push(t);
    out
}
fn e4<'s>(src: &'s str) -> (&'static str, &'s str) {
    let mut out = ("", "");
    out = ("x", src);
    out
}
fn e5<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    let mut out = Vec::new();
    Vec::push(&mut out, ("x", src));
    out
}
fn e6<'s>(src: &'s str) -> Vec<(&'s str, &'static str)> {
    let mut out = Vec::new();
    out.push((src, "x"));
    out
}
pub fn main_2() {
    println!("{:?}", (e1("a"), e2("b"), e3("c"), e4("d"), e5("e"), e6("f")));
}

}
mod m3 {
fn f1<'s>(src: &'s str) -> &'s str { let t: (&'static str, &'s str) = ("x", src); t.1 }
fn f2<'s>(src: &'s str) -> &'s str { let t: (&'s str, &'static str) = (src, "x"); t.0 }
fn f3<'s>(src: &'s str) -> &'s str { let t: (&'s str, &'s str) = ("x", src); t.1 }
fn f4<'s>(src: &'s str) -> usize { let t: (&'static str, &'s str) = ("x", src); t.0.len() }
fn f5<'s>(src: &'s str) -> &'s str { let t: (&'static str, &str) = ("x", src); t.1 }
fn f6<'s>(src: &'s str) -> &'s str { let t: [&'s str; 2] = ["x", src]; t[1] }
fn f7<'s>(src: &'s str, o: &'static str) -> &'s str { let t: (&'static str, &'s str) = (o, src); t.1 }
pub fn main_3() { println!("{} {} {} {} {} {} {}", f1("a"), f2("b"), f3("c"), f4("d"), f5("e"), f6("f"), f7("g", "h")); }

}
mod m4 {
use std::collections::{BTreeMap, HashMap, VecDeque};
struct Pair<'a, 'b> { a: &'a str, b: &'b str }
fn m1<'s>(src: &'s str) -> HashMap<&'static str, &'s str> { let mut m = HashMap::new(); m.insert("k", src); m }
fn m2<'s>(src: &'s str) -> BTreeMap<&'s str, &'static str> { let mut m = BTreeMap::new(); m.insert(src, "v"); m.entry(src).or_insert("w"); m }
fn m3<'s>(src: &'s str) -> VecDeque<(&'static str, &'s str)> { let mut q = VecDeque::new(); q.push_back(("a", src)); q.push_front(("b", src)); q }
fn m4<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> { let mut v = Vec::new(); v.extend([("e", src)]); v.insert(0, ("i", src)); v }
fn m5<'s>(src: &'s str) -> Option<(&'static str, &'s str)> { let mut o = None; o.insert(("o", src)); o }
fn m6<'s>(src: &'s str) -> Pair<'static, 's> { let mut p = Pair { a: "", b: src }; p.a = "z"; p }
fn m7<'s>(src: &'s str) -> Vec<Pair<'static, 's>> { let mut v = Vec::new(); v.push(Pair { a: "q", b: src }); v }
fn m8<'s>(src: &'s str) -> [(&'static str, &'s str); 2] { let mut a = [("", ""); 2]; a[1] = ("x", src); a }
fn m9<'s>(srcs: &[&'s str]) -> Vec<(usize, &'static str, &'s str)> { let mut v = Vec::with_capacity(srcs.len()); for (i, s) in srcs.iter().enumerate() { v.push((i, if i % 2 == 0 { "even" } else { "odd" }, *s)); } v }
pub fn main_4() {
    let s = String::from("src");
    println!("{:?} {:?} {:?}", m1(&s), m2(&s), m3(&s));
    println!("{:?} {:?}", m4(&s), m5(&s));
    let p = m6(&s);
    println!("{} {} {}", p.a, p.b, m7(&s).len());
    println!("{:?} {:?}", m8(&s), m9(&["a", "b", &s]));
}

}
fn main() {
    m4::main_4();
    m0::main_0();
    m1::main_1();
    m2::main_2();
    m3::main_3();
}
