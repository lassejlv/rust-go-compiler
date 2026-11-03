// In a generic body, a method that comes from a bound (directly, through a
// blanket impl like `ToString for T: Display`, or a supertrait) is the one
// called, even when the instance type has an inherent method of the same
// name; `T::item` paths only see T's traits too.

use std::fmt::{self, Display};

struct B(u8);
impl B {
    fn to_string(&self) -> String {
        "inherent to_string".into()
    }
    fn get(&self) -> u8 {
        1
    }
    fn clone(&self) -> B {
        println!("inherent clone");
        B(self.0)
    }
    fn len(&self) -> usize {
        999
    }
}
impl Display for B {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B{}", self.0)
    }
}
impl Clone for B {
    fn clone(&self) -> B {
        println!("trait clone");
        B(self.0 + 100)
    }
}

trait Get {
    fn get(&self) -> u8;
}
impl Get for B {
    fn get(&self) -> u8 {
        2
    }
}

trait Show: Display {
    fn show(&self) -> String {
        self.to_string()
    }
}
impl Show for B {}

trait Total {
    fn len(&self) -> usize;
    fn total(&self) -> usize {
        self.len() * 2
    }
}
impl Total for B {
    fn len(&self) -> usize {
        3
    }
}

trait Describe {
    fn describe(&self) -> String;
}
impl<T: Display> Describe for Vec<T> {
    fn describe(&self) -> String {
        self.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("|")
    }
}

struct Wrap<T>(T);
impl<T: Display> Wrap<T> {
    fn s(&self) -> String {
        self.0.to_string()
    }
    fn all(&self, v: &[T]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }
}

fn s1<T: Display>(t: &T) -> String {
    t.to_string()
}
fn s2<T: ToString>(t: T) -> String {
    t.to_string()
}
fn s3<T: Display>(t: &T) -> String {
    T::to_string(t)
}
fn sum_get<T: Get>(v: &[T]) -> u32 {
    v.iter().map(|x| x.get() as u32).sum()
}
fn dup<T: Clone>(x: &T) -> T {
    x.clone()
}
fn tot<T: Total>(x: &T) -> (usize, usize) {
    (x.len(), x.total())
}
macro_rules! show {
    ($e:expr) => {
        $e.to_string()
    };
}
fn via_macro<T: Display>(x: T) -> String {
    show!(x)
}
fn nested<T: Display>(v: Vec<Option<T>>) -> Vec<String> {
    v.into_iter()
        .flatten()
        .map(|x| {
            let f = |y: &T| y.to_string();
            f(&x)
        })
        .collect()
}

fn main() {
    println!("{} {} {}", s1(&B(1)), s2(B(2)), s3(&B(3)));
    println!("{}", B(4).show());
    let w = Wrap(B(5));
    println!("{} {:?}", w.s(), w.all(&[B(6), B(7)]));
    println!("{}", vec![B(8), B(9)].describe());
    println!("{}", sum_get(&[B(0), B(0)]));
    println!("{}", dup(&B(10)).0);
    println!("{:?}", tot(&B(0)));
    println!("{}", via_macro(B(11)));
    println!("{:?}", nested(vec![Some(B(12)), None, Some(B(13))]));
    // outside generic bodies, inherent methods win
    println!("{} {} {} {}", B(0).to_string(), B(0).get(), B(0).len(), B(0).clone().0);
}
