use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Index, Range};
type Res<T> = Result<T, String>;
fn show_all<I>(it: I) -> String where I: IntoIterator<Item: Display> { it.into_iter().map(|x| format!("<{}>", x)).collect() }
#[derive(Debug, Default, PartialEq, Clone)]
struct Bag<T> { items: Vec<T> }
impl<T> FromIterator<T> for Bag<T> { fn from_iter<I: IntoIterator<Item = T>>(i: I) -> Self { Bag { items: i.into_iter().collect() } } }
impl<T> Extend<T> for Bag<T> { fn extend<I: IntoIterator<Item = T>>(&mut self, i: I) { for x in i { self.items.push(x); } } }
impl<'a, T> IntoIterator for &'a Bag<T> { type Item = &'a T; type IntoIter = std::slice::Iter<'a, T>; fn into_iter(self) -> Self::IntoIter { self.items.iter() } }
impl<T> IntoIterator for Bag<T> { type Item = T; type IntoIter = std::vec::IntoIter<T>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }
impl<T> Index<Range<usize>> for Bag<T> { type Output = [T]; fn index(&self, r: Range<usize>) -> &[T] { &self.items[r] } }
struct Grid<'a, T, const W: usize> { cells: &'a [T], name: &'a str }
impl<'a, T: Copy + Into<f64>, const W: usize> Grid<'a, T, W> { fn row_sums(&self) -> Vec<f64> { self.cells.chunks(W).map(|r| r.iter().map(|&x| x.into()).sum()).collect() } }
extern "C" fn c_add(a: i32, b: i32) -> i32 { a + b }
unsafe fn raw_read(p: *const u32, i: usize) -> u32 { *p.add(i) }
fn total<T: AsRef<[u32]>>(x: T) -> u32 { x.as_ref().iter().sum() }
fn lookup<K: Borrow<str> + std::hash::Hash + Eq>(m: &HashMap<K, i32>, k: &str) -> i32 { *m.get(k).unwrap_or(&-1) }
fn normalize(s: &str) -> Cow<str> { if s.contains(' ') { Cow::Owned(s.replace(' ', "_")) } else { Cow::Borrowed(s) } }
fn parse_all(v: &[&str]) -> Res<Vec<i32>> { v.iter().map(|s| s.parse::<i32>().map_err(|e| format!("{}: {}", s, e))).collect() }
#[derive(Debug, Clone, Copy)] struct Rgb(u8, u8, u8);
fn main() {
    println!("{}", show_all(vec![1, 2]));
    println!("{}", show_all(["a", "b"].iter()));
    let mut b: Bag<i32> = (1..4).collect();
    b.extend(vec![10, 20]);
    let doubled: Bag<i32> = b.items.iter().map(|x| x * 2).collect();
    for x in &b { print!("{} ", x); }
    println!("| {:?} {:?}", &b[1..3], doubled);
    let s: i32 = b.into_iter().sum();
    println!("{}", s);
    let data = [1u8, 2, 3, 4, 5, 6];
    let g: Grid<u8, 3> = Grid { cells: &data, name: "g" };
    println!("{} {:?}", g.name, g.row_sums());
    println!("{}", c_add(2, 3));
    let arr = [7u32, 8, 9];
    println!("{}", unsafe { raw_read(arr.as_ptr(), 2) });
    println!("{} {} {}", total(vec![1, 2]), total([3u32; 3]), total(&arr[..]));
    let mut m: HashMap<String, i32> = HashMap::new(); m.insert("k".into(), 5);
    println!("{} {}", lookup(&m, "k"), lookup(&m, "z"));
    let mut c = normalize("a b c"); c.to_mut().push('!');
    println!("{} {} {}", c, normalize("abc"), matches!(normalize("x"), Cow::Borrowed(_)));
    println!("{:?} {:?}", parse_all(&["1", "2"]), parse_all(&["1", "x"]));
    let r = Rgb(1, 2, 3); let r2 = Rgb { 0: 9, ..r };
    println!("{:?} {:?}", r, r2);
    println!("{} {}", matches!(Some(5), Some(n) if n > 3), matches!('x', 'a'..='f' | 'x'));
    let raw = r#"He said "hi" \n"#; let bs = b"bytes\x01"; let bc = b'\x7f'; let em = '\u{1F600}';
    println!("{} {:?} {} {} {}", raw, bs, bc, em, em.len_utf8());
    println!("{} {} {} {} {} {}", 1_000i64, 0o777, 0b1010_1010u8, 1e3, 2.5E-3f32, 0xFFu8 as i8);
    println!("{:?} {}", -1i64 as u8 as char, (65u8 + 1) as char);
    let res: Vec<i32> = (|| { let mut v = vec![]; for x in [3, -1, 4] { if x < 0 { return v; } v.push(x); } v })();
    println!("{:?}", res);
    let add = |a: i32| move |b: i32| move |c: i32| a + b + c;
    println!("{}", add(1)(2)(3));
    let outer = |x: i32| { let inner = |y: i32| x * y; inner(x + 1) };
    println!("{}", outer(4));
    let _: () = ();
    let v = vec![Some(1), None, Some(3)];
    let out: Vec<String> = v.iter().map(|o| if let Some(1) = o { "one".into() } else if let Some(n) = o { n.to_string() } else { "none".into() }).collect();
    println!("{:?}", out);
    {
        use std::cmp::Ordering::*;
        let o = 3.cmp(&4);
        println!("{}", match o { Less => "lt", Equal => "eq", Greater => "gt" });
    }
}
