use std::fmt::Debug;

#[derive(Debug, Default, Clone, PartialEq)]
struct Point { x: i32, y: i32 }
#[derive(Debug)]
enum Shape { Dot(Point), Line { a: Point, b: Point } }

trait Factory { type Out: Debug; type Kind; const SCALE: i32; fn make(n: i32) -> Self::Out; }
struct PF; struct SF;
impl Factory for PF {
    type Out = Point;
    type Kind = Shape;
    const SCALE: i32 = 10;
    fn make(n: i32) -> Self::Out { Self::Out { x: n * Self::SCALE, y: -n } }
}
impl Factory for SF {
    type Out = Shape;
    type Kind = Point;
    const SCALE: i32 = 2;
    fn make(n: i32) -> Shape {
        if n % 2 == 0 { Self::Out::Dot(Point { x: n, y: n }) } else { Shape::Line { a: Point::default(), b: Point { x: n * Self::SCALE, y: 0 } } }
    }
}
trait Check: Factory { fn is_line(s: &Self::Out) -> bool; fn origin() -> Self::Out; }
impl Check for SF {
    fn is_line(s: &Self::Out) -> bool { matches!(s, Self::Out::Line { a: Point { x: 0, .. }, .. }) }
    fn origin() -> Self::Out { Self::Out::Line { a: Point::default(), b: Point { x: 0, y: 0 } } }
}
impl Check for PF {
    fn is_line(s: &Self::Out) -> bool { let Self::Out { x, y } = s; x == y }
    fn origin() -> Self::Out { Self::Out { x: 0, y: 0 } }
}
trait HasScale { const SCALE: i32; }
impl HasScale for Point { const SCALE: i32 = 3; }

fn scale_of<F: Factory>() -> i32 where F::Kind: HasScale { <F::Kind as HasScale>::SCALE * F::SCALE }
type PO = <PF as Factory>::Out;
type SO = <SF as Factory>::Out;
fn sum_x(p: <PF as Factory>::Out) -> i32 { let PO { x, .. } = p; x }
fn describe(s: &<SF as Factory>::Out) -> String {
    match s {
        SO::Dot(p) => format!("dot@{}", p.x),
        SO::Line { b, .. } => format!("line->{}", b.x),
    }
}

struct Wrapper<T>(T);
impl<T: Factory> Wrapper<T> {
    fn build(&self, n: i32) -> T::Out { T::make(n) }
    fn batch(&self, ns: &[i32]) -> Vec<<T as Factory>::Out> { ns.iter().map(|&n| T::make(n)).collect() }
}

fn main() {
    let p = PF::make(2);
    println!("{:?} {}", p, sum_x(p.clone()));
    for n in 1..=2 { let s = SF::make(n); println!("{:?} {}", s, describe(&s)); }
    println!("{}", scale_of::<SF>());
    println!("{:?}", Wrapper(PF).batch(&[1, 2]));
    println!("{:?}", Wrapper(SF).build(4));
    println!("{} {} {:?} {:?}", SF::is_line(&SF::make(3)), PF::is_line(&PF::origin()), SF::origin(), PF::origin());
    let q = PO { x: 1, ..Default::default() };
    println!("{:?}", q);
}
