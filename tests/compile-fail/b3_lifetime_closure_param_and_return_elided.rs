//~ ERROR lifetime
fn main() {
    let first = |v: &Vec<i32>| -> &i32 { &v[0] };
    let v = vec![1];
    println!("{}", first(&v));
}
