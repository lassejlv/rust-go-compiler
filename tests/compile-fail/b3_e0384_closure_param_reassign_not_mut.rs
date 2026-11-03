//~ ERROR E0384
fn main() {
    let v = vec![1, 2];
    let total = 0;
    for x in &v {
        total = total + x;
    }
    println!("{}", total);
}
