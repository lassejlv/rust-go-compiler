use std::fmt::Debug;

trait Shape {
    type Unit: Debug + Copy + Into<f64>;
    fn area(&self) -> Self::Unit;
    fn label(&self) -> String;
}

trait Solid: Shape {
    fn depth(&self) -> Self::Unit;
    fn volume(&self) -> f64 {
        self.area().into() * self.depth().into()
    }
    fn report(&self) -> String {
        format!("{} area={:?} depth={:?} vol={:.2}", self.label(), self.area(), self.depth(), self.volume())
    }
}

struct Block { w: u32, h: u32, d: u32 }
struct Cyl { r: f32, d: f32 }

impl Shape for Block {
    type Unit = u32;
    fn area(&self) -> u32 { self.w * self.h }
    fn label(&self) -> String { "block".into() }
}
impl Solid for Block {
    fn depth(&self) -> u32 { self.d }
}

impl Shape for Cyl {
    type Unit = f32;
    fn area(&self) -> f32 { 3.0 * self.r * self.r }
    fn label(&self) -> String { "cyl".into() }
}
impl Solid for Cyl {
    fn depth(&self) -> f32 { self.d }
}

fn biggest<S: Solid>(items: &[S]) -> Option<&S> {
    let mut best: Option<&S> = None;
    for it in items {
        match best {
            None => best = Some(it),
            Some(b) if it.volume() > b.volume() => best = Some(it),
            _ => {}
        }
    }
    best
}

fn sum_areas<S>(items: &[S]) -> f64
where
    S: Shape,
{
    items.iter().map(|s| s.area().into()).sum()
}

trait Named {
    fn name(&self) -> String;
}
trait Greeter: Named {
    fn greet(&self) -> String {
        format!("Hello, {}", self.name())
    }
}
trait LoudGreeter: Greeter + Debug {
    fn loud(&self) -> String {
        format!("{} [{:?}]", self.greet().to_uppercase(), self)
    }
}
#[derive(Debug)]
struct P(&'static str);
impl Named for P { fn name(&self) -> String { self.0.to_string() } }
impl Greeter for P {}
impl LoudGreeter for P {}

fn use_dyn(g: &dyn LoudGreeter) -> String {
    format!("{} / {} / {}", g.name(), g.greet(), g.loud())
}

fn main() {
    let blocks = vec![Block { w: 2, h: 3, d: 4 }, Block { w: 5, h: 1, d: 1 }, Block { w: 3, h: 3, d: 3 }];
    for b in &blocks { println!("{}", b.report()); }
    println!("best {:?}", biggest(&blocks).map(|b| b.report()));
    println!("areas {}", sum_areas(&blocks));
    let cyls = vec![Cyl { r: 1.0, d: 2.0 }, Cyl { r: 0.5, d: 10.0 }];
    for c in &cyls { println!("{}", c.report()); }
    println!("best {:?}", biggest(&cyls).map(|c| c.label()));
    println!("areas {}", sum_areas(&cyls));
    let empty: Vec<Cyl> = vec![];
    println!("{}", biggest(&empty).is_none());
    println!("{}", use_dyn(&P("ann")));
}
