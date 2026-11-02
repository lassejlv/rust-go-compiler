use std::fmt::Display;

trait Summary {
    fn summarize(&self) -> String;
}

impl Summary for i32 {
    fn summarize(&self) -> String {
        format!("int {}", self)
    }
}

impl Summary for str {
    fn summarize(&self) -> String {
        format!("str of {} bytes", self.len())
    }
}

impl<T: Summary> Summary for Vec<T> {
    fn summarize(&self) -> String {
        let parts: Vec<String> = self.iter().map(|x| x.summarize()).collect();
        format!("vec[{}]", parts.join("; "))
    }
}

impl<T: Summary> Summary for Option<T> {
    fn summarize(&self) -> String {
        match self {
            Some(x) => format!("some({})", x.summarize()),
            None => "none".to_string(),
        }
    }
}

trait Loud {
    fn loud(&self) -> String;
}

impl<T: Display> Loud for T {
    fn loud(&self) -> String {
        format!("{}!!", self.to_string().to_uppercase())
    }
}

trait Doubled {
    fn doubled(&self) -> Self;
}

impl<T: Clone + std::ops::Add<Output = T>> Doubled for T {
    fn doubled(&self) -> T {
        self.clone() + self.clone()
    }
}

trait Ext {
    fn is_palindrome(&self) -> bool;
    fn word_count(&self) -> usize;
}

impl Ext for String {
    fn is_palindrome(&self) -> bool {
        let cs: Vec<char> = self.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect();
        cs.iter().eq(cs.iter().rev())
    }
    fn word_count(&self) -> usize {
        self.split_whitespace().count()
    }
}

fn main() {
    println!("{}", 5.summarize());
    println!("{}", "hello".summarize());
    println!("{}", vec![1, 2, 3].summarize());
    println!("{}", Some(7).summarize());
    println!("{}", vec![Some(1), None].summarize());
    println!("{}", None::<i32>.summarize());
    println!("{}", "quiet".loud());
    println!("{}", 42.loud());
    println!("{}", 'x'.loud());
    println!("{}", 21.doubled());
    println!("{}", 1.25.doubled());
    println!("{}", String::from("ab").doubled2());
    let s = String::from("A man, a plan, a canal: Panama");
    println!("palindrome: {} words: {}", s.is_palindrome(), s.word_count());
    println!("palindrome: {}", String::from("rust").is_palindrome());
}

trait Doubled2 {
    fn doubled2(&self) -> String;
}

impl Doubled2 for String {
    fn doubled2(&self) -> String {
        self.clone() + self
    }
}
