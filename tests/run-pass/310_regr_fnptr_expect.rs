struct W<F> { f: F }
trait Ext { fn with<F: FnMut(char) -> bool>(&self, f: F) -> W<F> { W { f } } }
impl Ext for str {}
struct Holder { it: W<fn(char) -> bool> }
fn main() {
    let h = Holder { it: "x".with(char::is_whitespace) };
    let g = Holder { it: W { f: char::is_alphabetic } };
    println!("{} {}", (h.it.f)(' '), (g.it.f)('a'));
}
