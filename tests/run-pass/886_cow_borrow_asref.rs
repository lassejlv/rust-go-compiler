use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashMap};

fn normalize(s: &str) -> Cow<str> {
    if s.chars().any(|c| c.is_uppercase() || c == ' ') {
        Cow::Owned(s.to_lowercase().replace(' ', "_"))
    } else {
        Cow::Borrowed(s)
    }
}

fn dedupe<'a, T: Clone + PartialEq>(v: &'a [T]) -> Cow<'a, [T]> {
    let has_dup = v.windows(2).any(|w| w[0] == w[1]);
    if !has_dup { return Cow::Borrowed(v); }
    let mut out: Vec<T> = Vec::new();
    for x in v {
        if out.last() != Some(x) { out.push(x.clone()); }
    }
    Cow::Owned(out)
}

fn lookup<K, Q, V>(m: &BTreeMap<K, V>, q: &Q) -> Option<V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
    V: Clone,
{
    m.get(q).cloned()
}

fn total_len<S: AsRef<str>>(items: &[S]) -> usize { items.iter().map(|s| s.as_ref().len()).sum() }

fn sum_bytes<B: AsRef<[u8]>>(b: B) -> u32 { b.as_ref().iter().map(|&x| x as u32).sum() }

fn make_owned<T: ToOwned + ?Sized>(t: &T) -> T::Owned { t.to_owned() }

struct Config<'a> { name: Cow<'a, str>, tags: Cow<'a, [&'a str]> }
impl<'a> Config<'a> {
    fn add_tag(&mut self, t: &'a str) { self.tags.to_mut().push(t); }
    fn rename(&mut self, n: String) { self.name = Cow::Owned(n); }
    fn describe(&self) -> String {
        let kind = |c: bool| if c { "borrowed" } else { "owned" };
        format!("{} ({}) tags={:?} ({})", self.name, kind(matches!(self.name, Cow::Borrowed(_))), self.tags, kind(matches!(self.tags, Cow::Borrowed(_))))
    }
}

fn main() {
    for s in ["hello", "Hello World", "x y"] {
        let n = normalize(s);
        let kind = match &n { Cow::Borrowed(_) => "B", Cow::Owned(_) => "O" };
        println!("{} -> {} [{}]", s, n, kind);
    }
    let a = [1, 2, 3];
    let b = [1, 1, 2, 3, 3, 3];
    let da = dedupe(&a);
    let db = dedupe(&b);
    println!("{:?} {} / {:?} {}", da, matches!(da, Cow::Borrowed(_)), db, matches!(db, Cow::Borrowed(_)));
    let mut m: BTreeMap<String, i32> = BTreeMap::new();
    m.insert("one".into(), 1);
    m.insert("two".into(), 2);
    println!("{:?} {:?}", lookup(&m, "two"), lookup(&m, "three"));
    let mut hm: HashMap<String, Vec<u8>> = HashMap::new();
    hm.insert("k".to_string(), vec![1]);
    let r: Option<&Vec<u8>> = hm.get("k");
    println!("{:?}", r);
    println!("{}", total_len(&["ab", "cde"]));
    println!("{}", total_len(&vec![String::from("xyz"), String::from("")]));
    println!("{} {} {}", sum_bytes("AB"), sum_bytes(vec![1u8, 2, 3]), sum_bytes([10u8; 4]));
    let s: String = make_owned("borrowed str");
    let v: Vec<i32> = make_owned(&[4, 5][..]);
    println!("{} {:?}", s, v);
    let tags = ["a", "b"];
    let mut cfg = Config { name: Cow::Borrowed("base"), tags: Cow::Borrowed(&tags) };
    println!("{}", cfg.describe());
    cfg.add_tag("c");
    println!("{}", cfg.describe());
    cfg.rename("renamed".into());
    println!("{}", cfg.describe());
    let c2: Cow<str> = Cow::from("lit");
    let c3: Cow<str> = Cow::from(String::from("own"));
    println!("{} {} {}", c2.len() + c3.len(), c2.into_owned() + "!", c3);
}
