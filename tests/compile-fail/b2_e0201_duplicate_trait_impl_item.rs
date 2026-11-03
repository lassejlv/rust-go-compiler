//~ ERROR E0201
trait T { fn f(&self); }
struct S;
impl T for S {
    fn f(&self) {}
    fn f(&self) {}
}
fn main() {}
