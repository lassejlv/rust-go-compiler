// Items of slice/vec iterators over tuples whose fields have the same type
// but different lifetimes, through adapters (map, enumerate, chain, rev,
// filter, flatten, peekable, zip, step_by, windows, chunks).
use std::collections::VecDeque;

type Pair<'a> = (&'static str, &'a str);

fn firsts<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.iter().map(|p| p.0).collect()
}

fn seconds<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().map(|&(_, b)| b).collect()
}

fn enumerated<'a>(v: &[Pair<'a>]) -> Vec<(usize, &'static str)> {
    v.iter().enumerate().map(|(i, p)| (i, p.0)).collect()
}

fn chained<'a>(v: &[Pair<'a>], w: &[Pair<'a>]) -> Vec<&'static str> {
    v.iter().chain(w.iter()).map(|p| p.0).collect()
}

fn reversed<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.iter().rev().map(|p| p.0).collect()
}

fn filtered<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.iter().filter(|p| !p.1.is_empty()).map(|p| p.0).collect()
}

fn flattened<'a>(v: &[Vec<Pair<'a>>]) -> Vec<&'static str> {
    v.iter().flatten().map(|p| p.0).collect()
}

fn found<'a>(v: &[Pair<'a>]) -> Option<&'static str> {
    v.iter().find(|p| p.1.is_empty()).map(|p| p.0)
}

fn peeked<'a>(v: &[Pair<'a>]) -> Option<&'static str> {
    let mut it = v.iter().peekable();
    it.peek().map(|p| p.0)
}

fn zipped<'a>(a: &[&'static str], b: &[&'a str]) -> Vec<&'static str> {
    a.iter().zip(b.iter()).map(|(x, _)| *x).collect()
}

fn zipped2<'a>(a: &[&'a str], b: &[&'static str]) -> Vec<&'static str> {
    a.iter().zip(b).map(|(_, y)| *y).collect()
}

fn stepped<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.iter().step_by(2).map(|p| p.0).collect()
}

fn windowed<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.windows(2).map(|w| w[0].0).collect()
}

fn chunked<'a>(v: &[Pair<'a>]) -> Vec<&'static str> {
    v.chunks(2).flatten().map(|w| w.0).collect()
}

fn into_iter_vec<'a>(v: Vec<Pair<'a>>) -> Vec<&'static str> {
    v.into_iter().map(|(a, _)| a).collect()
}

fn into_iter_ref<'a>(v: &Vec<Pair<'a>>) -> Vec<&'static str> {
    v.into_iter().map(|p| p.0).collect()
}

fn deque<'a>(v: &VecDeque<Pair<'a>>) -> Vec<&'static str> {
    v.iter().map(|p| p.0).collect()
}

fn option_iter<'a>(o: &Option<Pair<'a>>) -> Vec<&'static str> {
    o.iter().map(|p| p.0).collect()
}

fn array_iter<'a>(x: &'a str) -> Vec<&'static str> {
    [("a", x), ("b", x)].into_iter().map(|(a, _)| a).collect()
}

fn iter_mut<'a>(v: &mut [Pair<'a>]) -> Vec<&'static str> {
    v.iter_mut().map(|p| p.0).collect()
}

fn drained<'a>(v: &mut Vec<Pair<'a>>) -> Vec<&'static str> {
    v.drain(..).map(|p| p.0).collect()
}

fn main() {
    let owned = String::from("tmp");
    let v: Vec<Pair> = vec![("a", owned.as_str()), ("b", ""), ("c", owned.as_str())];
    let w: Vec<(&str, &'static str)> = vec![(owned.as_str(), "w1")];
    let r1 = firsts(&v);
    let r2 = seconds(&w);
    let r3 = enumerated(&v);
    let r4 = chained(&v, &v[..1]);
    let r5 = reversed(&v);
    let r6 = filtered(&v);
    let r7 = flattened(&[v.clone(), v.clone()]);
    let r8 = found(&v);
    let r9 = peeked(&v);
    let r10 = zipped(&["x", "y"], &[owned.as_str()]);
    let r11 = zipped2(&[owned.as_str()], &["q"]);
    let r12 = stepped(&v);
    let r13 = windowed(&v);
    let r14 = chunked(&v);
    let r15 = into_iter_vec(v.clone());
    let r16 = into_iter_ref(&v);
    let r17 = deque(&v.iter().copied().collect());
    let r18 = option_iter(&Some(v[0]));
    let r19 = array_iter(&owned);
    let r20 = iter_mut(&mut v.clone());
    let r21 = drained(&mut v.clone());
    drop(v);
    drop(w);
    drop(owned);
    println!("{:?} {:?} {:?}", r1, r2, r3);
    println!("{:?} {:?} {:?}", r4, r5, r6);
    println!("{:?} {:?} {:?}", r7, r8, r9);
    println!("{:?} {:?} {:?} {:?}", r10, r11, r12, r13);
    println!("{:?} {:?} {:?} {:?}", r14, r15, r16, r17);
    println!("{:?} {:?} {:?} {:?}", r18, r19, r20, r21);
}
