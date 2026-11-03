fn both<T>(t: T) -> usize
where
    T: AsRef<str> + AsRef<[u8]>,
{
    let s: &str = t.as_ref();
    let b: &[u8] = t.as_ref();
    s.len() + b.len()
}

fn cmp2<T>(t: T, a: u32, b: &str) -> (bool, bool)
where
    T: PartialEq<u32> + PartialEq<&'static str>,
{
    let _ = b;
    (t.eq(&a), t.ne(&"x"))
}

#[derive(PartialEq)]
struct W(u32);
impl PartialEq<u32> for W {
    fn eq(&self, o: &u32) -> bool {
        self.0 == *o
    }
}
impl PartialEq<&'static str> for W {
    fn eq(&self, o: &&'static str) -> bool {
        o.len() as u32 == self.0
    }
}

fn ext<C>(mut c: C) -> C
where
    C: Extend<&'static str> + Extend<char>,
{
    c.extend(vec!["ab"]);
    c.extend("cd".chars());
    c
}

fn conv<T: Copy>(t: T) -> (i64, f64)
where
    T: Into<i64> + Into<f64>,
{
    let a = t.into();
    let b = t.into();
    takes(a, b)
}

fn takes(a: i64, b: f64) -> (i64, f64) {
    (a, b)
}

fn main() {
    println!("{}", both(String::from("hey")));
    println!("{:?}", cmp2(W(3), 3, "x"));
    println!("{}", ext(String::new()));
    println!("{:?}", conv(5i32));
}
