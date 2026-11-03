// Std trait plumbing for custom types: Sum/Product, FromIterator/Extend,
// IntoIterator for &T and &mut T, Borrow for map lookups, AsRef generic,
// HRTB operator bounds on references, collect into Result/Option, and
// Cow with custom ToOwned.
use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashSet};
use std::iter::{FromIterator, Product, Sum};
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Money { cents: i64 }

impl Add for Money { type Output = Money; fn add(self, o: Money) -> Money { Money { cents: self.cents + o.cents } } }
impl<'a> Add<&'a Money> for &'a Money { type Output = Money; fn add(self, o: &Money) -> Money { *self + *o } }
impl Sum for Money { fn sum<I: Iterator<Item = Money>>(it: I) -> Money { it.fold(Money::default(), Add::add) } }
impl<'a> Sum<&'a Money> for Money { fn sum<I: Iterator<Item = &'a Money>>(it: I) -> Money { it.copied().sum() } }

#[derive(Debug, Clone, Copy, PartialEq)]
struct Factor(u32);
impl Mul for Factor { type Output = Factor; fn mul(self, o: Factor) -> Factor { Factor(self.0 * o.0) } }
impl Product for Factor { fn product<I: Iterator<Item = Factor>>(it: I) -> Factor { it.fold(Factor(1), Mul::mul) } }

#[derive(Debug, Default)]
struct Bag { items: Vec<String>, total_len: usize }

impl<S: Into<String>> FromIterator<S> for Bag {
    fn from_iter<I: IntoIterator<Item = S>>(it: I) -> Self {
        let mut b = Bag::default();
        b.extend(it);
        b
    }
}
impl<S: Into<String>> Extend<S> for Bag {
    fn extend<I: IntoIterator<Item = S>>(&mut self, it: I) {
        for s in it {
            let s = s.into();
            self.total_len += s.len();
            self.items.push(s);
        }
    }
}
impl<'a> IntoIterator for &'a Bag {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}
impl<'a> IntoIterator for &'a mut Bag {
    type Item = &'a mut String;
    type IntoIter = std::slice::IterMut<'a, String>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter_mut() }
}
impl IntoIterator for Bag {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Key { ns: String }
impl Borrow<str> for Key { fn borrow(&self) -> &str { &self.ns } }

fn total_len<T: AsRef<str>>(xs: &[T]) -> usize { xs.iter().map(|x| x.as_ref().len()).sum() }

fn sum_refs<T>(xs: &[T]) -> T where T: Default + Clone, for<'a> &'a T: Add<&'a T, Output = T> {
    let mut acc = T::default();
    for x in xs { acc = &acc + x; }
    acc
}

#[derive(Debug, Clone, PartialEq)]
struct Path2(Vec<u8>);
#[derive(Debug, PartialEq)]
struct PathRef([u8]);
impl PathRef { fn new(b: &[u8]) -> &PathRef { unsafe { &*(b as *const [u8] as *const PathRef) } } }
impl Borrow<PathRef> for Path2 { fn borrow(&self) -> &PathRef { PathRef::new(&self.0) } }
impl ToOwned for PathRef { type Owned = Path2; fn to_owned(&self) -> Path2 { Path2(self.0.to_vec()) } }

fn normalize(p: &PathRef) -> Cow<'_, PathRef> {
    if p.0.contains(&b'\\') {
        Cow::Owned(Path2(p.0.iter().map(|&b| if b == b'\\' { b'/' } else { b }).collect()))
    } else {
        Cow::Borrowed(p)
    }
}

fn main() {
    let wallet = vec![Money { cents: 150 }, Money { cents: 275 }, Money { cents: -25 }];
    let by_ref: Money = wallet.iter().sum();
    let by_val: Money = wallet.clone().into_iter().sum();
    println!("sum ref={:?} val={:?} generic={:?}", by_ref, by_val, sum_refs(&wallet));
    let f: Factor = (1..=5).map(Factor).product();
    println!("product {:?}", f);
    let empty: Money = Vec::<Money>::new().into_iter().sum();
    println!("empty sum {:?}", empty);

    let mut bag: Bag = vec!["apple", "kiwi"].into_iter().collect();
    bag.extend(vec![String::from("fig")]);
    bag.extend(["plum"].iter().copied());
    for s in &mut bag { s.make_ascii_uppercase(); }
    let mut lens = vec![];
    for s in &bag { lens.push(s.len()); }
    println!("bag {:?} lens {:?} total {}", bag.items, lens, bag.total_len);
    let joined: String = bag.into_iter().map(|s| s.to_lowercase()).collect::<Vec<_>>().join("+");
    println!("joined {}", joined);

    let mut m: BTreeMap<Key, u32> = BTreeMap::new();
    for (i, n) in ["net", "fs", "io"].iter().enumerate() { m.insert(Key { ns: n.to_string() }, i as u32); }
    println!("lookup by &str: {:?} {:?} contains io: {}", m.get("fs"), m.get("zz"), m.contains_key("io"));
    let set: HashSet<Key> = m.keys().cloned().collect();
    println!("hashset borrow lookup: {}", set.contains("net"));

    println!("as_ref lens: {} {} {}", total_len(&["ab", "cde"]), total_len(&[String::from("xyz")]), total_len::<Cow<str>>(&[Cow::Borrowed("q"), Cow::Owned("rr".into())]));

    let parsed: Result<Vec<i32>, _> = "1 2 3".split(' ').map(str::parse::<i32>).collect();
    let failed: Result<Vec<i32>, _> = "1 x 3".split(' ').map(str::parse::<i32>).collect();
    println!("collect result: {:?} {:?}", parsed, failed.map_err(|e| e.to_string()));
    let opt: Option<Vec<char>> = ["a", "b", ""].iter().map(|s| s.chars().next()).collect();
    let opt2: Option<String> = ["a", "b"].iter().map(|s| s.chars().next()).collect();
    println!("collect option: {:?} {:?}", opt, opt2);
    let total: Option<u32> = [Some(1u32), Some(2), Some(3)].into_iter().sum();
    let bad: Result<i32, String> = vec![Ok(1), Err("boom".to_string()), Ok(3)].into_iter().sum();
    println!("sum option={:?} sum result={:?}", total, bad);
    let (small, big): (Vec<u32>, HashSet<u32>) = (1..=6).map(|x| (x, x * 100)).unzip();
    let mut big: Vec<_> = big.into_iter().collect();
    big.sort();
    println!("unzip {:?} {:?}", small, big);
    let rc: std::rc::Rc<[u8]> = (1..=4).collect();
    let bx: Box<[String]> = ["x", "y"].iter().map(|s| s.repeat(2)).collect();
    println!("rc slice {:?} boxed {:?}", rc, bx);

    let p1 = Path2(b"a\\b\\c".to_vec());
    let p2 = Path2(b"a/b".to_vec());
    for p in [&p1, &p2] {
        let n = normalize(p.borrow());
        let kind = match &n { Cow::Borrowed(_) => "borrowed", Cow::Owned(_) => "owned" };
        let owned: Path2 = n.into_owned();
        println!("{} -> {} ({})", String::from_utf8_lossy(&p.0), String::from_utf8_lossy(&owned.0), kind);
    }
}
