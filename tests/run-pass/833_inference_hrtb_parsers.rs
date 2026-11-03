use std::collections::{BTreeMap, HashMap, BTreeSet};
use std::fmt::Debug;
trait Parser<'a> { type Out: Debug; fn parse(&self, s: &'a str) -> Option<(Self::Out, &'a str)>; }
struct Word; struct Num;
impl<'a> Parser<'a> for Word { type Out = &'a str; fn parse(&self, s: &'a str) -> Option<(&'a str, &'a str)> { let i = s.find(' ').unwrap_or(s.len()); if i == 0 { None } else { Some((&s[..i], s[i..].trim_start())) } } }
impl<'a> Parser<'a> for Num { type Out = i64; fn parse(&self, s: &'a str) -> Option<(i64, &'a str)> { let (w, r) = Word.parse(s)?; Some((w.parse().ok()?, r)) } }
fn run_all<P>(p: &P, inputs: &[&str]) -> Vec<String> where P: for<'a> Parser<'a> { inputs.iter().map(|s| format!("{:?}", p.parse(s).map(|(o, r)| (o, r.len())))).collect() }
struct Pair<A, B>(A, B);
impl<'a, A: Parser<'a>, B: Parser<'a>> Parser<'a> for Pair<A, B> { type Out = (A::Out, B::Out); fn parse(&self, s: &'a str) -> Option<(Self::Out, &'a str)> { let (a, r) = self.0.parse(s)?; let (b, r) = self.1.parse(r)?; Some(((a, b), r)) } }
fn mk<T: Default + Debug>() -> T { T::default() }
#[derive(Debug, Default)] struct Cfg { name: String, n: u8, tags: Vec<String>, m: BTreeMap<u8, u8> }
fn main() {
    println!("{:?}", run_all(&Word, &["hi there", "", "solo"]));
    println!("{:?}", run_all(&Num, &["12 x", "z 1", "-7"]));
    println!("{:?}", run_all(&Pair(Word, Num), &["a 1 rest", "a b"]));
    println!("{:?}", run_all(&Pair(Pair(Num, Num), Word), &["1 2 w z"]));
    let r: Result<HashMap<String, i32>, std::num::ParseIntError> = "a=1,b=2".split(',').map(|kv| { let (k, v) = kv.split_once('=').unwrap(); v.parse().map(|v| (k.to_string(), v)) }).collect();
    let mut r: Vec<_> = r.unwrap().into_iter().collect(); r.sort(); println!("{:?}", r);
    let total = [1.5, 2.5].iter().sum::<f64>() + [1, 2].iter().copied().sum::<i32>() as f64;
    println!("{}", total);
    let c = Cfg { name: "x".into(), ..Default::default() };
    println!("{:?} {:?}", c, mk::<(u8, String, Option<bool>)>());
    let mut v = Vec::new();
    v.push(Default::default());
    v[0] = 3u16;
    println!("{:?}", v);
    let f = |a, b| a * b + 1;
    println!("{}", f(3i64, 4));
    let s: BTreeSet<_> = [3, 1, 2].into();
    let w: Vec<_> = s.iter().map(|x| x * 2).collect();
    println!("{:?} {:?}", s, w);
    let parsed = "42".parse().map(|x: u64| x + 1);
    println!("{:?}", parsed);
    let mut m = BTreeMap::new();
    for (i, w) in "a b a c b a".split(' ').enumerate() { m.entry(w).or_insert_with(Vec::new).push(i); }
    println!("{:?}", m);
    let x = 5;
    let y = if x > 3 { 1.0 } else { 2.5 };
    let z = y * 2.0;
    println!("{}", z);
    let big = 3_000_000_000u32 as u64 * 4;
    println!("{}", big);
    let n = 255u8;
    let k = n as i8 as i32;
    println!("{}", k);
    let arr = [[0u8; 3]; 2];
    let flat: Vec<u8> = arr.concat();
    println!("{:?}", flat);
    let total: u64 = (1..=20).product();
    println!("{}", total);
    let avg = { let v = [3, 4, 5]; v.iter().sum::<i32>() as f32 / v.len() as f32 };
    println!("{}", avg);
    let o: Option<Option<i32>> = Some(None);
    println!("{:?} {:?}", o.flatten(), Some(Some(3)).flatten());
    let res: Option<Result<i32, String>> = Some(Ok(1));
    println!("{:?}", res.transpose());
    let e: Result<Option<i32>, String> = Ok(None);
    println!("{:?}", e.transpose());
    println!("{:?}", Some(3).zip(Some("a")).map(|(a, b)| format!("{}{}", b, a)));
    println!("{:?} {:?}", Some(1).xor(None::<i32>), Some(1).xor(Some(2)));
    println!("{:?}", Some(4).filter(|x| x % 2 == 1).or(Some(9)).map_or_else(|| -1, |v| v * 10));
    let mut g = None; *g.get_or_insert_with(|| 10) += 1; println!("{:?}", g);
    println!("{:?}", Ok::<i32, String>(3).and_then(|x| if x > 2 { Err(format!("big {}", x)) } else { Ok(x) }));
    println!("{}", Err::<i32, &str>("e").unwrap_or_default());
    println!("{:?}", Err::<i32, &str>("e").or_else(|e| if e == "e" { Ok::<i32, &str>(0) } else { Err(e) }));
    println!("{:?}", [Ok(1), Err("x"), Ok(3)].into_iter().filter_map(Result::ok).collect::<Vec<i32>>());
    println!("{:?}", [Some(1), None].iter().map(|o| o.ok_or("none")).collect::<Vec<_>>());
    println!("{:?}", vec![Some(2), Some(3)].into_iter().sum::<Option<i32>>());
    println!("{:?}", vec![Ok(2), Err("bad"), Ok(3)].into_iter().product::<Result<i32, &str>>());
}
