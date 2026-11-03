use std::fmt::Debug;

trait Base: Debug {
    fn id(&self) -> u32;
    fn describe(&self) -> String { format!("base#{}", self.id()) }
}
trait Mid: Base {
    fn level(&self) -> u8;
    fn describe_mid(&self) -> String { format!("{} lvl{}", self.describe(), self.level()) }
}
trait Top: Mid {
    fn title(&self) -> String;
}

#[derive(Debug)]
struct Item { id: u32, lvl: u8 }
impl Base for Item { fn id(&self) -> u32 { self.id } }
impl Mid for Item { fn level(&self) -> u8 { self.lvl } }
impl Top for Item { fn title(&self) -> String { format!("Item{}", self.id) } }

#[derive(Debug)]
struct Other(u32);
impl Base for Other { fn id(&self) -> u32 { self.0 * 1000 } fn describe(&self) -> String { "other!".into() } }
impl Mid for Other { fn level(&self) -> u8 { 9 } }

fn use_base(b: &dyn Base) -> String { b.describe() }
fn use_mid(m: &dyn Mid) -> String { m.describe_mid() }

trait Shape { fn area(&self) -> f64; }
impl dyn Shape {
    fn bigger_than(&self, o: &dyn Shape) -> bool { self.area() > o.area() }
}
struct Sq(f64);
struct Ci(f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
impl Shape for Ci { fn area(&self) -> f64 { 3.0 * self.0 * self.0 } }

fn main() {
    let tops: Vec<Box<dyn Top>> = vec![Box::new(Item { id: 1, lvl: 3 }), Box::new(Item { id: 2, lvl: 5 })];
    for t in &tops {
        let m: &dyn Mid = t.as_ref();
        let b: &dyn Base = m;
        println!("{} | {} | {} | {:?}", t.title(), use_mid(m), use_base(b), b);
    }
    let mids: Vec<Box<dyn Mid>> = vec![Box::new(Other(4)), Box::new(Item { id: 7, lvl: 0 })];
    let bases: Vec<Box<dyn Base>> = mids.into_iter().map(|m| m as Box<dyn Base>).collect();
    for b in &bases { println!("{} {}", b.id(), b.describe()); }
    let t: Box<dyn Top> = Box::new(Item { id: 9, lvl: 1 });
    let b: Box<dyn Base> = t;
    println!("{:?}", b);
    let rc: std::rc::Rc<dyn Mid> = std::rc::Rc::new(Other(2));
    let rb: std::rc::Rc<dyn Base> = rc.clone();
    println!("{} {} {}", rb.describe(), rc.describe_mid(), std::rc::Rc::strong_count(&rc));
    let s: Box<dyn Shape> = Box::new(Sq(2.0));
    let c: Box<dyn Shape> = Box::new(Ci(1.0));
    println!("{} {}", s.bigger_than(&*c), c.bigger_than(&*s));
}
