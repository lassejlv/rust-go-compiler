//~ ERROR E0499
struct Writer<'a> { buf: &'a mut String }
impl<'a> Writer<'a> { fn w(&mut self, s: &str) { self.buf.push_str(s); } }
fn main() {
    let mut s = String::new();
    let mut w1 = Writer { buf: &mut s };
    let mut w2 = Writer { buf: &mut s };
    w1.w("a");
    w2.w("b");
}
