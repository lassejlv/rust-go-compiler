// Valid borrow patterns involving function pointers, dyn Fn objects, trait
// object lifetime defaults, closures returning references, and HRTB
// callbacks. All of these compile with rustc; results are printed.
use std::collections::HashMap;

trait Shape { fn area(&self) -> f64; fn label(&self) -> String; }
struct Sq(f64);
struct Named<'a> { name: &'a str, side: f64 }
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } fn label(&self) -> String { format!("sq{}", self.0) } }
impl<'a> Shape for Named<'a> { fn area(&self) -> f64 { self.side * 2.0 } fn label(&self) -> String { self.name.to_string() } }

type Maker = fn(&str) -> Box<dyn Shape>;
type DynMaker = Box<dyn Fn(&str) -> Box<dyn Shape>>;
type Picker = for<'a> fn(&'a [i32]) -> &'a i32;

fn make_sq(s: &str) -> Box<dyn Shape> { Box::new(Sq(s.len() as f64)) }
fn first(v: &[i32]) -> &i32 { &v[0] }
fn largest(v: &[i32]) -> &i32 { v.iter().max().unwrap() }

struct Registry<'a> {
    shapes: Vec<Box<dyn Shape + 'a>>,
    makers: Vec<Maker>,
    dyn_makers: Vec<DynMaker>,
}

impl<'a> Registry<'a> {
    fn build_all(&mut self, arg: &str) -> usize {
        let mut n = 0;
        for m in &self.makers {
            let s = m(arg);
            self.shapes.push(s);
            n += 1;
        }
        for m in self.dyn_makers.iter() {
            self.shapes.push(m(arg));
            n += 1;
        }
        n
    }
    fn add_named(&mut self, name: &'a str, side: f64) { self.shapes.push(Box::new(Named { name, side })); }
    fn best(&self) -> Option<&(dyn Shape + 'a)> {
        self.shapes.iter().map(|b| b.as_ref()).max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap())
    }
}

fn apply_picker(p: Picker, data: &[i32]) -> i32 { *p(data) }

fn with_callback<F>(data: &[String], f: F) -> Vec<usize> where F: for<'x> Fn(&'x str) -> &'x str {
    data.iter().map(|s| f(s).len()).collect()
}

fn cache_lookup<'m>(cache: &'m mut HashMap<u32, String>, k: u32) -> &'m String {
    if !cache.contains_key(&k) {
        cache.insert(k, format!("v{}", k * k));
    }
    cache.get(&k).unwrap()
}

fn get_or_default<'m>(map: &'m mut HashMap<&'static str, Vec<i32>>, k: &'static str) -> &'m mut Vec<i32> {
    map.entry(k).or_default()
}

struct Parser<'s> { src: &'s str, pos: usize }
impl<'s> Parser<'s> {
    fn next_word(&mut self) -> Option<&'s str> {
        let rest = &self.src[self.pos..];
        let rest_trim = rest.trim_start();
        if rest_trim.is_empty() { return None; }
        let start = self.pos + (rest.len() - rest_trim.len());
        let end = rest_trim.find(' ').map(|i| start + i).unwrap_or(self.src.len());
        self.pos = end;
        Some(&self.src[start..end])
    }
}

fn longest_word<'s>(p: &mut Parser<'s>) -> &'s str {
    let mut best = "";
    while let Some(w) = p.next_word() {
        if w.len() > best.len() { best = w; }
    }
    best
}

fn main() {
    let names = vec!["alpha".to_string(), "be".to_string()];
    let mut reg = Registry { shapes: vec![], makers: vec![make_sq, |s| Box::new(Sq(s.len() as f64 / 2.0))], dyn_makers: vec![] };
    let k = 3.0;
    reg.dyn_makers.push(Box::new(move |s: &str| Box::new(Sq(k + s.len() as f64)) as Box<dyn Shape>));
    let arg = String::from("four");
    println!("built {}", reg.build_all(&arg));
    for n in &names { reg.add_named(n, n.len() as f64 * 10.0); }
    let labels: Vec<String> = reg.shapes.iter().map(|s| format!("{}={}", s.label(), s.area())).collect();
    println!("{}", labels.join(" "));
    println!("best: {:?}", reg.best().map(|s| s.label()));

    let data = [4, 9, 2, 7];
    let pickers: [(&str, Picker); 2] = [("first", first), ("largest", largest)];
    for (n, p) in pickers { print!("{}={} ", n, apply_picker(p, &data)); }
    println!();
    let r: &i32 = { let p: Picker = largest; p(&data) };
    println!("picked ref {}", r);

    let words = vec!["  padded ".to_string(), "x".to_string(), "\ttab\t".to_string()];
    println!("trimmed lens {:?}", with_callback(&words, str::trim));
    println!("first-half lens {:?}", with_callback(&words, |s| &s[..s.len() / 2]));

    let mut cache = HashMap::new();
    let a = cache_lookup(&mut cache, 7).clone();
    let b = cache_lookup(&mut cache, 7).len();
    let c = cache_lookup(&mut cache, 12).clone();
    println!("cache {} {} {} size={}", a, b, c, cache.len());

    let mut groups: HashMap<&'static str, Vec<i32>> = HashMap::new();
    for (i, key) in ["odd", "even", "odd", "odd", "even"].iter().enumerate() {
        get_or_default(&mut groups, key).push(i as i32);
    }
    let v = get_or_default(&mut groups, "odd");
    v.retain(|x| *x > 0);
    let mut keys: Vec<_> = groups.iter().map(|(k, v)| format!("{}:{:?}", k, v)).collect();
    keys.sort();
    println!("groups {:?}", keys);

    let text = String::from("the quick brownish fox jumped");
    let lw;
    {
        let mut p = Parser { src: &text, pos: 0 };
        lw = longest_word(&mut p);
    }
    println!("longest word: {}", lw);

    let picker_closure = |v: &Vec<i32>| -> Option<i32> { v.iter().copied().filter(|x| x % 2 == 1).last() };
    let fn_ptr: fn(&Vec<i32>) -> Option<i32> = picker_closure;
    println!("closure as fn ptr: {:?}", fn_ptr(&vec![1, 2, 3, 4, 5, 6]));
    let ops: Vec<(&str, fn(i64, i64) -> i64)> = vec![("add", |a, b| a + b), ("max", i64::max), ("pow", |a, b| a.pow(b as u32))];
    println!("{}", ops.iter().map(|(n, f)| format!("{}={}", n, f(3, 4))).collect::<Vec<_>>().join(","));
}
