//~ ERROR E0308
fn main() {
    let v: Vec<Option<Box<i32>>> = vec![Some(1), None];
    println!("{:?}", v);
}
