// Items through adapters whose items come from closures or bounds:
// flat_map, filter_map, scan, fold, reduce, max_by, min_by_key, nth,
// skip/take, cycle, skip_while, map_while, and collecting into maps,
// Option and Result.
use std::collections::{BTreeMap, HashMap};

fn flat_map<'a>(v: &[Vec<(&'static str, &'a str)>]) -> Vec<&'static str> {
    v.iter().flat_map(|w| w.iter().map(|p| p.0)).collect()
}

fn filter_map<'a>(v: &[(&'static str, &'a str)]) -> Vec<&'static str> {
    v.iter().filter_map(|p| if p.1.is_empty() { None } else { Some(p.0) }).collect()
}

fn scan<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().scan(0, |n, p| { *n += 1; Some(p.1) }).collect()
}

fn fold<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().fold(Vec::new(), |mut acc, p| { acc.push(p.1); acc })
}

fn reduce<'a>(v: &[(&'a str, &'static str)]) -> Option<&'static str> {
    v.iter().map(|p| p.1).reduce(|a, b| if a < b { a } else { b })
}

fn max_by<'a>(v: &[(&'a str, &'static str)]) -> Option<&'static str> {
    v.iter().copied().max_by(|a, b| a.0.cmp(b.0)).map(|p| p.1)
}

fn min_by_key<'a>(v: &[(&'a str, &'static str)]) -> Option<&'static str> {
    v.iter().min_by_key(|p| p.0).map(|p| p.1)
}

fn nth<'a>(v: &[(&'static str, &'a str)]) -> Option<&'static str> {
    v.iter().map(|p| p.0).nth(1)
}

fn skip_take<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().skip(1).take(3).map(|p| p.1).collect()
}

fn cycle<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().cycle().take(4).map(|p| p.1).collect()
}

fn skip_while<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().skip_while(|p| p.0.len() > 3).map(|p| p.1).collect()
}

fn map_while<'a>(v: &[(&'static str, &'a str)]) -> Vec<&'static str> {
    v.iter().map_while(|p| Some(p.0)).collect()
}

fn to_btree<'a>(v: Vec<(&'static str, &'a str)>) -> BTreeMap<&'static str, &'a str> {
    v.into_iter().collect()
}

fn swapped<'a>(v: &[(&'a str, &'static str)]) -> HashMap<&'static str, &'a str> {
    v.iter().map(|&(a, b)| (b, a)).collect()
}

fn lens<'a>(v: &[(&'static str, &'a str)]) -> BTreeMap<&'static str, usize> {
    v.iter().map(|p| (p.0, p.1.len())).collect()
}

fn all_some<'a>(v: &[Option<(&'static str, &'a str)>]) -> Option<Vec<&'static str>> {
    v.iter().map(|o| o.map(|p| p.0)).collect()
}

fn all_ok<'a>(v: Vec<Result<&'static str, &'a str>>) -> Result<Vec<&'static str>, &'a str> {
    v.into_iter().collect()
}

fn unzipped<'a>(v: Vec<(&'a str, &'static str)>) -> Vec<&'static str> {
    let (_a, b): (Vec<&'a str>, Vec<&'static str>) = v.into_iter().unzip();
    b
}

fn extended<'a>(m: &HashMap<&'static str, &'a str>, out: &mut Vec<&'static str>) {
    out.extend(m.keys().copied());
}

fn main() {
    let owned = String::from("long-word");
    let o = owned.as_str();
    let v: Vec<(&'static str, &str)> = vec![("a", o), ("b", ""), ("c", o)];
    let w: Vec<(&str, &'static str)> = vec![(o, "x"), ("yy", "y"), (o, "z")];
    let r1 = flat_map(&[v.clone()]);
    let r2 = filter_map(&v);
    let r3 = scan(&w);
    let r4 = fold(&w);
    let r5 = reduce(&w);
    let r6 = max_by(&w);
    let r7 = min_by_key(&w);
    let r8 = nth(&v);
    let r9 = skip_take(&w);
    let r10 = cycle(&w);
    let r11 = skip_while(&w);
    let r12 = map_while(&v);
    let r13 = to_btree(v.clone());
    let r14 = swapped(&w);
    let r15 = lens(&v);
    let r16 = all_some(&[Some(v[0]), Some(v[1])]);
    let r17 = all_ok(vec![Ok("ok"), Err(o)]);
    let r18 = unzipped(w.clone());
    let mut hm = HashMap::new();
    hm.insert("key", o);
    let mut r19 = Vec::new();
    extended(&hm, &mut r19);
    println!("{:?} {:?} {:?} {:?}", r13, r14.len(), r17, r14.get("x"));
    drop(r13);
    drop(r14);
    drop(r17);
    drop(hm);
    drop(v);
    drop(w);
    drop(owned);
    println!("{:?} {:?} {:?} {:?}", r1, r2, r3, r4);
    println!("{:?} {:?} {:?} {:?}", r5, r6, r7, r8);
    println!("{:?} {:?} {:?} {:?}", r9, r10, r11, r12);
    println!("{:?} {:?} {:?} {:?}", r15, r16, r18, r19);
}
