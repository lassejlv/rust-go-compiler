// dyn Any usage: a heterogeneous component store keyed by TypeId, downcast_ref/downcast_mut,
// Box<dyn Any> downcast to owned, Any as a supertrait with an as_any() method on a custom trait,
// type_name, and a plugin registry dispatching on concrete types.
use std::any::{type_name, Any, TypeId};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq)]
struct Position(f32, f32);
#[derive(Debug, Clone, PartialEq)]
struct Health(i32);
#[derive(Debug, Clone, PartialEq)]
struct Name(String);

#[derive(Default)]
struct Store {
    data: HashMap<TypeId, Box<dyn Any>>,
    names: BTreeMap<&'static str, TypeId>,
}

impl Store {
    fn insert<T: Any>(&mut self, v: T) -> Option<T> {
        self.names.insert(type_name::<T>(), TypeId::of::<T>());
        self.data.insert(TypeId::of::<T>(), Box::new(v)).map(|old| *old.downcast::<T>().unwrap())
    }
    fn get<T: Any>(&self) -> Option<&T> {
        self.data.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }
    fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.data.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }
    fn remove<T: Any>(&mut self) -> Option<T> {
        self.data.remove(&TypeId::of::<T>()).and_then(|b| b.downcast().ok()).map(|b| *b)
    }
}

trait Plugin: Any {
    fn name(&self) -> String;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct Counter {
    n: u32,
}
struct Logger {
    lines: Vec<String>,
}
impl Plugin for Counter {
    fn name(&self) -> String {
        format!("counter({})", self.n)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl Plugin for Logger {
    fn name(&self) -> String {
        format!("logger[{}]", self.lines.len())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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
        format!("{:?}", p)
    } else if v.is::<()>() {
        "unit".into()
    } else if let Some(v) = v.downcast_ref::<Vec<u8>>() {
        format!("bytes {:?}", v)
    } else {
        "unknown".into()
    }
}

fn main() {
    let mut s = Store::default();
    s.insert(Position(1.0, 2.5));
    s.insert(Health(100));
    println!("{:?}", s.insert(Health(80)));
    s.insert(Name("hero".into()));
    println!("{:?} {:?} {:?}", s.get::<Position>(), s.get::<Health>(), s.get::<Name>());
    println!("{:?}", s.get::<u64>());
    if let Some(h) = s.get_mut::<Health>() {
        h.0 -= 25;
    }
    println!("{:?}", s.get::<Health>());
    println!("{:?} {:?}", s.remove::<Name>(), s.remove::<Name>());
    println!("{:?}", s.names.keys().map(|k| k.rsplit("::").next().unwrap()).collect::<Vec<_>>());

    let items: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("hi")), Box::new("lit"), Box::new(Position(0.0, -1.0)), Box::new(()), Box::new(vec![1u8, 2]), Box::new(5i64)];
    for it in &items {
        println!("{}", describe(it.as_ref()));
    }
    let ints: i32 = items.iter().filter_map(|b| b.downcast_ref::<i32>()).sum();
    println!("int sum {}", ints);

    let mut plugins: Vec<Box<dyn Plugin>> = vec![Box::new(Counter { n: 0 }), Box::new(Logger { lines: vec![] }), Box::new(Counter { n: 10 })];
    for round in 0..3 {
        for p in plugins.iter_mut() {
            if let Some(c) = p.as_any_mut().downcast_mut::<Counter>() {
                c.n += round + 1;
            } else if let Some(l) = p.as_any_mut().downcast_mut::<Logger>() {
                l.lines.push(format!("round {}", round));
            }
        }
    }
    println!("{:?}", plugins.iter().map(|p| p.name()).collect::<Vec<_>>());
    let counters = plugins.iter().filter(|p| p.as_any().is::<Counter>()).count();
    println!("counters {}", counters);
    if let Some(l) = plugins[1].as_any().downcast_ref::<Logger>() {
        println!("{:?}", l.lines);
    }
    println!("{} {}", TypeId::of::<i32>() == TypeId::of::<i32>(), TypeId::of::<i32>() == TypeId::of::<u32>());
    println!("{} | {} | {}", type_name::<Option<Vec<i32>>>(), type_name::<&str>(), type_name::<(u8, char)>());
    let b: Box<dyn Any> = Box::new(Health(1));
    match b.downcast::<Position>() {
        Ok(p) => println!("pos {:?}", p),
        Err(orig) => println!("not a position; is health: {}", orig.is::<Health>()),
    }
}
