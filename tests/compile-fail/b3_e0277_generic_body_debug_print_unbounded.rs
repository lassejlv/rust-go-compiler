//~ ERROR E0277
fn dump<T>(items: &[T]) { for i in items { println!("{:?}", i); } }
fn main() { dump(&[1, 2]); }
