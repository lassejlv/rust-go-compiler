use std::collections::BTreeMap;

type Splitter = Box<dyn for<'a> Fn(&'a str) -> Box<dyn Iterator<Item = &'a str> + 'a>>;

fn splitters() -> BTreeMap<&'static str, Splitter> {
    let mut m: BTreeMap<&'static str, Splitter> = BTreeMap::new();
    m.insert("ws", Box::new(|s| Box::new(s.split_whitespace())));
    m.insert("comma", Box::new(|s| Box::new(s.split(',').map(str::trim))));
    m.insert("chars2", Box::new(|s| Box::new((0..s.len()).step_by(2).map(move |i| &s[i..(i + 2).min(s.len())]))));
    m
}

trait Transformer { fn transform<'a>(&self, s: &'a str) -> &'a str; }
impl<F> Transformer for F where F: for<'a> Fn(&'a str) -> &'a str { fn transform<'a>(&self, s: &'a str) -> &'a str { self(s) } }
struct TrimStart(char);
impl Transformer for TrimStart { fn transform<'a>(&self, s: &'a str) -> &'a str { s.trim_start_matches(self.0) } }

fn chain_all<'t>(ts: &'t [&'t dyn Transformer]) -> impl for<'a> Fn(&'a str) -> &'a str + 't {
    move |s| ts.iter().fold(s, |acc, t| t.transform(acc))
}

fn measure<T, R, F>(items: &[T], f: F) -> Vec<R> where F: for<'a> Fn(&'a T) -> R { items.iter().map(&f).collect() }

fn trim_bang(s: &str) -> &str { s.trim_end_matches('!') }
fn first_two<'a>(s: &'a str) -> &'a str { &s[..2.min(s.len())] }

struct Registry<'h> { hooks: Vec<(&'static str, Box<dyn Fn(&mut Vec<String>, &str) + 'h>)> }
impl<'h> Registry<'h> {
    fn on(&mut self, name: &'static str, f: impl Fn(&mut Vec<String>, &str) + 'h) -> &mut Self { self.hooks.push((name, Box::new(f))); self }
    fn fire(&self, arg: &str) -> Vec<String> {
        let mut log = vec![];
        for (n, h) in &self.hooks { log.push(format!("[{}]", n)); h(&mut log, arg); }
        log
    }
}

fn main() {
    let sp = splitters();
    let text = String::from("a, bb ,ccc");
    for (name, f) in &sp { println!("{} {:?}", name, f(&text).collect::<Vec<_>>()); }
    let trim_x = TrimStart('x');
    let ts: [&dyn Transformer; 3] = [&trim_x, &first_two, &trim_bang];
    let c = chain_all(&ts);
    let owned = String::from("xxhello!");
    println!("{:?} {:?}", c(&owned), c("xa!"));
    println!("{:?}", measure(&["abc".to_string(), "de".to_string()], |s| s.len()));
    println!("{:?}", measure(&[(1, 'a'), (2, 'b')], |p: &(i32, char)| p.1));
    let suffix = String::from("!");
    let mut reg = Registry { hooks: vec![] };
    reg.on("upper", |log, a| log.push(a.to_uppercase())).on("suffix", |log, a| log.push(format!("{}{}", a, suffix)));
    println!("{:?}", reg.fire("hey"));
}
