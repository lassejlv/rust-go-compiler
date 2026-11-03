use std::any::{Any, TypeId};
use std::fmt::Debug;

trait Shape: Debug {
    fn area(&self) -> f64;
    fn name(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct Sq(f64);
#[derive(Debug)]
struct Circle { r: f64 }

impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
impl Shape for Circle { fn area(&self) -> f64 { 3.0 * self.r * self.r } }
impl Drop for Sq { fn drop(&mut self) { println!("drop Sq({})", self.0); } }

fn type_name_of<T: Any>(_: &T) -> &'static str { std::any::type_name::<T>() }

fn describe(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() { format!("i32 {}", i) }
    else if let Some(s) = v.downcast_ref::<String>() { format!("String {:?}", s) }
    else if v.is::<Sq>() { "a Sq".to_string() }
    else { "unknown".to_string() }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Sq(2.0)), Box::new(Circle { r: 1.0 })];
    let raws: Vec<*mut dyn Shape> = shapes.into_iter().map(Box::into_raw).collect();
    for &r in &raws {
        unsafe { println!("{} area {}", (*r).name(), (*r).area()); }
    }
    let total: f64 = raws.iter().map(|&r| unsafe { (*r).area() }).sum();
    println!("total {}", total);
    for r in raws {
        let b = unsafe { Box::from_raw(r) };
        println!("reboxed {:?}", b);
    }
    let s = Sq(3.0);
    let dp: *const dyn Shape = &s;
    let thin = dp as *const Sq;
    println!("thin cast {}", unsafe { (*thin).0 });
    println!("size of fat {} thin {}", std::mem::size_of::<*const dyn Shape>(), std::mem::size_of::<*const Sq>());
    println!("size_of_val dyn {}", std::mem::size_of_val::<dyn Shape>(&s));
    let r1: &dyn Shape = &s;
    let r2: &dyn Shape = &s;
    println!("addr_eq {}", std::ptr::addr_eq(r1 as *const dyn Shape, r2 as *const dyn Shape));
    let anys: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("hi")), Box::new(1.5f32)];
    for a in &anys { println!("{}", describe(a.as_ref())); }
    println!("{}", describe(&s));
    let b: Box<dyn Any> = Box::new(77i32);
    match b.downcast::<String>() {
        Ok(_) => println!("string?"),
        Err(orig) => println!("not string, i32 = {:?}", orig.downcast::<i32>().map(|b| *b)),
    }
    println!("{}", TypeId::of::<i32>() == TypeId::of::<i32>());
    println!("{}", TypeId::of::<i32>() == TypeId::of::<u32>());
    println!("{} {}", type_name_of(&1u8), type_name_of(&vec![1i64]));
    let mut boxed: Box<dyn Any> = Box::new(vec![1, 2]);
    if let Some(v) = boxed.downcast_mut::<Vec<i32>>() { v.push(3); }
    println!("{:?}", boxed.downcast_ref::<Vec<i32>>());
    let sendable: Box<dyn Any + Send> = Box::new("static");
    println!("{:?}", sendable.downcast_ref::<&str>());
    println!("end");
}
