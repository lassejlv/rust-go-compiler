use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Display};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

trait Counter { fn incr(&mut self) -> u32; fn name(&self) -> String; }
struct Simple(u32);
impl Counter for Simple { fn incr(&mut self) -> u32 { self.0 += 1; self.0 } fn name(&self) -> String { "simple".into() } }
struct Stepped { v: u32, step: u32 }
impl Counter for Stepped { fn incr(&mut self) -> u32 { self.v += self.step; self.v } fn name(&self) -> String { format!("step{}", self.step) } }

struct Header<T: ?Sized> { tag: u8, data: T }

trait Conv<T> { type Out; fn conv(&self, t: T) -> Self::Out; }
struct Doubler;
impl Conv<i32> for Doubler { type Out = i64; fn conv(&self, t: i32) -> i64 { t as i64 * 2 } }
impl Conv<&str> for Doubler { type Out = String; fn conv(&self, t: &str) -> String { t.repeat(2) } }

fn total_len<T: ?Sized + AsRef<[u8]>>(h: &Header<T>) -> usize { h.data.as_ref().len() + h.tag as usize }

fn main() {
    let shared: Vec<Rc<RefCell<dyn Counter>>> = vec![Rc::new(RefCell::new(Simple(0))), Rc::new(RefCell::new(Stepped { v: 0, step: 5 }))];
    let alias = Rc::clone(&shared[1]);
    for c in &shared { c.borrow_mut().incr(); }
    alias.borrow_mut().incr();
    for c in &shared { let b = c.borrow(); println!("{} {}", b.name(), Rc::strong_count(c)); }
    println!("{}", alias.borrow_mut().incr());

    let am: Arc<Mutex<dyn Counter + Send>> = Arc::new(Mutex::new(Simple(10)));
    let hs: Vec<_> = (0..4).map(|_| { let a = Arc::clone(&am); std::thread::spawn(move || { a.lock().unwrap().incr(); }) }).collect();
    for h in hs { h.join().unwrap(); }
    println!("{}", am.lock().unwrap().incr());

    let mut s = Simple(0);
    let mut st = Stepped { v: 1, step: 2 };
    let mut cs: Vec<&mut dyn Counter> = vec![&mut s, &mut st];
    for _ in 0..3 { for c in cs.iter_mut() { c.incr(); } }
    let r: &mut dyn Counter = &mut *cs[1];
    println!("{}", r.incr());
    println!("{} {}", s.0, st.v);

    let bs: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
    let rs: Rc<[i32]> = Rc::from(vec![4, 5]);
    let rstr: Rc<str> = Rc::from("rcstr");
    let astr: Arc<str> = "arc".into();
    println!("{:?} {:?} {} {} {}", bs, rs, rstr, astr, bs.len() + rs.len());
    let f: Arc<dyn Fn(i32) -> i32 + Send + Sync> = Arc::new(|x| x * 3);
    let f2 = f.clone();
    println!("{} {}", std::thread::spawn(move || f2(5)).join().unwrap(), f(1));
    let once: Cell<Option<Box<dyn FnOnce() -> String>>> = Cell::new(Some(Box::new(|| "fired".to_string())));
    println!("{:?} {:?}", once.take().map(|f| f()), once.take().map(|f| f()));
    let iters: Box<dyn Iterator<Item = Box<dyn Fn(i32) -> i32>>> = Box::new((1..4).map(|k| Box::new(move |x| x * k) as Box<dyn Fn(i32) -> i32>));
    println!("{:?}", iters.map(|f| f(10)).collect::<Vec<_>>());

    let h: Header<[u8; 3]> = Header { tag: 1, data: [1, 2, 3] };
    let hs: &Header<[u8]> = &h;
    println!("{} {:?} {}", hs.data.len(), &hs.data, total_len(hs));
    let hd: Box<Header<dyn Display>> = Box::new(Header { tag: 9, data: 2.5f32 });
    println!("{} {}", hd.tag, &hd.data);
    let hdbg: &Header<dyn Debug> = &Header { tag: 0, data: vec!["v"] };
    println!("{:?}", &hdbg.data);
    println!("{}", total_len(&Header { tag: 2, data: "abc" }));

    let d = Doubler;
    let c1: &dyn Conv<i32, Out = i64> = &d;
    let c2: Box<dyn Conv<&str, Out = String>> = Box::new(Doubler);
    println!("{} {}", c1.conv(21), c2.conv("ab"));
}
