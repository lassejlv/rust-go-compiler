use std::fmt::{Debug, Write as FmtWrite};
use std::io::Write as IoWrite;

trait Sink {
    fn put(&mut self, s: &str);
    fn put_all<I: IntoIterator>(&mut self, items: I) where Self: Sized, I::Item: Debug {
        for x in items { self.put(&format!("{:?}", x)); }
    }
    fn tagged(self, tag: &'static str) -> Tagged<Self> where Self: Sized { Tagged { inner: self, tag } }
}
impl<S: Sink + ?Sized> Sink for &mut S { fn put(&mut self, s: &str) { (**self).put(s) } }
impl<S: Sink + ?Sized> Sink for Box<S> { fn put(&mut self, s: &str) { (**self).put(s) } }

struct VecSink(Vec<String>);
impl Sink for VecSink { fn put(&mut self, s: &str) { self.0.push(s.to_string()) } }
struct CountSink(usize);
impl Sink for CountSink { fn put(&mut self, s: &str) { self.0 += s.len() } }
struct Tagged<S> { inner: S, tag: &'static str }
impl<S: Sink> Sink for Tagged<S> { fn put(&mut self, s: &str) { let t = format!("{}:{}", self.tag, s); self.inner.put(&t) } }

fn feed(s: &mut dyn Sink) {
    s.put("start");
    let mut r: &mut dyn Sink = s;
    (&mut r).put_all(vec![1, 2]);
    (&mut r).put_all(["x"]);
    let mut t = r.tagged("T");
    t.put("end");
    t.put_all(Some(3.5));
}

fn write_dyn(w: &mut dyn FmtWrite, n: i32) -> std::fmt::Result {
    write!(w, "n={}", n)?;
    writeln!(w, ";")
}
fn write_gen<W: FmtWrite>(mut w: W) -> W { let _ = write!(w, "[gen]"); w }
fn io_dyn(w: &mut dyn IoWrite) -> std::io::Result<()> { w.write_all(b"io-")?; write!(w, "{}", 42) }

trait Counter { fn next_val(&mut self) -> u32; fn take_vals(&mut self, n: usize) -> Vec<u32> where Self: Sized { (0..n).map(|_| self.next_val()).collect() } }
struct Up(u32);
impl Counter for Up { fn next_val(&mut self) -> u32 { self.0 += 1; self.0 } }
impl<C: Counter + ?Sized> Counter for &mut C { fn next_val(&mut self) -> u32 { (**self).next_val() } }

fn main() {
    let mut v = VecSink(vec![]);
    feed(&mut v);
    println!("{:?}", v.0);
    let mut c = CountSink(0);
    feed(&mut c);
    println!("{}", c.0);
    let mut b: Box<dyn Sink> = Box::new(VecSink(vec![]));
    b.put_all(&[7u8, 8]);
    feed(&mut *b);
    let mut s = String::new();
    write_dyn(&mut s, 5).unwrap();
    let s = write_gen(s);
    let s2 = write_gen(String::from("pre"));
    println!("{:?} {:?}", s, s2);
    let mut buf: Vec<u8> = Vec::new();
    io_dyn(&mut buf).unwrap();
    let mut cur = std::io::Cursor::new(Vec::new());
    io_dyn(&mut cur).unwrap();
    println!("{:?} {:?}", String::from_utf8(buf).unwrap(), cur.into_inner().len());
    let mut u = Up(0);
    let d: &mut dyn Counter = &mut u;
    let mut dd = d;
    println!("{:?}", (&mut dd).take_vals(3));
    println!("{:?} {}", (&mut dd).take_vals(2), u.0);
}
