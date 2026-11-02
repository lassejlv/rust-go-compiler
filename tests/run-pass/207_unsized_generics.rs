use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::path::Path;

fn show<T: ?Sized + Display>(t: &T) -> String {
    format!("<{}>", t)
}

fn debug_len<T: ?Sized + Debug>(t: &T) -> usize {
    format!("{:?}", t).len()
}

fn size_of_ref<T: ?Sized>(_: &T) -> usize {
    std::mem::size_of::<&T>()
}

// A struct with an unsized last field
struct Labeled<T: ?Sized> {
    id: u32,
    value: T,
}

fn label_info(l: &Labeled<[i32]>) -> String {
    format!("id={} len={} sum={}", l.id, l.value.len(), l.value.iter().sum::<i32>())
}

fn label_str(l: &Labeled<dyn Display>) -> String {
    format!("id={} value={}", l.id, &l.value)
}

// Generic over AsRef<str> / AsRef<[T]> / Borrow
fn count_chars<S: AsRef<str>>(items: &[S]) -> usize {
    items.iter().map(|s| s.as_ref().chars().count()).sum()
}

fn total<A: AsRef<[u32]>>(a: A) -> u32 {
    a.as_ref().iter().sum()
}

fn contains_key<K, Q>(keys: &[K], q: &Q) -> bool
where
    K: Borrow<Q>,
    Q: ?Sized + PartialEq,
{
    keys.iter().any(|k| k.borrow() == q)
}

fn ext_of<P: AsRef<Path> + ?Sized>(p: &P) -> Option<String> {
    p.as_ref().extension().map(|e| e.to_string_lossy().into_owned())
}

trait Shape {
    fn area(&self) -> f64;
}
struct Sq(f64);
impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

fn total_area<S: ?Sized + Shape>(shapes: &[&S]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

// Box<T: ?Sized> conversions
fn into_boxed_slice_sum(v: Vec<u64>) -> u64 {
    let b: Box<[u64]> = v.into_boxed_slice();
    b.iter().sum()
}

trait Visitor {
    fn visit(&mut self, s: &str);
}
struct Collect(Vec<String>);
impl Visitor for Collect {
    fn visit(&mut self, s: &str) {
        self.0.push(s.to_uppercase());
    }
}

fn walk<V: Visitor + ?Sized>(v: &mut V, items: &[&str]) {
    for i in items {
        v.visit(i);
    }
}

fn main() {
    println!("{}", show("str slice"));
    println!("{}", show(&String::from("owned")));
    println!("{}", show(&3.5));
    let d: &dyn Display = &'z';
    println!("{}", show(d));
    println!("{} {}", debug_len(&[1, 2, 3][..]), debug_len("abc"));
    println!(
        "{} {} {}",
        size_of_ref(&5u8) == std::mem::size_of::<usize>(),
        size_of_ref("x") == 2 * std::mem::size_of::<usize>(),
        size_of_ref(&[1u8, 2][..]) == 2 * std::mem::size_of::<usize>()
    );

    let l: Box<Labeled<[i32]>> = Box::new(Labeled { id: 7, value: [1, 2, 3, 4] });
    println!("{}", label_info(&l));
    let arr = Labeled { id: 8, value: [10, 20] };
    let r: &Labeled<[i32]> = &arr;
    println!("{}", label_info(r));
    let dl: &Labeled<dyn Display> = &Labeled { id: 9, value: "dynamic" };
    println!("{}", label_str(dl));

    println!("{}", count_chars(&["héllo", "wörld"]));
    println!("{}", count_chars(&vec![String::from("abc")]));
    println!("{} {} {}", total(vec![1, 2, 3]), total([4u32; 3]), total(&[5u32][..]));
    let keys = vec![String::from("a"), String::from("bc")];
    println!("{} {}", contains_key(&keys, "bc"), contains_key(&keys, "zz"));
    println!("{:?} {:?} {:?}", ext_of("a/b.rs"), ext_of(&String::from("x.tar.gz")), ext_of(Path::new("noext")));

    let s1 = Sq(2.0);
    let s2 = Sq(3.0);
    let dyns: Vec<&dyn Shape> = vec![&s1, &s2];
    println!("{} {}", total_area::<dyn Shape>(&dyns), total_area(&[&s1, &s2]));
    println!("{}", into_boxed_slice_sum(vec![100, 200, 300]));

    let mut c = Collect(vec![]);
    walk(&mut c, &["a", "b"]);
    let dv: &mut dyn Visitor = &mut c;
    walk(dv, &["c"]);
    println!("{:?}", c.0);
}
