// std::any::Any downcasting: refs, mut refs, Box, type-keyed registry
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, PartialEq)]
struct Position(i32, i32);
#[derive(Debug, PartialEq)]
struct Velocity {
    dx: i32,
    dy: i32,
}
#[derive(Debug)]
struct Name(String);

struct Registry {
    items: HashMap<TypeId, Box<dyn Any>>,
}

impl Registry {
    fn new() -> Self {
        Registry { items: HashMap::new() }
    }
    fn insert<T: Any>(&mut self, v: T) -> Option<T> {
        self.items
            .insert(TypeId::of::<T>(), Box::new(v))
            .map(|old| *old.downcast::<T>().unwrap())
    }
    fn get<T: Any>(&self) -> Option<&T> {
        self.items.get(&TypeId::of::<T>()).and_then(|b| b.downcast_ref::<T>())
    }
    fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.items.get_mut(&TypeId::of::<T>()).and_then(|b| b.downcast_mut::<T>())
    }
    fn remove<T: Any>(&mut self) -> Option<T> {
        self.items.remove(&TypeId::of::<T>()).and_then(|b| b.downcast().ok()).map(|b: Box<T>| *b)
    }
}

fn describe(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() {
        format!("i32 {}", i)
    } else if let Some(s) = v.downcast_ref::<String>() {
        format!("String {:?}", s)
    } else if let Some(s) = v.downcast_ref::<&str>() {
        format!("&str {:?}", s)
    } else if let Some(p) = v.downcast_ref::<Position>() {
        format!("pos {:?}", p)
    } else if v.is::<f64>() {
        format!("f64 {}", v.downcast_ref::<f64>().unwrap())
    } else if let Some(v) = v.downcast_ref::<Vec<u8>>() {
        format!("bytes len {}", v.len())
    } else {
        "unknown".to_string()
    }
}

trait Component: Any + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn tag(&self) -> &'static str;
}
impl Component for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn tag(&self) -> &'static str {
        "pos"
    }
}
impl Component for Velocity {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn tag(&self) -> &'static str {
        "vel"
    }
}

fn main() {
    let vals: Vec<Box<dyn Any>> = vec![
        Box::new(42i32),
        Box::new(String::from("hi")),
        Box::new("static"),
        Box::new(Position(1, 2)),
        Box::new(2.5f64),
        Box::new(vec![1u8, 2, 3]),
        Box::new(7u64),
    ];
    for v in &vals {
        println!("{}", describe(v.as_ref()));
    }
    let mut r = Registry::new();
    println!("{:?}", r.insert(Position(0, 0)));
    println!("{:?}", r.insert(Velocity { dx: 1, dy: -1 }));
    println!("{:?}", r.insert(Name("ent".into())));
    println!("{:?}", r.insert(Position(5, 5)));
    if let Some(p) = r.get_mut::<Position>() {
        p.0 += 10;
    }
    println!("{:?} {:?}", r.get::<Position>(), r.get::<Velocity>());
    println!("{:?}", r.get::<u8>());
    println!("{:?}", r.remove::<Name>().map(|n| n.0));
    println!("{:?}", r.remove::<Name>().is_none());
    println!("count {}", r.items.len());

    let mut comps: Vec<Box<dyn Component>> = vec![Box::new(Position(3, 4)), Box::new(Velocity { dx: 2, dy: 0 })];
    for c in comps.iter_mut() {
        if let Some(p) = c.as_any_mut().downcast_mut::<Position>() {
            p.1 *= 100;
        }
    }
    for c in &comps {
        let is_vel = c.as_any().is::<Velocity>();
        println!("{} {:?} vel={}", c.tag(), c, is_vel);
    }
    let b: Box<dyn Any> = Box::new(Velocity { dx: 9, dy: 9 });
    match b.downcast::<Position>() {
        Ok(p) => println!("pos {:?}", p),
        Err(b) => match b.downcast::<Velocity>() {
            Ok(v) => println!("was vel {:?}", *v),
            Err(_) => println!("?"),
        },
    }
    println!("{}", TypeId::of::<i32>() == TypeId::of::<i32>());
    println!("{}", TypeId::of::<i32>() == TypeId::of::<u32>());
    println!("{}", std::any::type_name::<Option<i32>>().contains("Option"));
}
