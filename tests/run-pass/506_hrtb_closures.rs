// Higher-ranked trait bounds: for<'a> Fn(&'a T) -> &'a U
fn apply_to_all<F>(items: &[String], f: F) -> Vec<&str>
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    items.iter().map(|s| f(s.as_str())).collect()
}

fn first_word(s: &str) -> &str {
    s.split(' ').next().unwrap_or("")
}

struct Matcher<F>
where
    F: for<'x> Fn(&'x [i32]) -> Option<&'x i32>,
{
    pick: F,
    name: &'static str,
}

impl<F> Matcher<F>
where
    F: for<'x> Fn(&'x [i32]) -> Option<&'x i32>,
{
    fn run<'s>(&self, data: &'s [i32]) -> String {
        match (self.pick)(data) {
            Some(v) => format!("{}: {}", self.name, v),
            None => format!("{}: none", self.name),
        }
    }
}

trait Visit {
    fn visit<'a>(&self, s: &'a str) -> Vec<&'a str>;
}
impl<F> Visit for F
where
    F: for<'a> Fn(&'a str) -> Vec<&'a str>,
{
    fn visit<'a>(&self, s: &'a str) -> Vec<&'a str> {
        self(s)
    }
}

fn run_visitors(vs: &[&dyn Visit], text: &str) {
    for (i, v) in vs.iter().enumerate() {
        println!("v{} {:?}", i, v.visit(text));
    }
}

fn call_with_local<F>(f: F) -> usize
where
    F: for<'a> Fn(&'a Vec<u8>) -> usize,
{
    let local = vec![1u8, 2, 3, 4, 5];
    let tmp = vec![9u8; 3];
    f(&local) + f(&tmp)
}

fn boxed_hrtb() -> Box<dyn for<'a> Fn(&'a str, &'a str) -> &'a str> {
    Box::new(|a, b| if a.len() >= b.len() { a } else { b })
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn fold_refs<'d, T, F>(data: &'d [T], init: &'d T, f: F) -> &'d T
where
    F: for<'a> Fn(&'a T, &'a T) -> &'a T,
{
    data.iter().fold(init, |acc, x| f(acc, x))
}

fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}
fn caps(s: &str) -> Vec<&str> {
    s.split_whitespace().filter(|w| w.chars().next().map_or(false, char::is_uppercase)).collect()
}
fn fixed(s: &str) -> Vec<&str> {
    vec![&s[..1], &s[s.len() - 1..]]
}

fn main() {
    let items = vec!["hello world".to_string(), "  trim me ".to_string(), "single".to_string()];
    println!("{:?}", apply_to_all(&items, first_word));
    println!("{:?}", apply_to_all(&items, |s| s.trim()));
    println!("{:?}", apply_to_all(&items, |s| &s[s.len() / 2..]));

    let m1 = Matcher { pick: |d: &[i32]| d.iter().max(), name: "max" };
    let m2 = Matcher { pick: |d: &[i32]| d.iter().find(|x| **x < 0), name: "neg" };
    let m3 = Matcher { pick: |d: &[i32]| d.last(), name: "last" };
    let data = [3, 9, -2, 7];
    println!("{} | {} | {}", m1.run(&data), m2.run(&data), m3.run(&data));
    println!("{}", m2.run(&[1, 2]));

    run_visitors(&[&words, &caps, &fixed], "Alpha beta Gamma delta");

    println!("{}", call_with_local(|v| v.len()));
    println!("{}", call_with_local(|v| v.iter().map(|&x| x as usize).sum()));
    let longer = boxed_hrtb();
    let s1 = String::from("short");
    let r;
    {
        let s2 = String::from("much longer");
        r = longer(&s1, &s2).to_string();
    }
    println!("{}", r);
    let f = compose(|x: i32| x + 1, |y: i32| format!("<{}>", y));
    let g = compose(&f, |s: String| s.len());
    println!("{} {}", f(41), g(1000));
    let nums = [4, 17, 8, 23, 1];
    let zero = 0;
    println!("{}", fold_refs(&nums, &zero, |a, b| if b > a { b } else { a }));
    let strs = ["pear".to_string(), "fig".to_string(), "banana".to_string()];
    let empty = String::new();
    println!("{:?}", fold_refs(&strs, &empty, |a, b| if b.len() > a.len() { b } else { a }));
}
