use std::fmt::Display;

trait Shout {
    fn shout(&self) -> String;
}

impl<T: Display + ?Sized> Shout for T {
    fn shout(&self) -> String {
        format!("{}!", self.to_string().to_uppercase())
    }
}

trait Describe {
    fn describe(&self) -> String;
}

impl<T: std::fmt::Debug> Describe for Vec<T> {
    fn describe(&self) -> String {
        format!("Vec of {} items: {:?}", self.len(), self)
    }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> String {
        match self {
            Some(x) => format!("Some[{}]", x.describe()),
            None => "Nothing".to_string(),
        }
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

trait Summary {
    fn summarize_author(&self) -> String;
    fn summarize(&self) -> String {
        format!("(Read more from {}...)", self.summarize_author())
    }
}

struct Tweet {
    username: String,
}

impl Summary for Tweet {
    fn summarize_author(&self) -> String {
        format!("@{}", self.username)
    }
}

trait Loud: Summary {
    fn loud(&self) -> String {
        self.summarize().shout()
    }
}
impl<T: Summary> Loud for T {}

fn main() {
    println!("{}", "hello".shout());
    println!("{}", 42.shout());
    println!("{}", 'x'.shout());
    let s = String::from("mixed Case");
    println!("{}", s.shout());
    println!("{}", vec![1, 2, 3].describe());
    println!("{}", Some(vec!["a", "b"]).describe());
    let none: Option<Vec<u8>> = None;
    println!("{}", none.describe());
    println!("{}", Some(Some(vec![1.5])).describe());
    println!("{}", 21.double());
    println!("{}", 1.25f64.double());
    println!("{}", String::from("ab").double_s());
    let t = Tweet { username: "rustlang".into() };
    println!("{}", t.summarize());
    println!("{}", t.loud());
}

trait DoubleS {
    fn double_s(&self) -> String;
}
impl DoubleS for String {
    fn double_s(&self) -> String {
        self.clone() + self
    }
}
