// A string interner returning Copy symbols, a la lasso / string-interner / SharedString.
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Symbol(u32);

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Default)]
struct Interner {
    map: HashMap<Rc<str>, Symbol>,
    strings: Vec<Rc<str>>,
}

impl Interner {
    fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }
        let sym = Symbol(self.strings.len() as u32);
        let rc: Rc<str> = Rc::from(s);
        self.strings.push(rc.clone());
        self.map.insert(rc, sym);
        sym
    }
    fn get(&self, s: &str) -> Option<Symbol> {
        self.map.get(s).copied()
    }
    fn resolve(&self, sym: Symbol) -> &str {
        &self.strings[sym.0 as usize]
    }
    fn len(&self) -> usize {
        self.strings.len()
    }
}

// SharedString: either a &'static str or an Arc<str>, cheap to clone
#[derive(Clone)]
enum SharedString {
    Static(&'static str),
    Owned(Arc<str>),
}

impl SharedString {
    fn as_str(&self) -> &str {
        match self {
            SharedString::Static(s) => s,
            SharedString::Owned(s) => s,
        }
    }
}

impl From<&'static str> for SharedString {
    fn from(s: &'static str) -> Self {
        SharedString::Static(s)
    }
}
impl From<String> for SharedString {
    fn from(s: String) -> Self {
        SharedString::Owned(Arc::from(s))
    }
}
impl std::ops::Deref for SharedString {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq for SharedString {
    fn eq(&self, o: &Self) -> bool {
        self.as_str() == o.as_str()
    }
}
impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Debug for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            SharedString::Static(_) => "static",
            SharedString::Owned(_) => "owned",
        };
        write!(f, "{}:{:?}", kind, self.as_str())
    }
}

fn tokenize(interner: &mut Interner, src: &str) -> Vec<Symbol> {
    src.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(|w| interner.intern(w)).collect()
}

fn main() {
    let mut i = Interner::default();
    let toks = tokenize(&mut i, "let x = y + x * (y - z); let z = x;");
    println!("{:?}", toks);
    println!("unique={}", i.len());
    let mut freq = vec![0usize; i.len()];
    for t in &toks {
        freq[t.0 as usize] += 1;
    }
    for (idx, n) in freq.iter().enumerate() {
        println!("  {:>3} {:<4} x{}", idx, i.resolve(Symbol(idx as u32)), n);
    }
    println!("{:?} {:?}", i.get("let"), i.get("missing"));
    let again = i.intern("x");
    println!("x again = {:?}, same = {}", again, again == toks[1]);

    let mut sorted: Vec<Symbol> = toks.clone();
    sorted.sort_by(|a, b| i.resolve(*a).cmp(i.resolve(*b)));
    sorted.dedup();
    let words: Vec<&str> = sorted.iter().map(|s| i.resolve(*s)).collect();
    println!("{:?}", words);

    let a: SharedString = "hello".into();
    let b: SharedString = format!("hel{}", "lo").into();
    let c = b.clone();
    println!("{:?} {:?} eq={} len={} upper={}", a, c, a == b, c.len(), c.to_uppercase());
    let list: Vec<SharedString> = vec!["x".into(), String::from("y").into(), "z".into()];
    let joined: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    println!("{} starts_with_x={}", joined.join(","), list[0].starts_with('x'));
    if let SharedString::Owned(arc) = &b {
        println!("arc strong count = {}", Arc::strong_count(arc));
    }
}
