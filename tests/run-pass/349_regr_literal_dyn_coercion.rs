// Regression: `{integer}`/`{float}` literals unsize to trait objects (fallback i32/f64).
use std::any::Any;
use std::fmt::{Debug, Display};

fn show_all(items: &[&dyn Display]) -> String {
    items.iter().map(|d| format!("<{}>", d)).collect()
}

fn main() {
    println!("{}", show_all(&[&1, &"two", &3.5]));
    let d: &dyn Display = &7;
    println!("{}", d);
    let b: Box<dyn Any> = Box::new(5);
    println!("i32? {} f64? {}", b.is::<i32>(), b.is::<f64>());
    let f: Box<dyn Any> = Box::new(2.5);
    println!("f64? {}", f.downcast_ref::<f64>().is_some());
    let v: Vec<Box<dyn Display>> = vec![Box::new(1), Box::new(2.5), Box::new("s")];
    for x in &v {
        print!("{} ", x);
    }
    println!();
    let dbg: &dyn Debug = &-3;
    println!("{:?} {}", dbg, std::mem::size_of_val(dbg));
    let big: &dyn Display = &3_000_000_000u64;
    println!("{}", big);
}
