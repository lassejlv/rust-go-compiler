use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

trait Widget: Debug {
    fn draw(&self) -> String;
    fn area(&self) -> u32;
}
#[derive(Debug)] struct Button { label: String }
#[derive(Debug)] struct Slider { min: i32, max: i32 }
#[derive(Debug)] struct Spacer(u32);
impl Widget for Button { fn draw(&self) -> String { format!("[ {} ]", self.label) } fn area(&self) -> u32 { self.label.len() as u32 + 4 } }
impl Widget for Slider { fn draw(&self) -> String { format!("{}|----|{}", self.min, self.max) } fn area(&self) -> u32 { 10 } }
impl Widget for Spacer { fn draw(&self) -> String { " ".repeat(self.0 as usize) } fn area(&self) -> u32 { self.0 } }

type Factory = Box<dyn Fn() -> Box<dyn Widget>>;

struct Registry { makers: BTreeMap<&'static str, Factory> }
impl Registry {
    fn new() -> Self {
        let mut makers: BTreeMap<&'static str, Factory> = BTreeMap::new();
        makers.insert("ok", Box::new(|| Box::new(Button { label: "OK".into() })));
        makers.insert("volume", Box::new(|| Box::new(Slider { min: 0, max: 11 })));
        makers.insert("gap", Box::new(|| Box::new(Spacer(3))));
        Registry { makers }
    }
    fn register(&mut self, name: &'static str, f: Factory) { self.makers.insert(name, f); }
    fn build(&self, name: &str) -> Option<Box<dyn Widget>> { self.makers.get(name).map(|f| f()) }
}

fn labels() -> Vec<Box<dyn Fn(u32) -> Box<dyn Display>>> {
    vec![
        Box::new(|n| Box::new(n * 2)),
        Box::new(|n| Box::new(format!("#{}", n))),
        Box::new(|n| Box::new(n as f64 / 4.0)),
    ]
}

fn main() {
    let mut reg = Registry::new();
    let title = String::from("Cancel");
    reg.register("cancel", Box::new(move || Box::new(Button { label: title.clone() })));
    for name in ["ok", "cancel", "volume", "gap", "missing"] {
        match reg.build(name) {
            Some(w) => println!("{:<8} {:<14} area={} {:?}", name, w.draw(), w.area(), w),
            None => println!("{:<8} <none>", name),
        }
    }
    let total: u32 = reg.makers.values().map(|f| f().area()).sum();
    println!("total area {}", total);
    for (i, f) in labels().iter().enumerate() { println!("{} -> {}", i, f(10)); }
}
