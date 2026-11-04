use std::fmt::Debug;

trait Named {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("hello, {}", self.name())
    }
}

trait Animal: Named + Debug {
    fn legs(&self) -> u32;
    fn describe(&self) -> String {
        format!("{} has {} legs", self.name(), self.legs())
    }
}

trait Pet: Animal {
    fn owner(&self) -> &str;
}

#[derive(Debug)]
struct Dog {
    name: String,
    owner: String,
}

#[derive(Debug)]
struct Bird(u8);

impl Named for Dog {
    fn name(&self) -> String {
        self.name.clone()
    }
}
impl Animal for Dog {
    fn legs(&self) -> u32 {
        4
    }
}
impl Pet for Dog {
    fn owner(&self) -> &str {
        &self.owner
    }
}

impl Named for Bird {
    fn name(&self) -> String {
        format!("bird#{}", self.0)
    }
    fn greet(&self) -> String {
        "tweet".to_string()
    }
}
impl Animal for Bird {
    fn legs(&self) -> u32 {
        2
    }
}
impl Pet for Bird {
    fn owner(&self) -> &str {
        "nobody"
    }
}

fn use_named(n: &dyn Named) -> String {
    n.greet()
}

fn use_animal(a: &dyn Animal) -> String {
    a.describe()
}

// trait with a generic method that is excluded from the vtable
trait Visitor {
    fn visit(&mut self, v: i64);
    fn visit_all<I: IntoIterator<Item = i64>>(&mut self, it: I)
    where
        Self: Sized,
    {
        for x in it {
            self.visit(x);
        }
    }
    fn total(&self) -> i64;
}

struct Sum(i64);
impl Visitor for Sum {
    fn visit(&mut self, v: i64) {
        self.0 += v;
    }
    fn total(&self) -> i64 {
        self.0
    }
}

struct Max(Option<i64>);
impl Visitor for Max {
    fn visit(&mut self, v: i64) {
        self.0 = Some(self.0.map_or(v, |m| m.max(v)));
    }
    fn total(&self) -> i64 {
        self.0.unwrap_or(i64::MIN)
    }
}

fn drive(vs: &mut [Box<dyn Visitor>], data: &[i64]) {
    for v in vs.iter_mut() {
        for &d in data {
            v.visit(d);
        }
    }
}

fn main() {
    let pets: Vec<Box<dyn Pet>> = vec![
        Box::new(Dog { name: "rex".into(), owner: "ann".into() }),
        Box::new(Bird(7)),
    ];
    for p in &pets {
        let a: &dyn Animal = p.as_ref();
        let n: &dyn Named = a;
        println!("{} | {} | {} | {}", p.owner(), use_animal(a), use_named(n), n.name());
        println!("{:?}", a);
    }
    // upcast Box
    let b: Box<dyn Pet> = Box::new(Bird(3));
    let b2: Box<dyn Animal> = b;
    let b3: Box<dyn Named> = b2;
    println!("{}", b3.greet());

    // Rc upcast
    let r: std::rc::Rc<dyn Animal> = std::rc::Rc::new(Dog { name: "fido".into(), owner: "x".into() });
    let r2: std::rc::Rc<dyn Named> = r.clone();
    println!("{} {}", r2.name(), std::rc::Rc::strong_count(&r));

    let mut s = Sum(0);
    s.visit_all(vec![1, 2, 3]);
    s.visit_all(10..13);
    println!("sum={}", s.total());

    let mut vs: Vec<Box<dyn Visitor>> = vec![Box::new(Sum(0)), Box::new(Max(None))];
    drive(&mut vs, &[5, -3, 9, 2]);
    let totals: Vec<i64> = vs.iter().map(|v| v.total()).collect();
    println!("{:?}", totals);
}
