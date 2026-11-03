//~ ERROR E0614
fn main() {
    let v = vec![10, 20];
    for (i, x) in v.iter().enumerate() {
        println!("{} {}", *i, x);
    }
}
