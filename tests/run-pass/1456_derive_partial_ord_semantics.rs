// derived PartialOrd/Ord/PartialEq semantics: variant order, field order, floats, generics
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Shape { Point, Circle(u32), Rect { w: u32, h: u32 }, Poly(Vec<(i32, i32)>) }

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum F { A(f64), B { x: f32 } }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version { major: u16, minor: u16, patch: u16, pre: Option<String> }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Prio { Low = 10, High = 1, Mid = 5 }

#[derive(Debug, PartialEq, PartialOrd)]
struct G<T, U> where T: PartialOrd, U: PartialOrd { t: T, u: U }

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Tup<'a>(&'a str, i8);

fn main() {
    let mut v = vec![
        Shape::Poly(vec![(1, 2)]), Shape::Rect { w: 2, h: 1 }, Shape::Circle(3),
        Shape::Point, Shape::Rect { w: 1, h: 5 }, Shape::Circle(1), Shape::Poly(vec![]),
    ];
    v.sort();
    println!("{:?}", v);
    println!("{:?}", Shape::Circle(9).cmp(&Shape::Rect { w: 0, h: 0 }));
    println!("{:?}", F::A(1.0).partial_cmp(&F::B { x: 0.0 }));
    println!("{:?}", F::A(f64::NAN).partial_cmp(&F::A(1.0)));
    println!("{} {}", F::A(f64::NAN) == F::A(f64::NAN), F::A(1.0) < F::A(2.0));
    println!("{}", F::B { x: 1.0 } > F::A(100.0));
    let mut vs = vec![
        Version { major: 1, minor: 2, patch: 3, pre: None },
        Version { major: 1, minor: 2, patch: 3, pre: Some("alpha".into()) },
        Version { major: 0, minor: 9, patch: 0, pre: None },
        Version { major: 1, minor: 10, patch: 0, pre: None },
    ];
    vs.sort();
    for x in &vs { println!("{:?}", x); }
    let mut ps = vec![Prio::Low, Prio::High, Prio::Mid];
    ps.sort();
    println!("{:?}", ps);
    println!("{:?} {:?}", Prio::Low.cmp(&Prio::High), Prio::Low.max(Prio::Mid));
    let a = G { t: 1, u: "b" };
    let b = G { t: 1, u: "a" };
    println!("{:?} {}", a.partial_cmp(&b), a > b);
    println!("{:?}", Tup("a", 5).cmp(&Tup("a", -5)));
    println!("{:?}", [Tup("b", 0), Tup("a", 9)].iter().max());
    println!("{:?}", Ordering::Less.then(Ordering::Greater));
    println!("{}", Shape::Point <= Shape::Point && Shape::Point >= Shape::Point);
    println!("{:?}", Shape::Poly(vec![(0, 9)]).partial_cmp(&Shape::Poly(vec![(1, 0)])));
}
