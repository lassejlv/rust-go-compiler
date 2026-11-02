// Higher-ranked trait bounds in library-style APIs.
use std::collections::BTreeMap;

struct Record {
    name: String,
    tags: Vec<String>,
    score: i32,
}

fn project<'r, F>(records: &'r [Record], f: F) -> Vec<&'r str>
where
    F: for<'a> Fn(&'a Record) -> &'a str,
{
    records.iter().map(|r| f(r)).collect()
}

fn get_name(r: &Record) -> &str {
    &r.name
}

fn first_tag(r: &Record) -> &str {
    r.tags.first().map(|s| s.as_str()).unwrap_or("-")
}

fn for_each_word<F>(text: &str, mut f: F)
where
    F: for<'a> FnMut(&'a str),
{
    for w in text.split_whitespace() {
        f(w);
    }
}

// A boxed higher-ranked callback stored in a struct
struct Extractor {
    label: &'static str,
    func: Box<dyn for<'a> Fn(&'a str) -> Option<&'a str>>,
}

impl Extractor {
    fn run<'s>(&self, s: &'s str) -> Option<&'s str> {
        (self.func)(s)
    }
}

// Trait with lifetime parameter used under HRTB
trait Parser<'a> {
    type Out;
    fn parse(&self, s: &'a str) -> Option<(Self::Out, &'a str)>;
}

struct Digits;
impl<'a> Parser<'a> for Digits {
    type Out = &'a str;
    fn parse(&self, s: &'a str) -> Option<(&'a str, &'a str)> {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        if end == 0 {
            None
        } else {
            Some((&s[..end], &s[end..]))
        }
    }
}

struct Ident;
impl<'a> Parser<'a> for Ident {
    type Out = String;
    fn parse(&self, s: &'a str) -> Option<(String, &'a str)> {
        let end = s.find(|c: char| !c.is_alphabetic()).unwrap_or(s.len());
        if end == 0 {
            None
        } else {
            Some((s[..end].to_uppercase(), &s[end..]))
        }
    }
}

fn parse_all<P>(p: &P, inputs: &[&str]) -> usize
where
    P: for<'a> Parser<'a>,
{
    let mut ok = 0;
    for inp in inputs {
        if let Some((_, rest)) = p.parse(inp) {
            ok += 1;
            println!("  {:?} -> rest {:?}", inp, rest);
        }
    }
    ok
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn apply_to_pairs<F>(map: &BTreeMap<String, i32>, f: F) -> Vec<String>
where
    F: for<'k, 'v> Fn(&'k str, &'v i32) -> String,
{
    map.iter().map(|(k, v)| f(k, v)).collect()
}

fn main() {
    let records = vec![
        Record { name: "alpha".into(), tags: vec!["x".into(), "y".into()], score: 3 },
        Record { name: "beta".into(), tags: vec![], score: 9 },
        Record { name: "gamma".into(), tags: vec!["z".into()], score: -1 },
    ];
    println!("{:?}", project(&records, get_name));
    println!("{:?}", project(&records, first_tag));
    println!("{:?}", project(&records, |r: &Record| if r.score > 0 { r.name.as_str() } else { "neg" }));

    let mut longest = String::new();
    let mut count = 0;
    for_each_word("the quick brown fox jumped", |w| {
        count += 1;
        if w.len() > longest.len() {
            longest = w.to_string();
        }
    });
    println!("{} words, longest {:?}", count, longest);

    let exs = vec![
        Extractor { label: "after-colon", func: Box::new(|s| s.split_once(':').map(|(_, b)| b.trim())) },
        Extractor { label: "first-word", func: Box::new(|s| s.split_whitespace().next()) },
        Extractor { label: "ext", func: Box::new(|s| s.rsplit_once('.').map(|p| p.1)) },
    ];
    for inp in ["key: value here", "file.tar.gz", ""] {
        for e in &exs {
            println!("{:>12}({:?}) = {:?}", e.label, inp, e.run(inp));
        }
    }

    println!("digits ok = {}", parse_all(&Digits, &["123abc", "x1", "42"]));
    println!("ident ok = {}", parse_all(&Ident, &["abc123", "1x", "Zed"]));

    let f = compose(|x: i32| x * 3, |y: i32| format!("<{}>", y));
    let g = compose(|s: &str| s.len(), |n: usize| n % 2 == 0);
    println!("{} {} {}", f(7), g("four"), g("odd"));

    let mut m = BTreeMap::new();
    m.insert("b".to_string(), 2);
    m.insert("a".to_string(), 1);
    println!("{:?}", apply_to_pairs(&m, |k, v| format!("{}={}", k, v * 10)));
}
