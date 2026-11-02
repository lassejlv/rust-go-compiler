use std::fmt::Display;
use std::ops::{Add, Mul, Neg, Sub};

trait Shout {
    fn shout(&self) -> String;
}

impl<T: Display> Shout for T {
    fn shout(&self) -> String {
        format!("{}!", self.to_string().to_uppercase())
    }
}

trait Weight {
    fn weight(&self) -> u32;
}

impl Weight for u32 {
    fn weight(&self) -> u32 {
        *self
    }
}
impl Weight for str {
    fn weight(&self) -> u32 {
        self.len() as u32
    }
}
impl<T: Weight + ?Sized> Weight for &T {
    fn weight(&self) -> u32 {
        (**self).weight() + 1000
    }
}
impl<T: Weight> Weight for Vec<T> {
    fn weight(&self) -> u32 {
        self.iter().map(|x| x.weight()).sum()
    }
}
impl<T: Weight> Weight for Option<T> {
    fn weight(&self) -> u32 {
        match self {
            Some(x) => x.weight(),
            None => 0,
        }
    }
}

fn total<W: Weight>(w: W) -> u32 {
    w.weight()
}

#[derive(Debug, Clone, PartialEq)]
struct V2 {
    x: i64,
    y: i64,
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
}
impl<'a> Add<&'a V2> for &'a V2 {
    type Output = V2;
    fn add(self, o: &V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
}
impl Sub<&V2> for V2 {
    type Output = V2;
    fn sub(self, o: &V2) -> V2 {
        V2 { x: self.x - o.x, y: self.y - o.y }
    }
}
impl Mul<i64> for &V2 {
    type Output = V2;
    fn mul(self, k: i64) -> V2 {
        V2 { x: self.x * k, y: self.y * k }
    }
}
impl Mul<&V2> for i64 {
    type Output = V2;
    fn mul(self, v: &V2) -> V2 {
        v * self
    }
}
impl Neg for &V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2 { x: -self.x, y: -self.y }
    }
}

fn sum_refs(vs: &[V2]) -> V2 {
    let mut acc = V2 { x: 0, y: 0 };
    for v in vs {
        acc = &acc + v;
    }
    acc
}

fn main() {
    println!("{}", 42.shout());
    println!("{}", "hey".shout());
    println!("{}", 'q'.shout());
    println!("{}", 1.5f64.shout());
    println!("{}", String::from("mixed Case").shout());

    println!("{}", total(5u32));
    println!("{}", total(&5u32));
    println!("{}", total(&&5u32));
    println!("{}", total("abcd"));
    println!("{}", total(vec![1u32, 2, 3]));
    println!("{}", total(vec![Some(4u32), None, Some(6)]));
    println!("{}", total(vec![&1u32, &2]));

    let a = V2 { x: 1, y: 2 };
    let b = V2 { x: 10, y: 20 };
    let c = &a + &b;
    println!("{:?}", c);
    println!("{:?}", c.clone() - &a);
    println!("{:?}", &b * 3);
    println!("{:?}", 2 * &a);
    println!("{:?}", -&c);
    println!("{:?}", a.clone() + b.clone());
    println!("{:?}", sum_refs(&[a.clone(), b.clone(), c.clone()]));
    println!("a still usable: {:?}, eq {}", a, &a + &b == c);
}
