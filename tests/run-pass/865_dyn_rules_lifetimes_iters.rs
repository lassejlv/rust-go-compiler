use std::fmt::Debug;
trait Rule<'a>: Debug { fn check(&self, s: &'a str) -> Option<&'a str>; }
#[derive(Debug)] struct Prefix<'p>(&'p str);
#[derive(Debug)] struct MinLen(usize);
impl<'a, 'p> Rule<'a> for Prefix<'p> { fn check(&self, s: &'a str) -> Option<&'a str> { s.strip_prefix(self.0) } }
impl<'a> Rule<'a> for MinLen { fn check(&self, s: &'a str) -> Option<&'a str> { if s.len() >= self.0 { Some(s) } else { None } } }
struct Engine<'r, 'a> { rules: Vec<Box<dyn Rule<'a> + 'r>> }
impl<'r, 'a> Engine<'r, 'a> {
    fn run(&self, inputs: &[&'a str]) -> Vec<(&'a str, Vec<Option<&'a str>>)> { inputs.iter().map(|&s| (s, self.rules.iter().map(|r| r.check(s)).collect())).collect() }
    fn first_match(&self, s: &'a str) -> Option<&'a str> { self.rules.iter().find_map(|r| r.check(s)) }
}
fn make_filter<'a, 'b>(words: &'a [&'b str], min: usize) -> impl Iterator<Item = &'b str> + 'a where 'b: 'a { words.iter().copied().filter(move |w| w.len() >= min) }
fn boxed_iter<'a>(v: &'a [i32], rev: bool) -> Box<dyn Iterator<Item = &'a i32> + 'a> { if rev { Box::new(v.iter().rev()) } else { Box::new(v.iter()) } }
fn apply<'a>(fs: &[&dyn Fn(&'a str) -> &'a str], s: &'a str) -> Vec<&'a str> { fs.iter().map(|f| f(s)).collect() }
fn main() {
    let pfx = String::from("ab");
    let e = Engine { rules: vec![Box::new(Prefix(&pfx)), Box::new(MinLen(3)), Box::new(Prefix("x"))] };
    let inputs = ["abc", "xy", "abxyz", "q"];
    for (s, r) in e.run(&inputs) { println!("{} {:?}", s, r); }
    println!("{:?}", inputs.iter().map(|s| e.first_match(s)).collect::<Vec<_>>());
    println!("{:?}", e.rules);
    let ws = ["a", "abc", "abcd", "xy"];
    println!("{:?}", make_filter(&ws, 3).collect::<Vec<_>>());
    println!("{:?}", boxed_iter(&[1, 2, 3], true).collect::<Vec<_>>());
    let trim = |s: &'static str| s.trim();
    let f1: &dyn Fn(&str) -> &str = &|s| &s[1..];
    println!("{:?}", apply(&[f1, &|s| s.trim_end_matches('!')], "!hi!!"));
    println!("{}", trim("  t  "));
}
