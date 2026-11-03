//~ ERROR E0191
fn main() {
    let v: Vec<i32> = vec![1];
    let it: Box<dyn Iterator> = Box::new(v.into_iter());
    println!("{}", it.count());
}
