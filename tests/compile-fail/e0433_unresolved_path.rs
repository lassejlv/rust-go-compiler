//~ ERROR E0433
fn main() {
    let m = collections::HashMap::<i32, i32>::new();
    println!("{}", m.len());
}
