// Borrow checking: an operand that reads a Copy place is a copy taken at its
// position in evaluation order, so later operands of the same aggregate,
// constructor or call may move or mutate that place (ttf-parser colr.rs:
// `LinearGradient { extend: color_line.extend, color_line: ColorLine::NonVar(color_line) }`).
#[derive(Clone, Copy, Debug)]
struct E(u8);
#[derive(Debug)]
struct Line {
    e: E,
    s: String,
}
#[derive(Debug)]
enum W {
    Non(Line),
}
#[derive(Debug)]
struct Wrap(Line);
#[derive(Debug)]
struct G {
    e: E,
    l: W,
}
#[derive(Debug)]
struct G2 {
    e: E,
    l: Wrap,
}
#[derive(Debug)]
struct G3 {
    e: E,
    l: Line,
}
fn id(l: Line) -> Line {
    l
}
fn take(e: E, l: Line) -> String {
    format!("{:?}/{}", e, l.s)
}
struct P {
    a: u32,
    b: Vec<u32>,
}
fn pair(a: u32, b: &mut P) -> u32 {
    b.b.push(a);
    a + b.a
}

fn main() {
    let l = Line { e: E(1), s: "x".into() };
    let g = G { e: l.e, l: W::Non(l) };
    println!("{:?}", g);

    let l = Line { e: E(2), s: "y".into() };
    let g = G2 { e: l.e, l: Wrap(l) };
    println!("{:?}", g);

    let l = Line { e: E(3), s: "z".into() };
    let g = G3 { e: l.e, l: id(l) };
    println!("{:?}", g);

    let l = Line { e: E(4), s: "w".into() };
    let t = (l.e, W::Non(l));
    println!("{:?}", t);

    let l = Line { e: E(5), s: "v".into() };
    println!("{}", take(l.e, l));

    let l = Line { e: E(6), s: "u".into() };
    let arr = [Some(W::Non(Line { e: l.e, s: String::new() })), Some(W::Non(l))];
    println!("{:?}", arr);

    // a copied field, then a mutable borrow of the whole value
    let mut p = P { a: 7, b: vec![] };
    let r = pair(p.a, &mut p);
    println!("{} {:?}", r, p.b);

    // an element read through Index, then the vector borrowed mutably
    let mut v = vec![1, 2];
    let t = (v[0], { v[0] = 9; v[0] }, v[1] + { v.push(3); v.len() });
    println!("{:?} {:?}", t, v);

    // a field read, then the struct mutated
    let mut p2 = P { a: 1, b: vec![2] };
    let x = (p2.a, { p2.a = 3; p2.a }, p2.b[0] + { p2.b.clear(); 0 });
    println!("{:?} {} {:?}", x, p2.a, p2.b);
}
