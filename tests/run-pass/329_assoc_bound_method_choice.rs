// The receiver's trait comes from the associated type's bound in the generic
// body, even though after substitution one type implements both traits.
trait Ser: Copy {
    type A: SeqLike;
    type B: MapLike;
    fn seq(self) -> Self::A;
    fn map(self) -> Self::B;
    fn both(self) -> String {
        let s = self.seq();
        let m = self.map();
        format!("{}{}", s.end(), m.end())
    }
}
trait SeqLike { fn end(self) -> &'static str; }
trait MapLike { fn end(self) -> &'static str; }
#[derive(Clone, Copy)]
struct C;
impl SeqLike for C { fn end(self) -> &'static str { "]" } }
impl MapLike for C { fn end(self) -> &'static str { "}" } }
#[derive(Clone, Copy)]
struct S;
impl Ser for S {
    type A = C;
    type B = C;
    fn seq(self) -> C { C }
    fn map(self) -> C { C }
}
fn go<T: Ser>(t: T) -> (&'static str, &'static str) { (t.seq().end(), t.map().end()) }
fn main() {
    println!("{:?} {}", go(S), S.both());
}
