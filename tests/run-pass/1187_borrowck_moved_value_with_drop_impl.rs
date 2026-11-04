// Borrow checking: a value whose type has a Drop impl and holds a borrow is
// not dropped at the end of its scope once it has been moved out (into
// drop(), another binding, a function), so the borrow ends at the move
// (utf-8's LossyDecoder: `let mut d = LossyDecoder::new(|s| out.push_str(s));
// d.feed(..); drop(d); use(out)`).
struct D<'a> {
    r: &'a mut i32,
}
impl<'a> Drop for D<'a> {
    fn drop(&mut self) {
        *self.r += 1
    }
}
struct C<F: FnMut()> {
    f: F,
}
impl<F: FnMut()> Drop for C<F> {
    fn drop(&mut self) {
        (self.f)()
    }
}
fn consume(d: D) -> i32 {
    *d.r * 10
}
fn cond(n: i32) -> bool {
    n % 2 == 0
}

fn main() {
    let mut n = 0;
    let d = D { r: &mut n };
    drop(d);
    println!("{}", n);

    let c = C { f: || n += 1 };
    std::mem::drop(c);
    let c2 = C { f: || n += 1 };
    let moved = c2;
    drop(moved);
    println!("{}", n);

    let d = D { r: &mut n };
    let x = consume(d);
    println!("{} {}", x, n);

    // moved on every path
    let k = n;
    let d = D { r: &mut n };
    if cond(k) {
        drop(d);
    } else {
        let _keep = d;
    }
    println!("{}", n);

    // moved, then reassigned with a borrow of something else
    let mut m = 100;
    let mut d = D { r: &mut n };
    *d.r += 5;
    drop(d);
    d = D { r: &mut m };
    println!("{}", n);
    *d.r += 1;
    drop(d);
    println!("{} {}", n, m);

    // moved inside a loop body
    let mut out = String::new();
    for i in 0..3 {
        let mut s = C { f: || out.push('x') };
        (s.f)();
        drop(s);
        out.push_str(&i.to_string());
    }
    println!("{}", out);
}
