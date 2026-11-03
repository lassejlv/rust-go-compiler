//~ ERROR E0505
fn main() {
    let v = vec![1, 2, 3];
    let it = v.iter();
    drop(v);
    println!("{}", it.count());
}
