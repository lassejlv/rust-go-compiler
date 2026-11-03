//~ ERROR E0277
fn main() {
    let v = vec![Some(1), None];
    let s: i32 = v.into_iter().sum();
    println!("{}", s);
}
