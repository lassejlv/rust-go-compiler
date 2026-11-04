// Map entries storing values whose fields have the same type with
// different lifetimes: or_insert stores into the map's V slot by slot.
use std::collections::{BTreeMap, HashMap};

type P<'s> = (&'static str, &'s str);

fn by_entry<'s>(s: &'s str) -> HashMap<u8, (&'static str, &'s str)> {
    let mut m = HashMap::new();
    m.entry(2).or_insert(("e", s));
    m
}

fn by_alias<'s>(s: &'s str) -> BTreeMap<u8, P<'s>> {
    let mut m = BTreeMap::new();
    m.insert(1, ("m", s));
    m.entry(2).or_insert(("e", s));
    let e = m.entry(3);
    e.or_insert(("f", s));
    m
}

fn keys_static<'s>(words: &[&'s str]) -> BTreeMap<&'s str, (&'static str, usize)> {
    let mut m = BTreeMap::new();
    for w in words {
        let slot = m.entry(*w).or_insert(("new", 0));
        slot.1 += 1;
        if slot.1 > 1 {
            slot.0 = "seen";
        }
    }
    m
}

fn main() {
    let owned = String::from("owned");
    let a = by_entry(&owned);
    let b = by_alias(&owned);
    let c = keys_static(&["x", "y", owned.as_str(), "x"]);
    println!("{:?}", a);
    println!("{:?}", b);
    println!("{:?}", c);
}
