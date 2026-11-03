//~ ERROR E0133
fn main() {
    let v = vec![1, 2];
    let x = v.get_unchecked(0);
    println!("{}", x);
}
