use std::fmt::Debug;
use std::marker::PhantomData;

// Finite polymorphic recursion: the depth is bounded by a type-level number.
struct Z;
struct S<N>(PhantomData<N>);
trait Nat {
    const N: usize;
    type Wrap<T: Debug>: Debug;
    fn wrap<T: Debug>(t: T) -> Self::Wrap<T>;
    fn pairs<T: Debug + Clone>(t: T) -> String;
}
impl Nat for Z {
    const N: usize = 0;
    type Wrap<T: Debug> = T;
    fn wrap<T: Debug>(t: T) -> T { t }
    fn pairs<T: Debug + Clone>(t: T) -> String { format!("{:?}", t) }
}
impl<P: Nat> Nat for S<P> {
    const N: usize = P::N + 1;
    type Wrap<T: Debug> = P::Wrap<Vec<T>>;
    fn wrap<T: Debug>(t: T) -> Self::Wrap<T> { P::wrap(vec![t]) }
    fn pairs<T: Debug + Clone>(t: T) -> String { P::pairs((t.clone(), t)) }
}
type N3 = S<S<S<Z>>>;
type N6 = S<S<S<N3>>>;
type N12 = S<S<S<S<S<S<N6>>>>>>;

// Recursion through trait objects: one instance, unbounded depth.
fn nest_dyn(depth: u32, x: &dyn Debug) -> String {
    if depth == 0 { format!("{:?}", x) } else { nest_dyn(depth - 1, &[x]) }
}

// A nested data type traversed through a trait object.
#[derive(Debug)]
enum Tree<T> { Leaf(T), Node(Vec<Tree<T>>) }
trait Size { fn size(&self) -> usize; fn depth(&self) -> usize; }
impl<T> Size for Tree<T> {
    fn size(&self) -> usize { match self { Tree::Leaf(_) => 1, Tree::Node(ch) => ch.iter().map(|c| c.size()).sum() } }
    fn depth(&self) -> usize { match self { Tree::Leaf(_) => 0, Tree::Node(ch) => 1 + ch.iter().map(Size::depth).max().unwrap_or(0) } }
}
fn build(d: u32) -> Tree<u32> { if d == 0 { Tree::Leaf(d) } else { Tree::Node((0..d).map(|i| build(d - 1 - i.min(d - 1))).collect()) } }

fn main() {
    println!("{:?}", <N3 as Nat>::wrap(7u8));
    println!("{} {:?}", N6::N, N6::wrap("x"));
    println!("{}", N12::N);
    let deep = format!("{:?}", N12::wrap(1));
    println!("{} {}", deep.len(), &deep[..14]);
    println!("{}", <N3 as Nat>::pairs('z'));
    println!("{}", nest_dyn(5, &"core"));
    let t = build(4);
    println!("{} {}", t.size(), t.depth());
}
