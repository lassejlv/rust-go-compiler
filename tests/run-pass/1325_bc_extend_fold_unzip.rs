// Items stored by extend (into locals and into `&mut` parameters),
// accumulated by fold, split by unzip/partition: only the parts the
// impls and closures declare flow.
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug)]
struct Token<'a> {
    w: &'a str,
}

struct Doc<'a> {
    words: Vec<&'a str>,
    toks: Vec<Token<'a>>,
}

impl<'a> Doc<'a> {
    fn add_all(&mut self, xs: &[&'a str]) {
        self.words.extend(xs.iter().copied());
    }
    fn add_text(&mut self, t: &'a str) {
        self.toks.extend(t.split(' ').map(|w| Token { w }));
        self.words.extend(t.lines());
    }
}

fn keys_into_local<'a>(m: &HashMap<&'static str, &'a str>) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend(m.keys().copied());
    out
}

fn second_into_set<'a>(v: &[(&'a str, &'static str)]) -> HashSet<&'static str> {
    let mut s = HashSet::new();
    s.extend(v.iter().map(|p| p.1));
    s
}

fn counts<'a>(v: &[(&'a str, &'static str)]) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::new();
    m.extend(v.iter().map(|p| (p.1, p.0.len())));
    m
}

fn into_param<'a>(out: &mut Vec<&'a str>, s: &'a str, v: &'a [String]) {
    out.extend(s.split(','));
    out.extend(v.iter().map(|x| x.as_str()));
    out.extend([s]);
}

fn pairs_into_param<'a>(out: &mut BTreeMap<&'a str, &'static str>, v: &[(&'a str, &'static str)]) {
    out.extend(v.iter().copied());
}

fn statics_into_param<'a>(out: &mut HashSet<&'static str>, v: &[(&'a str, &'static str)]) {
    out.extend(v.iter().map(|p| p.1));
}

fn fold_seconds<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().fold(Vec::new(), |mut a, p| {
        a.push(p.1);
        a
    })
}

fn fold_pick<'a>(v: &[(&'a str, &'static str)]) -> &'static str {
    v.iter().fold("init", |acc, p| if p.0.is_empty() { acc } else { p.1 })
}

fn fold_longest<'a>(v: &'a [String]) -> &'a str {
    v.iter().fold("", |best, s| if s.len() > best.len() { s.as_str() } else { best })
}

fn unzip_first<'a>(v: Vec<(&'static str, &'a str)>) -> Vec<&'static str> {
    let (a, _b): (Vec<&'static str>, Vec<&'a str>) = v.into_iter().unzip();
    a
}

fn partition_static<'a>(v: Vec<(&'static str, &'a str)>) -> Vec<(&'static str, &'a str)> {
    let (a, _b): (Vec<_>, Vec<_>) = v.into_iter().partition(|p| p.1.is_empty());
    a
}

fn main() {
    let owned = String::from("dyn");
    let o = owned.as_str();
    let mut hm = HashMap::new();
    hm.insert("k", o);
    let v: Vec<(&str, &'static str)> = vec![(o, "x"), ("", "y")];
    let w: Vec<(&'static str, &str)> = vec![("a", o), ("b", "")];

    let r1 = keys_into_local(&hm);
    let r2 = second_into_set(&v);
    let r3 = counts(&v);
    let r4 = fold_seconds(&v);
    let r5 = fold_pick(&v);
    let r6 = unzip_first(w.clone());
    let r7 = partition_static(w.clone());
    let mut set = HashSet::new();
    statics_into_param(&mut set, &v);
    println!("{:?}", r7);
    drop(r7);
    drop(hm);
    drop(v);
    drop(w);
    drop(owned);
    let mut r2: Vec<_> = r2.into_iter().collect();
    r2.sort();
    let mut set: Vec<_> = set.into_iter().collect();
    set.sort();
    println!("{:?} {:?} {:?} {:?} {:?} {:?} {:?}", r1, r2, r3, r4, r5, r6, set);

    let text = String::from("a,b");
    let strings = vec![String::from("abc"), String::from("de")];
    let mut out = Vec::new();
    into_param(&mut out, &text, &strings);
    println!("{:?} {}", out, fold_longest(&strings));
    let mut bm = BTreeMap::new();
    pairs_into_param(&mut bm, &[(text.as_str(), "t")]);
    println!("{:?}", bm);

    let mut d = Doc { words: vec![], toks: vec![] };
    d.add_all(&["w"]);
    d.add_text("x y");
    println!("{:?} {:?}", d.words, d.toks);
}
