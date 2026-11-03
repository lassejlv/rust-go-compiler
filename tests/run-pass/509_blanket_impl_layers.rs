// blanket impls interacting with concrete impls, references, and Box
use std::fmt::{self, Display};

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Display> Describe for T {
    fn describe(&self) -> String {
        format!("«{}»", self)
    }
}

trait Loud: Describe {
    fn loud(&self) -> String {
        self.describe().to_uppercase()
    }
}
impl<T: Describe + ?Sized> Loud for T {}

struct Celsius(f64);
impl Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}°c", self.0)
    }
}

trait Summary {
    fn summarize_author(&self) -> String;
    fn summarize(&self) -> String {
        format!("(Read more from {}...)", self.summarize_author())
    }
}
impl<T: Summary + ?Sized> Summary for Box<T> {
    fn summarize_author(&self) -> String {
        format!("boxed {}", (**self).summarize_author())
    }
}
impl<T: Summary> Summary for &T {
    fn summarize_author(&self) -> String {
        format!("ref {}", (*self).summarize_author())
    }
}
impl<T: Summary> Summary for Vec<T> {
    fn summarize_author(&self) -> String {
        self.iter().map(|x| x.summarize_author()).collect::<Vec<_>>().join("+")
    }
    fn summarize(&self) -> String {
        format!("{} items by {}", self.len(), self.summarize_author())
    }
}
struct Tweet {
    user: String,
}
impl Summary for Tweet {
    fn summarize_author(&self) -> String {
        format!("@{}", self.user)
    }
}

trait Double {
    fn double(&self) -> Self;
}
impl<T: Clone + std::ops::Add<Output = T>> Double for T {
    fn double(&self) -> T {
        self.clone() + self.clone()
    }
}

trait IntoPairs<K, V> {
    fn pairs(self) -> Vec<(K, V)>;
}
impl<K: Clone, V, I: IntoIterator<Item = V>> IntoPairs<K, V> for (K, I) {
    fn pairs(self) -> Vec<(K, V)> {
        let k = self.0;
        self.1.into_iter().map(|v| (k.clone(), v)).collect()
    }
}

#[derive(Clone, Debug)]
struct V2(i32, i32);
impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2(self.0 + o.0, self.1 + o.1)
    }
}

fn show_all(xs: &[&dyn Loud]) -> Vec<String> {
    xs.iter().map(|x| x.loud()).collect()
}

fn main() {
    println!("{}", 42.describe());
    println!("{}", "abc".describe());
    println!("{}", Celsius(21.456).describe());
    println!("{}", Celsius(-3.0).loud());
    println!("{}", 'x'.loud());
    let s = String::from("mixed Case");
    println!("{:?}", show_all(&[&1.5f32, &s, &Celsius(0.04), &true]));

    let t = Tweet { user: "rust".into() };
    println!("{}", t.summarize());
    println!("{}", (&t).summarize());
    let b: Box<Tweet> = Box::new(Tweet { user: "go".into() });
    println!("{}", b.summarize());
    let bd: Box<dyn Summary> = Box::new(Tweet { user: "dyn".into() });
    println!("{}", bd.summarize());
    let v = vec![Tweet { user: "a".into() }, Tweet { user: "b".into() }];
    println!("{}", v.summarize());
    let vr = vec![&t, &t];
    println!("{}", vr.summarize());

    println!("{} {} {:?}", 21.double(), 1.25f64.double(), V2(1, -2).double());
    println!("{}", String::from("ab").double_str());
    println!("{:?}", ("k", vec![1, 2, 3]).pairs());
    println!("{:?}", (7u8, "xy".chars()).pairs());
}

trait DoubleStr {
    fn double_str(&self) -> String;
}
impl DoubleStr for String {
    fn double_str(&self) -> String {
        self.repeat(2)
    }
}
