// Blanket impls interacting with dyn: impl<T: Trait + ?Sized> Other for T covering dyn Trait,
// impls for &T / Box<T>, and extension traits applied to trait objects.
use std::fmt::Display;

trait Speak {
    fn speak(&self) -> String;
}

trait Loud {
    fn loud(&self) -> String;
}

// covers sized types and dyn Speak alike
impl<T: Speak + ?Sized> Loud for T {
    fn loud(&self) -> String {
        self.speak().to_uppercase() + "!!"
    }
}

struct Cat;
struct Cow(u8);
impl Speak for Cat {
    fn speak(&self) -> String {
        "meow".into()
    }
}
impl Speak for Cow {
    fn speak(&self) -> String {
        "m".to_string() + &"o".repeat(self.0 as usize)
    }
}

impl<T: Speak + ?Sized> Speak for Box<T> {
    fn speak(&self) -> String {
        format!("boxed({})", (**self).speak())
    }
}
impl<'a, T: Speak + ?Sized> Speak for &'a T {
    fn speak(&self) -> String {
        format!("ref({})", (**self).speak())
    }
}

trait Describe {
    fn describe(&self) -> String;
}
impl<T: Display + ?Sized> Describe for T {
    fn describe(&self) -> String {
        let s = self.to_string();
        format!("<{}:{}>", s.len(), s)
    }
}

trait Summable {
    fn total(&self) -> i64;
}
impl<I> Summable for [I]
where
    I: Copy + Into<i64>,
{
    fn total(&self) -> i64 {
        self.iter().map(|&x| x.into()).sum()
    }
}

fn loud_twice<L: Loud + ?Sized>(l: &L) -> String {
    format!("{} {}", l.loud(), l.loud())
}

fn main() {
    let c = Cat;
    println!("{}", c.loud());
    let d: &dyn Speak = &Cow(3);
    println!("{}", d.loud());
    println!("{}", loud_twice(d));

    let b: Box<dyn Speak> = Box::new(Cat);
    println!("{}", b.speak());
    println!("{}", (*b).speak());
    println!("{}", b.loud());

    let r = &Cow(1);
    println!("{}", r.speak());
    println!("{}", (&r).speak());

    let bb: Box<Box<dyn Speak>> = Box::new(Box::new(Cow(2)));
    println!("{}", bb.speak());

    let animals: Vec<Box<dyn Speak>> = vec![Box::new(Cat), Box::new(Cow(4))];
    let all: Vec<String> = animals.iter().map(|a| a.loud()).collect();
    println!("{:?}", all);

    println!("{}", 42.describe());
    println!("{}", "héllo".describe());
    println!("{}", 3.5f64.describe());
    let disp: &dyn Display = &'x';
    println!("{}", disp.describe());
    let s = String::from("owned");
    println!("{}", s.describe());

    let a: [i32; 4] = [1, -2, 3, 40];
    println!("total i32 = {}", a.total());
    let v: Vec<u8> = vec![200, 100, 50];
    println!("total u8 = {}", v.total());
    println!("total slice = {}", a[1..].total());
}
