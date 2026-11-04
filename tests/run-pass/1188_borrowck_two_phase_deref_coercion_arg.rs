// Borrow checking: a call argument deref-coerced to `&mut S` (from
// `&mut &mut S` or `&mut Box<S>`) is a two-phase borrow like an autoref, so
// later arguments may read the place (zlib-rs:
// `let state = &mut stream.state; send_all_trees(state, state.l_desc.max_code + 1, ..)`).
struct S {
    x: usize,
    y: usize,
}
struct W<'a> {
    s: &'a mut S,
}
fn f(s: &mut S, v: usize) -> usize {
    s.y += v;
    s.x + v
}
fn g(s: &mut S, a: usize, b: usize) -> usize {
    s.x += a * b;
    s.x
}
fn a(state: &mut S) -> usize {
    f(state, state.x)
}
fn b(w: &mut W) -> usize {
    let state = &mut w.s;
    f(state, state.x)
}
fn c(w: &mut W) -> usize {
    let state = &mut *w.s;
    f(state, state.x)
}
fn d(w: &mut W) -> usize {
    let state = &mut w.s;
    g(state, state.x + 1, state.y)
}
fn e(bx: &mut Box<S>) -> usize {
    f(bx, bx.x + bx.y)
}
impl S {
    fn m(&mut self, s: &mut S, v: usize) -> usize {
        self.x + s.x + v
    }
}
fn main() {
    let mut s = S { x: 1, y: 0 };
    println!("{}", a(&mut s));
    let mut w = W { s: &mut s };
    println!("{}", b(&mut w));
    println!("{} {}", c(&mut w), w.s.y);
    println!("{} {}", d(&mut w), w.s.x);
    let mut bx = Box::new(S { x: 2, y: 3 });
    println!("{} {}", e(&mut bx), bx.y);
    let mut other = S { x: 5, y: 6 };
    let r = &mut bx;
    println!("{}", other.m(r, r.x));
}
