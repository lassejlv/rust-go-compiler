use std::fmt::{self, Display};

trait Handler { fn call(&self) -> String; }
impl<F: Fn() -> String> Handler for F { fn call(&self) -> String { format!("fn:{}", self()) } }
struct Static(&'static str);
impl Handler for Static { fn call(&self) -> String { format!("static:{}", self.0) } }

trait Render { fn render(&self) -> String; }
impl<T: Display> Render for T { fn render(&self) -> String { format!("disp:{}", self) } }
struct Opaque(u8);
impl Render for Opaque { fn render(&self) -> String { format!("opaque:{}", self.0) } }
impl Render for [u8] { fn render(&self) -> String { format!("bytes:{}", self.len()) } }
struct Many<T>(Vec<T>);
impl<T: Render> Render for Many<T> { fn render(&self) -> String { self.0.iter().map(|x| x.render()).collect::<Vec<_>>().join(",") } }
struct Maybe<T>(Option<T>);
impl<T: Render> Render for Maybe<T> { fn render(&self) -> String { match &self.0 { Some(x) => x.render(), None => "none".into() } } }

struct Celsius(f64);
impl Display for Celsius { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}C", self.0) } }

trait Stream: Iterator<Item = u8> { fn label(&self) -> &'static str; }
struct Bytes2(std::vec::IntoIter<u8>);
impl Iterator for Bytes2 { type Item = u8; fn next(&mut self) -> Option<u8> { self.0.next() } }
impl Stream for Bytes2 { fn label(&self) -> &'static str { "b2" } }

fn consume(it: impl Iterator<Item = u8>) -> u32 { it.map(u32::from).sum() }
fn consume_dyn(it: &mut dyn Iterator<Item = u8>) -> Vec<u8> { let mut v = vec![]; for x in it { v.push(x); } v }

trait Plugin { fn name(&self) -> String; fn run(&mut self, input: &str) -> String; }
impl<F> Plugin for (String, F) where F: FnMut(&str) -> String {
    fn name(&self) -> String { self.0.clone() }
    fn run(&mut self, input: &str) -> String { (self.1)(input) }
}

fn main() {
    let hs: Vec<Box<dyn Handler>> = vec![Box::new(|| "closure".to_string()), Box::new(Static("s"))];
    for h in &hs { println!("{}", h.call()); }
    println!("{}", (|| String::from("direct")).call());
    println!("{} {} {}", 5.render(), "x".render(), Celsius(21.5).render());
    println!("{} {}", Opaque(3).render(), Many(vec![Opaque(1), Opaque(2)]).render());
    println!("{} {} {}", b"abc"[..].render(), Maybe(Some(Opaque(9))).render(), Maybe(None::<Opaque>).render());
    println!("{}", Many(vec![Maybe(Some(Opaque(4))), Maybe(None)]).render());
    let mut s: Box<dyn Stream> = Box::new(Bytes2(vec![1, 2, 3, 4].into_iter()));
    println!("{} {:?}", s.label(), s.next());
    let doubled: Vec<u8> = s.by_ref().map(|x| x * 2).take(2).collect();
    println!("{:?} {:?}", doubled, s.next());
    let s2: Box<dyn Stream> = Box::new(Bytes2(vec![5, 6].into_iter()));
    println!("{}", consume(s2));
    let mut s3 = Bytes2(vec![7, 8].into_iter());
    println!("{:?}", consume_dyn(&mut s3));
    let mut counter = 0;
    let mut plugins: Vec<Box<dyn Plugin + '_>> = vec![
        Box::new(("upper".to_string(), |s: &str| s.to_uppercase())),
        Box::new(("count".to_string(), |s: &str| { counter += 1; format!("{}#{}", s, counter) })),
    ];
    for _ in 0..2 { for p in plugins.iter_mut() { println!("{} -> {}", p.name(), p.run("hi")); } }
}
