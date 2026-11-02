use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn describe(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() {
        format!("i32 {}", i)
    } else if let Some(s) = v.downcast_ref::<String>() {
        format!("String {:?}", s)
    } else if let Some(s) = v.downcast_ref::<&str>() {
        format!("&str {:?}", s)
    } else if let Some(p) = v.downcast_ref::<Point>() {
        format!("Point {:?}", p)
    } else if v.is::<f64>() {
        "some f64".to_string()
    } else {
        "unknown".to_string()
    }
}

fn take_box(b: Box<dyn Any>) -> String {
    match b.downcast::<Point>() {
        Ok(p) => format!("owned point sum {}", p.x + p.y),
        Err(b) => match b.downcast::<Vec<u8>>() {
            Ok(v) => format!("owned vec len {}", v.len()),
            Err(_) => "owned other".to_string(),
        },
    }
}

trait Component: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn label(&self) -> &'static str;
}

struct Health(i32);
struct Position(i32, i32);

impl Component for Health {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn label(&self) -> &'static str {
        "health"
    }
}
impl Component for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn label(&self) -> &'static str {
        "position"
    }
}

fn main() {
    let values: Vec<Box<dyn Any>> = vec![
        Box::new(42i32),
        Box::new(String::from("hi")),
        Box::new("static"),
        Box::new(Point { x: 1, y: 2 }),
        Box::new(2.5f64),
        Box::new(7u8),
    ];
    for v in &values {
        println!("{}", describe(v.as_ref()));
    }
    for v in values {
        println!("{}", take_box(v));
    }
    println!("{}", take_box(Box::new(vec![1u8, 2, 3])));

    println!("{}", TypeId::of::<i32>() == TypeId::of::<i32>());
    println!("{}", TypeId::of::<i32>() == TypeId::of::<u32>());
    let name = std::any::type_name::<Point>();
    println!("{}", name.ends_with("Point"));

    let mut comps: Vec<Box<dyn Component>> = vec![Box::new(Health(10)), Box::new(Position(3, 4))];
    for c in comps.iter_mut() {
        if let Some(h) = c.as_any_mut().downcast_mut::<Health>() {
            h.0 -= 3;
        }
        if let Some(p) = c.as_any_mut().downcast_mut::<Position>() {
            p.0 += 10;
        }
    }
    for c in &comps {
        let any = c.as_any();
        if let Some(h) = any.downcast_ref::<Health>() {
            println!("{} = {}", c.label(), h.0);
        } else if let Some(p) = any.downcast_ref::<Position>() {
            println!("{} = ({}, {})", c.label(), p.0, p.1);
        }
    }

    let mut store: HashMap<TypeId, Box<dyn Any>> = HashMap::new();
    store.insert(TypeId::of::<i32>(), Box::new(5i32));
    store.insert(TypeId::of::<String>(), Box::new("five".to_string()));
    let i = store.get(&TypeId::of::<i32>()).and_then(|b| b.downcast_ref::<i32>());
    let s = store.get(&TypeId::of::<String>()).and_then(|b| b.downcast_ref::<String>());
    let f = store.get(&TypeId::of::<f32>()).and_then(|b| b.downcast_ref::<f32>());
    println!("{:?} {:?} {:?}", i, s, f);
}
