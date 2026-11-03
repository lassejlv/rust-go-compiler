//~ ERROR E0596
fn main() {
    let mut total = 0;
    let add = |x: i32| total += x;
    add(1);
    println!("{}", total);
}
